use bytes::Bytes;
use std::fmt;

use crate::{buf::TryBuf, error};

use super::{impl_request_id, RequestId};

pub struct Write {
    pub id: u32,
    pub handle: String,
    pub offset: u64,
    pub data: Vec<u8>,
}

impl_request_id!(Write);

impl fmt::Debug for Write {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Write")
            .field("id", &self.id)
            .field("handle", &self.handle)
            .field("offset", &self.offset)
            .field("data", &self.data.len())
            .finish()
    }
}

impl TryFrom<&mut Bytes> for Write {
    type Error = error::Error;

    fn try_from(bytes: &mut Bytes) -> Result<Self, Self::Error> {
        Ok(Self {
            id: bytes.try_get_u32()?,
            handle: bytes.try_get_string()?,
            offset: bytes.try_get_u64()?,
            data: bytes.try_get_bytes()?,
        })
    }
}
