use bytes::{BufMut, Bytes, BytesMut};

use crate::{buf::PutBuf, protocol};

use super::impl_packet_for;

#[derive(Debug)]
pub struct Handle {
    pub id: u32,
    pub handle: String,
}

impl_packet_for!(Handle, protocol::Response);

impl From<Handle> for Bytes {
    fn from(handle: Handle) -> Self {
        let mut bytes = BytesMut::new();
        bytes.put_u32(handle.id);
        bytes.put_str(&handle.handle);
        bytes.freeze()
    }
}
