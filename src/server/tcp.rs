use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

use bincode::{deserialize, serialize};

use consts::*;
use utils::*;

pub type StreamOutCh = SyncSender<(HandlerOutCh, FnCall)>;
pub type HandlerInCh = Receiver<(HandlerOutCh, FnCall)>;
pub type HandlerOutCh = SyncSender<FnRes>;
pub type StreamInCh = Receiver<FnRes>;

// Spawns threads and such
pub fn init_widow_server(server_ip: Ipv4Addr, port: u16) {
    let (stream_outbox, handler_inbox) = sync_channel(1);

    let mut listener = WidowListener::new(server_ip, port, stream_outbox).unwrap();
    let mut handler = WidowHandler::new(handler_inbox);
    thread::spawn(move || listener.start());
    thread::spawn(move || handler.start());
}

pub struct WidowListener {
    tcp_listener: TcpListener,
    outbox: StreamOutCh,
}

impl WidowListener {
    pub fn new(server_ip: Ipv4Addr, port: u16, outbox: StreamOutCh) -> ResultB<WidowListener> {
        let tcp_conn = SocketAddrV4::new(server_ip, port);
        let tcp_listener = TcpListener::bind(tcp_conn)?;
        info!("Listening on {:?}", tcp_listener);

        Ok(WidowListener {
            tcp_listener,
            outbox,
        })
    }

    pub fn start(&mut self) {
        info!("Listenining on {:?}", self.tcp_listener);
        for _stream in self.tcp_listener.incoming() {
            if let Ok(stream) = _stream {
                info!("New client at {:?}", stream);
                stream.set_nodelay(true).unwrap();
                let peer_addr = stream.peer_addr().unwrap();
                let mut widow_stream = WidowStream::new(stream, self.outbox.clone());
                thread::spawn(move || {
                    // Stream returns on error with the error,
                    // otherwise it should never return
                    if let Err(e) = widow_stream.start() {
                        warn!("Killing stream from {}: {:?}", peer_addr, e);
                    }
                });
            }
        }
    }
}

pub struct WidowHandler {
    inbox: HandlerInCh,
    state: State,
}

impl WidowHandler {
    pub fn new(inbox: HandlerInCh) -> WidowHandler {
        WidowHandler {
            inbox,
            state: State::new(),
        }
    }

    pub fn start(&mut self) {
        loop {
            for (outbox, fncall) in &mut self.inbox.recv() {
                let res = self.state.dispatch(*fncall);
                if let Err(e) = outbox.send(res) {
                    error!("Stream inbox not receiving: {:?}", e);
                }
            }
        }
    }
}

pub struct WidowStream {
    stream: TcpStream,
    stream_inbox: StreamInCh,
    stream_outbox: StreamOutCh,
    handler_outbox: HandlerOutCh,
}

impl WidowStream {
    pub fn new(stream: TcpStream, stream_outbox: StreamOutCh) -> WidowStream {
        let (handler_outbox, stream_inbox) = sync_channel(1);
        WidowStream {
            stream,
            stream_inbox,
            stream_outbox,
            handler_outbox,
        }
    }

    pub fn start(&mut self) -> ResultB<()> {
        loop {
            match self.rcv() {
                Ok(fncall) => {
                    self.stream_outbox
                        .send((self.handler_outbox.clone(), fncall))?;
                    let fnres = self.stream_inbox.recv()?;
                    self.snd(fnres)?;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }

    fn snd(&mut self, fnres: FnRes) -> ResultB<()> {
        let mut buf = [0u8; MSG_BUF_SIZE];
        let encoded: Vec<u8> = serialize(&fnres)?;
        let enc_size_u8s = usize_to_u8_array(encoded.len());
        let buf_len = encoded.len() + 4;

        buf[..4].clone_from_slice(&enc_size_u8s);
        buf[4..buf_len].clone_from_slice(&encoded);
        let _amt = self.stream.write(&buf[..buf_len]);
        Ok(())
    }

    fn rcv(&mut self) -> ResultB<FnCall> {
        let mut n_buf = [0u8; 4];
        let mut buf = [0u8; MSG_BUF_SIZE];

        self.stream.read_exact(&mut n_buf)?;
        let n = u8_array_to_usize(&n_buf[..], 0);
        self.stream.read_exact(&mut buf[..n])?;

        let fncall: FnCall = deserialize(&buf[..n])?;

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
