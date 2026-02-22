use crate::shared::{create_packet_buffer, ClientID, Incoming, Outgoing, StreamBuffer};
use rand::random;
use serbytes::prelude::SerBytes;
use std::collections::HashMap;
use std::io::{ErrorKind, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, RecvError, SendError, Sender, TryRecvError};
use std::thread::JoinHandle;
use std::time::Duration;
use std::{io, thread};
use uuid::Uuid;

pub struct TcpWrapperServer<O, I> {
    incoming_rx: Receiver<Incoming<I>>,
    outgoing_tx: Sender<Outgoing<O>>,
    thread_handle: JoinHandle<()>,
}

impl<O, I> TcpWrapperServer<O, I>
where
    O: 'static + Send + SerBytes,
    I: 'static + Send + SerBytes,
{
    pub fn new(port: u16) -> Result<Self, ()> {
        let (outgoing_tx, outgoing_rx) = mpsc::channel();
        let (incoming_tx, incoming_rx) = mpsc::channel();

        let thread_handle = thread::Builder::new()
            .name("Server Listener".to_string())
            .spawn(move || if let Err(e) = server_thread(port, outgoing_rx, incoming_tx) {})
            .expect("Error creating server thread at os level");

        Ok(Self {
            incoming_rx,
            outgoing_tx,
            thread_handle,
        })
    }

    pub fn try_rcv(&self) -> Result<Incoming<I>, TryRecvError> {
        self.incoming_rx.try_recv()
    }

    pub fn rcv_blocking(&self) -> Result<Incoming<I>, RecvError> {
        self.incoming_rx.recv()
    }

    pub fn send(&self, data: O, client_id: ClientID) -> Result<(), SendError<Outgoing<O>>> {
        self.outgoing_tx.send(Outgoing { data, client_id })
    }
}

struct StreamData {
    stream: TcpStream,
    socket_addr: SocketAddr,
    stream_buffer: StreamBuffer,
}

fn server_thread<O, I>(
    port: u16,
    rx_outgoing: Receiver<Outgoing<O>>,
    tx_incoming: Sender<Incoming<I>>,
) -> io::Result<()>
where
    O: SerBytes,
    I: SerBytes,
{
    let tcp_listener = TcpListener::bind(("127.0.0.1", port))?;

    tcp_listener.set_nonblocking(true)?;

    let mut streams = HashMap::new();

    loop {
        let accepted = match tcp_listener.accept() {
            Ok((stream, socket_addr)) => Some(StreamData {
                stream,
                socket_addr,
                stream_buffer: StreamBuffer::new(),
            }),

            Err(err) => {
                if err.kind() != ErrorKind::WouldBlock {
                    eprintln!("Error accepting stream, {}", err);
                }

                None
            }
        };

        if let Some(accepted) = accepted {
            let client_id = Uuid::from_bytes(random());

            streams.insert(client_id, accepted);
        }

        for outgoing in rx_outgoing.try_iter() {
            println!("out to: {}", outgoing.client_id);
            let StreamData { stream, .. } =
                if let Some(stream) = streams.get_mut(&outgoing.client_id) {
                    stream
                } else {
                    continue;
                };
            println!("bouta send");

            let wbb = create_packet_buffer(&outgoing.data);

            stream
                .write_all(wbb.buf())
                .expect("Unable to write buffer to stream");
        }

        for (client_id, stream_data) in streams.iter_mut() {
            let StreamData {
                stream,
                stream_buffer,
                ..
            } = stream_data;

            stream_buffer
                .append_from_stream(stream)
                .expect("Append data to buf");

            if let Some(data) = stream_buffer.try_get_data().expect("Get buffer data") {
                let incoming = Incoming {
                    data,
                    client_id: *client_id,
                };

                tx_incoming
                    .send(incoming)
                    .expect("Error sending message, other end hung up");
            }
        }

        thread::sleep(Duration::from_millis(100));
    }
}
