use std::io;
use std::net::UdpSocket;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use bincode::{deserialize, serialize};

use consts::*;

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

    fn snd(&mut self, fnres: FnRes, client_addr: SocketAddr) -> Result<(), io::Error> {
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
