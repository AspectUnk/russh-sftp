use bytes::{BufMut, Bytes, BytesMut};

use crate::{
    buf::{PutBuf, TryBuf},
    error, protocol,
};

use super::{impl_packet_for, impl_request_id, RequestId};

#[derive(Debug)]
pub struct Handle {
    pub id: u32,
    pub handle: String,
}

impl_request_id!(Handle);
impl_packet_for!(Handle, protocol::Response);

impl From<Handle> for Bytes {
    fn from(handle: Handle) -> Self {
        let mut bytes = BytesMut::new();
        bytes.put_u32(handle.id);
        bytes.put_str(&handle.handle);
        bytes.freeze()
    }
}

impl TryFrom<&mut Bytes> for Handle {
    type Error = error::Error;

    fn try_from(bytes: &mut Bytes) -> Result<Self, Self::Error> {
        Ok(Self {
            id: bytes.try_get_u32()?,
            handle: bytes.try_get_string()?,
        })
    }
}
