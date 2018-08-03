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
mod tests {
    use std;
    use client::WidowClient;
    use server::init_widow_server;
    use env_logger;

    fn setup(port: u16) -> WidowClient {
        let localhost = std::net::Ipv4Addr::new(127, 0, 0, 1);
        assert_eq!(localhost.is_loopback(), true);

        init_widow_server(localhost, port);

        WidowClient::connect(localhost, port)
    }

    #[test]
    fn simple_adding() {
        let mut widow_client = setup(8765);

        for i in 1..32 {
            assert_eq!(widow_client.add(1).unwrap(), i);
        }

        widow_client.close();
    }

    #[test]
    fn simple_echo() {
        let mut widow_client = setup(8999);

        for i in 1..32 {
            assert_eq!(widow_client.echo(i).unwrap(), i);
        }

        widow_client.close();
    }

    #[test]
    fn multi_echo() {
        let localhost = std::net::Ipv4Addr::new(127, 0, 0, 1);
        let port = 9001;
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
