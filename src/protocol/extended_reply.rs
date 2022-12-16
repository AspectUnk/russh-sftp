use bytes::{BufMut, Bytes, BytesMut};

use crate::protocol;

use super::impl_packet_for;

#[derive(Debug)]
pub struct ExtendedReply {
    pub id: u32,
    pub data: Vec<u8>,
}

impl_packet_for!(ExtendedReply, protocol::Response);

impl From<ExtendedReply> for Bytes {
    fn from(reply: ExtendedReply) -> Self {
        let mut bytes = BytesMut::new();
        bytes.put_u32(reply.id);
        bytes.put_slice(&reply.data);
        bytes.freeze()
    }
}
