use std::net::{Ipv4Addr, SocketAddrV4, TcpStream, Shutdown};
use std::io::{Read, Write};
use std::io;

use bincode::{serialize, deserialize};

use consts::*;
use utils::*;

pub struct WidowClient {
    stream: TcpStream,
}

impl WidowClient {
    pub fn connect(server_ip: Ipv4Addr, port: u16) -> WidowClient {
        let addr = SocketAddrV4::new(server_ip, port);

        info!("Connecting to {:?}", addr);
        let stream = TcpStream::connect(addr).unwrap();
        stream.set_nodelay(true).unwrap();

        WidowClient {
            stream,
        }
    }

    pub fn close(&mut self) {
        self.stream.shutdown(Shutdown::Both).unwrap();
    }

    pub fn add(&mut self, x: i32) -> Result<i32, io::Error> {
        match self.call(FnCall::Add(x)) {
            Ok(FnRes::Add(n)) => Ok(n), 
            Err(e) => Err(e),
            _ => panic!("Got some other result")
        }
    }

    pub fn echo(&mut self, x: i32) -> Result<i32, io::Error> {
        match self.call(FnCall::Echo(x)) {
            Ok(FnRes::Echo(n)) => Ok(n), 
            Err(e) => Err(e),
            _ => panic!("Got some other result")
        }
    }

    // Enums are easy. Will switch to something better later
    fn call(&mut self, func: FnCall) -> Result<FnRes, io::Error> {
        self.snd(func);
        self.rcv()
    }

    fn snd(&mut self, fncall: FnCall) {
        info!("Sending {:?}", fncall);
        let mut buf = [0u8; MSG_BUF_SIZE];
        let encoded: Vec<u8> = serialize(&fncall).unwrap();
        let enc_size_u8s = usize_to_u8_array(encoded.len());
        let buf_len = encoded.len() + 4;

        buf[..4].clone_from_slice(&enc_size_u8s);
        buf[4..buf_len].clone_from_slice(&encoded);
        let _amt = self.stream.write(&buf[..buf_len]);
        info!("Sent {:?}", fncall);
    }

    fn rcv(&mut self) -> Result<FnRes, io::Error> {
        let mut n_buf = [0u8; 4];
        let mut buf = [0u8; MSG_BUF_SIZE];

        try!(self.stream.read_exact(&mut n_buf));
        info!("Rcv");
        let n = u8_array_to_usize(&n_buf[..], 0);
        try!(self.stream.read_exact(&mut buf[..n]));

        let fnres: FnRes = deserialize(&buf[..n]).unwrap();
        info!("Rcvd {:?}", fnres);

        Ok(fnres)
    }
}
