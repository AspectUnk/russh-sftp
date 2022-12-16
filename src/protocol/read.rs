use bytes::Bytes;

use crate::{buf::TryBuf, error};

use super::{impl_request_id, RequestId};

#[derive(Debug)]
pub struct Read {
    pub id: u32,
    pub handle: String,
    pub offset: u64,
    pub len: u32,
}

impl_request_id!(Read);

impl TryFrom<&mut Bytes> for Read {
    type Error = error::Error;

    fn try_from(bytes: &mut Bytes) -> Result<Self, Self::Error> {
        Ok(Self {
            id: bytes.try_get_u32()?,
            handle: bytes.try_get_string()?,
            offset: bytes.try_get_u64()?,
            len: bytes.try_get_u32()?,
        })
    }
}
