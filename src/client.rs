use crate::shared::{create_packet_buffer, StreamBuffer};
use serbytes::prelude::{BBReadResult, SerBytes};
use std::io;
use std::io::Write;
use std::marker::PhantomData;
use std::net::{TcpStream, ToSocketAddrs};

pub struct TcpWrapperClient<O, I> {
    tcp_stream: TcpStream,
    stream_buffer: StreamBuffer,
    _outgoing_packet: PhantomData<O>,
    _incoming_packet: PhantomData<I>,
}

impl<O, I> TcpWrapperClient<O, I>
where
    O: SerBytes,
    I: SerBytes,
{
    pub fn new(socket_addr: impl ToSocketAddrs) -> io::Result<Self> {
        let tcp_stream = TcpStream::connect(socket_addr)?;
        tcp_stream.set_nonblocking(true)?;

        Ok(Self {
            tcp_stream,
            stream_buffer: StreamBuffer::new(),
            _outgoing_packet: PhantomData,
            _incoming_packet: PhantomData,
        })
    }

    pub fn send(&mut self, data: &O) -> io::Result<usize> {
        let wbb = create_packet_buffer(data);

        self.tcp_stream.write(wbb.buf())
    }

    pub fn try_rcv(&mut self) -> BBReadResult<Option<I>> {
        self.stream_buffer
            .append_from_stream(&mut self.tcp_stream)?;

        self.stream_buffer.try_get_data()
    }

    pub fn rcv_blocking(&mut self) -> BBReadResult<Option<I>> {
        self.tcp_stream.set_nonblocking(false)?;
        let rcv_result = self.try_rcv();
        self.tcp_stream.set_nonblocking(true)?;

        rcv_result
    }
}
