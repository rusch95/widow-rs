extern crate bincode;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate serde;
#[macro_use]
extern crate serde_derive;

pub mod client;
pub mod server;
pub mod consts;
pub mod utils;

#[cfg(test)]
mod tcp_tests {
    use std;
    use client::tcp::WidowClient;
    use server::tcp::init_widow_server;

    fn setup(port: u16) -> WidowClient {
        let localhost = std::net::Ipv4Addr::new(127, 0, 0, 1);
        assert_eq!(localhost.is_loopback(), true);

        init_widow_server(localhost, port);

        WidowClient::connect(localhost, port)
    }

    #[test]
    fn simple_adding() {
        let mut widow_client = setup(1000);

        for i in 1..32 {
            assert_eq!(widow_client.add(1).unwrap(), i);
        }

        widow_client.close();
    }

    #[test]
    fn simple_echo() {
        let mut widow_client = setup(2000);

        for i in 1..32 {
            assert_eq!(widow_client.echo(i).unwrap(), i);
        }

        widow_client.close();
    }

    #[test]
    fn multi_echo() {
        let localhost = std::net::Ipv4Addr::new(127, 0, 0, 1);
        let port = 3000;
        init_widow_server(localhost, port);

        let (send, recv) = std::sync::mpsc::sync_channel(8);
        for _ in 0..8 {
            let send_clone = send.clone();
            std::thread::spawn(move||{
                let mut widow_client = WidowClient::connect(localhost, port);
                for i in 1..32 {
                    assert_eq!(widow_client.echo(i).unwrap(), i);
                }
                widow_client.close();
                send_clone.send(()).unwrap();
            });
        }
        for _ in 0..8 {
            recv.recv().unwrap();
        }
    }
}

#[cfg(test)]
mod udp_tests {
    use std;
    use std::sync::mpsc::{SyncSender, Receiver};
    use client::udp::WidowClient;
    use server::udp::WidowSocket;

    fn setup(client_port: u16, server_port: u16) -> WidowClient {
        let localhost = std::net::Ipv4Addr::new(127, 0, 0, 1);
        assert_eq!(localhost.is_loopback(), true);

        let mut widow_server = WidowSocket::new(localhost, server_port);
        std::thread::spawn(move|| widow_server.start());
        WidowClient::bind(localhost, client_port, 
                          localhost, server_port)
    }

    #[test]
    fn simple_udp_adding() {
        let mut widow_client = setup(4000, 4001);

        for i in 1..32 {
            assert_eq!(widow_client.add(1).unwrap(), i);
        }
    }

    #[test]
    fn simple_udp_echo() {
        let mut widow_client = setup(5000, 5001);

        for i in 1..32 {
            assert_eq!(widow_client.echo(i).unwrap(), i)
        }
    }

    #[test]
    fn aggresive_udp_add() {
        let localhost = std::net::Ipv4Addr::new(127, 0, 0, 1);
        assert_eq!(localhost.is_loopback(), true);
        let server_port = 6000;

        let mut widow_server = WidowSocket::new(localhost, server_port);
        std::thread::spawn(move|| widow_server.start());
        let num_threads = 32;
        let num_pings_per_thread = 512;
        let (send, recv): (SyncSender<i32>, Receiver<i32>) 
            = std::sync::mpsc::sync_channel(num_threads);
        for i in 0..num_threads {
            let client_port = server_port + (i as u16) + 1;
            let mut widow_client = WidowClient::bind(localhost, client_port, 
                                                     localhost, server_port);
            let mut send_clone = send.clone();
            std::thread::spawn(move||{
                for _ in 0..num_pings_per_thread {
                    send_clone.send(widow_client.add(1).unwrap()).unwrap();
                }
            });
        }
        for n in recv.iter() {
            if (n as usize) > num_threads * num_pings_per_thread / 10 * 9 {
                break;
            };
        }
    }
}
