use crate::prelude::*;
pub use crate::{Error, Result};

const MSGBOX: usize = 1024;
const SERVER: Token = Token(0);
const CLIENT: Token = Token(1);

//pub enum ReceivedMsg {}

#[derive(Debug, Clone)]
pub enum IncomingMsg {
    Msg { msg: Arc<Vec<u8>> },
}

pub struct Socket {
    receive_tx: broadcast::Sender<IncomingMsg>,
    done_tx: oneshot::Sender<()>,
}

impl Socket {
    pub fn connect(addr: SocketAddr) -> Result<Socket> {
        // Setup the client socket.
        let s = mio::net::TcpSocket::new_v4()?;
        let client = s.connect(addr)?;
        println!("after connect");
        start_stream(client, CLIENT)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<IncomingMsg> {
        return self.receive_tx.subscribe();
    }

    pub fn try_send(&self, msg: &[u8]) {
        // self.send_tx
        //     .try_send(SocketHandlerMsg::TrySend {
        //         msg: Arc::new(msg.to_vec()),
        //     })
        //     .unwrap();
    }
}

pub struct Listener {
    listener: TcpListener,
}

impl Listener {
    pub async fn bind(address: SocketAddr) -> Result<Listener> {
        let listener = TcpListener::bind(address)?;
        Ok(Listener { listener })
    }

    pub async fn accept(&self) -> Result<Socket> {
        let (server, _) = self.listener.accept()?;
        start_stream(server, SERVER)
    }
}

fn start_stream(mut stream: TcpStream, token: Token) -> Result<Socket> {
    // Create a poll instance.
    let poll = Poll::new()?;
    // Create storage for events.
    let events = Events::with_capacity(128);

    poll.registry()
        .register(&mut stream, token, Interest::READABLE | Interest::WRITABLE)?;

    let (done_tx, done_rx) = oneshot::channel();
    let (receive_tx, _receive_rx) = broadcast::channel(MSGBOX);
    let receive_tx_to_pass = receive_tx.clone();
    let _receive_handler = tokio::spawn(async move {
        // FIXME: do something with the result here
        receive_handler(events, poll, receive_tx_to_pass, done_rx).unwrap();
    });
    Ok(Socket {
        receive_tx,
        done_tx,
    })
}

fn receive_handler(
    mut events: Events,
    mut poll: Poll,
    receive_tx: broadcast::Sender<IncomingMsg>,
    mut done_rx: oneshot::Receiver<()>,
) -> Result<()> {
    loop {
        poll.poll(&mut events, None)?;

        for event in events.iter() {
            match event.token() {
                CLIENT => {
                    let buf = b"stuffos";
                    let payload = buf.to_vec();
                    receive_tx
                        .send(IncomingMsg::Msg {
                            msg: Arc::new(payload),
                        })
                        .unwrap(); // FIXME: handle error?
                    println!("stuff-------");
                    panic!("something received");
                }
                unexpected => {
                    eprintln!("Unexpected token polled: {:?}", unexpected);
                    return Ok(());
                }
            }
        }
    }
}
