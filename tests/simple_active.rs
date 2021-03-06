#[cfg(test)]
use std::net::SocketAddr;
use std::sync::Arc;
//use std::time::Duration;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::time::{self, Duration};

use zfx_socket_mio::active::IncomingMsg;
use zfx_socket_mio::active::Listener;
use zfx_socket_mio::active::Socket;
use zfx_socket_mio::Error;

const MSGBOX: usize = 1024;

#[derive(Debug, Clone)]
enum TestMessage {
    Send,
    Response { msg: IncomingMsg },
}

struct TestServer {
    send_tx: mpsc::Sender<TestMessage>,
    receive_rx: broadcast::Receiver<TestMessage>,
}

impl TestServer {
    async fn new(addr: SocketAddr) -> TestServer {
        let (send_tx, mut send_rx) = mpsc::channel(MSGBOX); //FIXME: channel size?
        let (receive_tx, receive_rx) = broadcast::channel(MSGBOX); //FIXME: channel size?

        let listener = Listener::bind(addr).await.unwrap();

        tokio::spawn(async move {
            let raw_socket = match listener.accept().await {
                Err(err) => panic!("Listener accept failure due to: {:?}", err),
                Ok(socket) => socket,
            };

            let mut listener_receive_rx = raw_socket.subscribe();

            tokio::spawn(async move {
                // Listener forward loop
                loop {
                    match listener_receive_rx.recv().await {
                        Ok(msg) => {
                            receive_tx.send(TestMessage::Response { msg }).unwrap();
                        }
                        Err(err) => panic!("Socket hung up: {:?}", err),
                    }
                }
            });

            loop {
                match send_rx.recv().await {
                    Some(TestMessage::Send) => (), //raw_socket.try_send(b"server-origin message"),
                    _ => panic!("unexpected msg"),
                }
            }
        });

        TestServer {
            send_tx,
            receive_rx,
        }
    }

    fn send(&self) {
        self.send_tx.try_send(TestMessage::Send).unwrap();
    }
}

/*
#[tokio::test]
async fn failure_to_connect_due_to_no_listener() {
    let target_address = "127.0.0.1:9090";
    let addr = target_address.parse::<SocketAddr>().unwrap();
    //let addr = "127.0.0.1:13265".parse().unwrap();
    let s = Socket::connect(addr);
    //tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    match s {
        Err(Error::IoError(std::io::Error { .. })) => (),
        Err(err) => panic!("Unexpected error: {:?}", err),
        Ok(_socket) => {
            panic!("Socket shouldn't be able to connect");
        }
    }
}
 */

//#[tokio::test]

#[tokio::test]
async fn simple_server_client_success() {
    let target_address = "127.0.0.1:9091";
    let addr = target_address.parse::<SocketAddr>().unwrap();

    let mut server = TestServer::new(addr).await;
    let client = Socket::connect(addr).unwrap();

    //assert_eq!((), client.try_send(b"client-origin message"));

    match server.receive_rx.recv().await {
        Ok(TestMessage::Response { msg: _ }) => (),
        unexpected => panic!("unexpected message: {:?}", unexpected),
    }

    let mut client_receive = client.subscribe();
    server.send();

    match client_receive.recv().await {
        Ok(IncomingMsg::Msg { msg }) => match Arc::try_unwrap(msg) {
            Ok(payload) => assert_eq!(payload.as_slice(), b"server-origin message"),
            Err(err) => panic!("unexpected error: {:?}", err),
        },
        unexpected => panic!("unexpected message from server to client: {:?}", unexpected),
    }
}
