use serbytes::prelude::{BBReadResult, ReadByteBufferRefMut, SerBytes, WriteByteBufferOwned};
use std::io::{ErrorKind, Read};
use std::net::TcpStream;
use uuid::Uuid;

pub type ClientID = Uuid;
pub type PacketLen = u32;

pub struct Incoming<I> {
    pub data: I,
    pub client_id: ClientID,
}

pub struct Outgoing<O> {
    pub data: O,
    pub client_id: ClientID,
}

pub(super) struct StreamBuffer {
    buf: Vec<u8>,
}

impl StreamBuffer {
    pub(super) fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub(super) fn append_from_stream(&mut self, stream: &mut TcpStream) -> std::io::Result<()> {
        let mut buf = [0; 1024];

        let written_len = match stream.read(&mut buf) {
            Ok(written_len) => written_len,
            Err(error) => {
                return if error.kind() == ErrorKind::WouldBlock {
                    Ok(())
                } else {
                    Err(error)
                };
            }
        };

        self.buf.extend_from_slice(&buf[0..written_len]);

        Ok(())
    }

    pub(super) fn try_get_data<O>(&mut self) -> BBReadResult<Option<O>>
    where
        O: SerBytes,
    {
        let packet_len_size = PacketLen::size_hint();

        if self.buf.len() < packet_len_size {
            return Ok(None);
        }

        let mut index = 0;
        let mut bit_index = 0;

        let mut rbb_ref =
            ReadByteBufferRefMut::from_bytes(&mut self.buf, &mut index, &mut bit_index);

        let packet_len = PacketLen::from_buf(&mut rbb_ref)? as usize;

        let inner_data = if self.buf.len() - packet_len_size >= packet_len {
            let packet_bytes = self.buf.drain(0..packet_len);

            let data = O::from_bytes(packet_bytes.as_ref())?;

            Some(data)
        } else {
            None
        };

        Ok(inner_data)
    }
}

pub(crate) fn create_packet_buffer<S>(data: &S) -> WriteByteBufferOwned
where
    S: SerBytes,
{
    let mut wbb = WriteByteBufferOwned::new();

    let len_ip = wbb.write_with_index_pointer(&0);

    let buf_len = wbb.len();

    data.to_buf(&mut wbb);

    let content_len = wbb.len() - buf_len;

    wbb.write_at_index_pointer(&len_ip, &(content_len as PacketLen));

    wbb
}
