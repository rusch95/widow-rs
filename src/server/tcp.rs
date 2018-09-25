use std::io;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::thread;

use bincode::{deserialize, serialize};
use tokio;
use tokio::net::{TcpStream, TcpListener};
use tokio::prelude::*;

use consts::*;
use utils::*;

pub type ListenerOutCh = Sender<(u32, FnCall)>;
pub type HandlerInCh = Receiver<(u32, FnCall)>;
pub type HandlerOutCh = Sender<(u32, FnRes)>;
pub type ListenerInCh = Receiver<(u32, Fnres)>;

// Spawns threads and such
pub fn init_widow_server(server_ip: Ipv4Addr, port: u16) {
    let (listener_outbox, handler_inbox) = channel();
    let (handler_outbox, listener_inbox) = channel();

    thread::spawn(move || init_widow_listener(server_ip, port, listener_outbox, listener_inbox));
    thread::spawn(move || init_widow_handler(handler_outbox, handler_inbox));
}

/* Start up a tokio::TcpListener. Does not return */
pub fn init_widow_listener(server_ip: Ipv4Addr, port: u16, outbox: ListenerOutCh, inbox: ListenerInCh) {
    let tcp_conn = SocketAddrV4::new(server_ip, port);
    let listener = TcpListener::bind(tcp_conn).unwrap();
    info!("Listening on {:?}", tcp_conn);

    tokio::run({
        listener.incoming()
            .map_err(|e| error!("Failed to accept socket; error = {:?}", e))
            .for_each(|socket| {
                WidowStream::new(socket).start();
                Ok(())
        });
    });
}

pub fn init_widow_handler(outbox: HandlerOutCh, inbox: HandlerInCh) {
    let state = State::new();

    // Service messages from streams
    for (stream_id, fncall) in &mut inbox.recv() {
        if let Ok(fncall) = inbox.try_recv() {
            let res = .state.dispatch(fncall);
            outbox.send(res).unwrap();
        }
    }
}

pub struct WidowStream {
    socket: TcpStream,
    inbox: StreamInCh,
    outbox: StreamOutCh,
}

impl WidowStream {
    pub fn new(socket: TcpStream, inbox: StreamInCh, outbox: StreamOutCh) -> WidowStream {
        WidowStream {
            socket,
            inbox,
            outbox,
        }
    }

    pub fn start(&mut self) {
        loop {
            match self.rcv() {
                Ok(fncall) => {
                    self.outbox.send(fncall).unwrap();
                    let fnres = self.inbox.recv().unwrap();
                    self.snd(fnres);
                }
                Err(e) => {
                    warn!("Killing stream because {}", e);
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
