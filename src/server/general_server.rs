use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener, TcpStream};
use std::net::UdpSocket;
use std::io::{Read, Write};
use std::io;
use std::sync::mpsc::{channel};
use std::thread;

use bincode::{serialize, deserialize};

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

    let mut listener = WidowListener::new(server_ip, port, new_stream_outbox);
    let mut server = WidowServer::new(new_stream_inbox);
    thread::spawn(move|| listener.start());
    thread::spawn(move|| server.start());
}

pub struct WidowListener {
    tcp_listener: TcpListener,
    outbox: NewStreamOutCh,
}

impl WidowListener {
    pub fn new(server_ip: Ipv4Addr, port: u16, outbox: NewStreamOutCh) -> WidowListener {
        let tcp_conn = SocketAddrV4::new(server_ip, port);
        let tcp_listener = TcpListener::bind(tcp_conn).unwrap();
        info!("Listening on {:?}", tcp_listener);

        WidowListener {
            tcp_listener,
            outbox,
        }
    }

    pub fn start(&mut self) {
        info!("Listenining on {:?}", self.tcp_listener);
        for _stream in self.tcp_listener.incoming() {
            if let Ok(stream) = _stream {
                info!("New client at {:?}", stream);
                stream.set_nodelay(true).unwrap();
                
                let (stream_outbox, server_inbox) = channel();
                let (server_outbox, stream_inbox) = channel();
                let mut widow_stream = WidowStream::new(stream, stream_inbox, stream_outbox);
                thread::spawn(move|| widow_stream.start());
                self.outbox.send((server_outbox, server_inbox)).unwrap();
            }
        }
    }
}

pub struct State {
    n: i32,
}

impl State {
    pub fn new() -> State {
        State {
            n: 0, 
        }
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

pub struct WidowServer {
    new_stream_inbox: NewStreamInCh,
    streams: Vec<NewStreamCh>,
    state: State,
}

impl WidowServer {
    pub fn new(new_stream_inbox: NewStreamInCh) -> WidowServer {
        WidowServer {
            new_stream_inbox,
            streams: Vec::new(),
            state: State::new(),
        }
    }

    pub fn start(&mut self) {
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
    pub fn new(stream: TcpStream, inbox: StreamInCh, outbox: StreamOutCh) -> WidowStream {
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

pub struct WidowSocket {
    socket: UdpSocket,
    state: State,
}

impl WidowSocket {
    pub fn new(server_ip: Ipv4Addr, server_port: u16) -> WidowSocket {
        let server_addr = SocketAddrV4::new(server_ip, server_port);
        info!("Connecting to {:?}", server_addr);
        let socket = UdpSocket::bind(server_addr).unwrap();

        WidowSocket {
            socket,
            state: State::new(),
        }
    }
    
    pub fn start(&mut self) {
        loop {
            let (fncall, src) = self.rcv().unwrap();
            let fnres = self.state.dispatch(fncall);
            self.snd(fnres, src).unwrap();
        }
    }

    fn snd(&mut self, fnres: FnRes, client_addr: SocketAddr) -> Result<(), io::Error>{
        info!("Sending fnres");
        let encoded: Vec<u8> = serialize(&fnres).unwrap();
        try!(self.socket.send_to(&encoded, client_addr));
        info!("Sent fncall");
        Ok(())
    }

    fn rcv(&mut self) -> Result<(FnCall, SocketAddr), io::Error> {
        let mut buf = [0u8; MSG_BUF_SIZE];

        let (amt, src) = try!(self.socket.recv_from(&mut buf));
        let fncall: FnCall = deserialize(&buf[..amt]).unwrap();
        info!("Received fncall from {:?}", src);

        Ok((fncall, src))
    }
}
