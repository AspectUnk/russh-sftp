use bytes::{BufMut, Bytes, BytesMut};
use std::fmt;

use crate::protocol;

use super::impl_packet_for;

pub struct Data {
    pub id: u32,
    pub data: Vec<u8>,
}

impl_packet_for!(Data, protocol::Response);

impl From<Data> for Bytes {
    fn from(data: Data) -> Self {
        let mut bytes = BytesMut::new();
        bytes.put_u32(data.id);
        bytes.put_u32(data.data.len() as u32);
        bytes.put_slice(&data.data);
        bytes.freeze()
    }
}

impl fmt::Debug for Data {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Data")
            .field("id", &self.id)
            .field("data", &self.data.len())
            .finish()
    }
}
