use std::io;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::thread;

use bincode::{deserialize, serialize};
use tokio::net::{TcpStream, TcpListener};
use tokio::prelude::*;

use consts::*;
use utils::*;

pub type StreamOutCh = Sender<FnCall>;
pub type StreamInCh = Receiver<FnRes>;
pub type ServerOutCh = Sender<FnRes>;
pub type ServerInCh = Receiver<FnCall>;
pub type NewStreamCh = (ServerOutCh, ServerInCh);
pub type NewStreamOutCh = Sender<NewStreamCh>;
pub type NewStreamInCh = Receiver<NewStreamCh>;

// Spawns threads and such
pub fn init_widow_server(server_ip: Ipv4Addr, port: u16) {
    let (new_stream_outbox, new_stream_inbox) = channel();

    thread::spawn(move || init_widow_listener(server_ip, port, new_stream_outbox);
    thread::spawn(move || server.start(new_stream_inbox));
}

/* Start up a tokio::TcpListener. Does not return */
pub fn init_widow_listener(server_ip: Ipv4Addr, port: u16, outbox: NewStreamOutCh) {
    let tcp_conn = SocketAddrV4::new(server_ip, port);
    let listener = TcpListener::bind(tcp_conn).unwrap();
    info!("Listening on {:?}", tcp_conn);

    tokio::run({
        listener.incoming()
            .map_err(|e| error!("Failed to accept socket; error = {:?}", e))
            .for_each(|socket| {
                WidowStream(socket).start();
                Ok(())
            }
    });
}

init_widow_server(new_stream_inbox: NewStreamInCh) -> WidowServer {
        let streams = Vec::new(),
        let state = State::new(),

        loop {
            // Look for new streams
            if let Ok(new_stream) = self.new_stream_inbox.try_recv() {
                self.streams.push(new_stream);
            }

            // Service messages from streams
            for (outbox, inbox) in &mut self.streams {
                if let Ok(fncall) = inbox.try_recv() {
                    let res = self.state.dispatch(fncall);
                    outbox.send(res).unwrap();
                }
            }
        }
    }
}

pub struct WidowStream {
    stream: TcpStream,
    inbox: StreamInCh,
    outbox: StreamOutCh,
}

impl WidowStream {
    pub fn new(socket: TcpStream, inbox: StreamInCh, outbox: StreamOutCh) -> WidowStream {
        WidowStream {
            stream,
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
        let _amt = self.stream.write(&buf[..buf_len]);
    }

    fn rcv(&mut self) -> Result<FnCall, io::Error> {
        let mut n_buf = [0u8; 4];
        let mut buf = [0u8; MSG_BUF_SIZE];

        try!(self.stream.read_exact(&mut n_buf));
        let n = u8_array_to_usize(&n_buf[..], 0);
        try!(self.stream.read_exact(&mut buf[..n]));

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
