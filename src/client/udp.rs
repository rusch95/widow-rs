use std::net::UdpSocket;
use std::net::{Ipv4Addr, SocketAddrV4};

use bincode::{deserialize, serialize};

use consts::*;

pub struct WidowClient {
    socket: UdpSocket,
    server_addr: SocketAddrV4,
}

impl WidowClient {
    pub fn bind(
        client_ip: Ipv4Addr,
        client_port: u16,
        server_ip: Ipv4Addr,
        server_port: u16,
    ) -> ResultB<WidowClient> {
        let client_addr = SocketAddrV4::new(client_ip, client_port);
        let server_addr = SocketAddrV4::new(server_ip, server_port);

        info!("Binding to {:?}", client_addr);
        info!("Connecting to {:?}", client_addr);
        let socket = UdpSocket::bind(client_addr)?;

        Ok(WidowClient {
            socket,
            server_addr,
        })
    }

    pub fn add(&mut self, x: i32) -> ResultB<i32> {
        match self.call(FnCall::Add(x)) {
            Ok(FnRes::Add(n)) => Ok(n),
            Err(e) => Err(e),
            _ => panic!("Result is not Add"),
        }
    }

    pub fn echo(&mut self, x: i32) -> ResultB<i32> {
        match self.call(FnCall::Echo(x)) {
            Ok(FnRes::Echo(n)) => Ok(n),
            Err(e) => Err(e),
            _ => panic!("Result is not Echo"),
        }
    }

    // Enums are easy. Will switch to something better later
    fn call(&mut self, func: FnCall) -> ResultB<FnRes> {
        self.snd(func)?;
        self.rcv()
    }

    fn snd(&mut self, fncall: FnCall) -> ResultB<()> {
        debug!("Sending fncall");
        let encoded: Vec<u8> = serialize(&fncall)?;
        self.socket.send_to(&encoded, self.server_addr)?;
        debug!("Sent fncall");

        Ok(())
    }

    fn rcv(&mut self) -> ResultB<FnRes> {
        let mut buf = [0u8; MSG_BUF_SIZE];

        let (amt, src) = try!(self.socket.recv_from(&mut buf));
        let fnres: FnRes = deserialize(&buf[..amt])?;
        debug!("Received fnres from {:?}", src);

        Ok(fnres)
    }
}
