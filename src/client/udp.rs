use std::io;
use std::net::UdpSocket;
use std::net::{IpAddr, SocketAddr};

use bincode::{deserialize, serialize};

use consts::*;

pub struct WidowClient {
    socket: UdpSocket,
    server_addr: SocketAddr,
}

impl WidowClient {
    pub fn bind(
        client_ip: IpAddr,
        client_port: u16,
        server_ip: IpAddr,
        server_port: u16,
    ) -> WidowClient {
        let client_addr = SocketAddr::new(client_ip, client_port);
        let server_addr = SocketAddr::new(server_ip, server_port);

        info!("Binding to {:?}", client_addr);
        info!("Connecting to {:?}", client_addr);
        let socket = UdpSocket::bind(client_addr).unwrap();

        WidowClient {
            socket,
            server_addr,
        }
    }

    pub fn add(&mut self, x: i32) -> Result<i32, io::Error> {
        match self.call(FnCall::Add(x)) {
            Ok(FnRes::Add(n)) => Ok(n),
            Err(e) => Err(e),
            _ => panic!("Got some other result"),
        }
    }

    pub fn echo(&mut self, x: i32) -> Result<i32, io::Error> {
        match self.call(FnCall::Echo(x)) {
            Ok(FnRes::Echo(n)) => Ok(n),
            Err(e) => Err(e),
            _ => panic!("Got some other result"),
        }
    }

    // Enums are easy. Will switch to something better later
    fn call(&mut self, func: FnCall) -> Result<FnRes, io::Error> {
        self.snd(func).unwrap();
        self.rcv()
    }

    fn snd(&mut self, fncall: FnCall) -> Result<(), io::Error> {
        info!("Sending fncall");
        let encoded: Vec<u8> = serialize(&fncall).unwrap();
        try!(self.socket.send_to(&encoded, self.server_addr));
        info!("Sent fncall");

        Ok(())
    }

    fn rcv(&mut self) -> Result<FnRes, io::Error> {
        let mut buf = [0u8; MSG_BUF_SIZE];

        let (amt, src) = try!(self.socket.recv_from(&mut buf));
        let fnres: FnRes = deserialize(&buf[..amt]).unwrap();
        info!("Received fnres from {:?}", src);

        Ok(fnres)
    }
}
