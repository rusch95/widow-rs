use std::io;
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr};
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::thread;

use bincode::{deserialize, serialize};
use tokio;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;

use consts::*;
use utils::*;

pub type StreamID = u32;
pub type ListenerOutCh = Sender<(HandlerOutCh, FnCall)>;
pub type HandlerInCh = Receiver<(HandlerOutCh, FnCall)>;
pub type HandlerOutCh = Sender<(FnRes)>;
pub type ListenerInCh = Receiver<(FnRes)>;

// Spawns threads and such
pub fn init_widow_server(server_ip: IpAddr, port: u16) {
    let (listener_outbox, handler_inbox) = channel();

    thread::spawn(move || init_widow_listener(server_ip, port, listener_outbox));
    thread::spawn(move || init_widow_handler(handler_inbox));
}

/* Start up a tokio::TcpListener. Does not return */
pub fn init_widow_listener(server_ip: IpAddr, port: u16, outbox: ListenerOutCh) {
    let tcp_conn = SocketAddr::new(server_ip, port);
    let listener = TcpListener::bind(&tcp_conn).unwrap();
    info!("Listening on {:?}", tcp_conn);

    let done = listener
        .incoming()
        .map_err(|e| error!("Failed to accept socket; error = {:?}", e))
        .for_each(move|socket| {
            WidowStream::new(socket, outbox.clone()).start();
            Ok(())
        });

    tokio::run(done);
}

pub fn init_widow_handler(inbox: HandlerInCh) {
    let mut state = State::new();

    // Service messages from streams
    for (outbox, fncall) in &mut inbox.iter() {
        let res = state.dispatch(fncall);
        outbox.send(res).unwrap();
    }
}

pub struct State {
    n: i32,
}

impl State {
    pub fn new() -> State {
        State { n: 0 }
    }

    pub fn dispatch(&mut self, fncall: FnCall) -> FnRes {
        match fncall {
            FnCall::Add(x) => FnRes::Add(self.add(x)),
            FnCall::Echo(x) => FnRes::Echo(self.echo(x)),
        }
    }

    fn add(&mut self, x: i32) -> i32 {
        self.n += x;
        self.n
    }

    fn echo(&self, x: i32) -> i32 {
        x
    }
}

pub struct WidowStream {
    socket: TcpStream,
    listener_outbox: ListenerOutCh,
    handler_outbox: HandlerOutCh,
    listener_inbox: ListenerInCh,
}

impl WidowStream {
    pub fn new(
        socket: TcpStream,
        listener_outbox: ListenerOutCh,
    ) -> WidowStream {
        let (handler_outbox, listener_inbox) = channel();
        WidowStream {
            socket,
            listener_outbox,
            handler_outbox,
            listener_inbox,
        }
    }

    pub fn start(&mut self) {
        loop {
            match self.rcv() {
                Ok(fncall) => {
                    self.listener_outbox
                        .send((self.handler_outbox.clone(), fncall))
                        .unwrap();
                    let fnres = self.listener_inbox.recv().unwrap();
                    self.snd(fnres);
                }
                Err(e) => {
                    warn!("Killing stream because {:?}", e);
                }
            }
        }
    }

    fn snd(&mut self, fnres: FnRes) {
        let mut buf = [0u8; MSG_BUF_SIZE];
        let encoded: Vec<u8> = serialize(&fnres).unwrap();
        let enc_size_u8s = usize_to_u8_array(encoded.len());
        let buf_len = encoded.len() + 4;

        buf[..4].clone_from_slice(&enc_size_u8s);
        buf[4..buf_len].clone_from_slice(&encoded);
        let _amt = self.socket.write(&buf[..buf_len]);
    }

    fn rcv(&mut self) -> Result<FnCall, io::Error> {
        let mut n_buf = [0u8; 4];
        let mut buf = [0u8; MSG_BUF_SIZE];


        try!(self.socket.read_exact(&mut n_buf));
        let n = u8_array_to_usize(&n_buf[..], 0);
        try!(self.socket.read_exact(&mut buf[..n]));

        let fncall: FnCall = deserialize(&buf[..n]).unwrap();

        Ok(fncall)
    }
}
