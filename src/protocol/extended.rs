use bytes::{Buf, Bytes};

use crate::{buf::TryBuf, error};

use super::{impl_request_id, RequestId};

#[derive(Debug)]
pub struct Extended {
    pub id: u32,
    pub request: String,
    pub data: Vec<u8>,
}

impl_request_id!(Extended);

impl TryFrom<&mut Bytes> for Extended {
    type Error = error::Error;

    fn try_from(bytes: &mut Bytes) -> Result<Self, Self::Error> {
        Ok(Self {
            id: bytes.try_get_u32()?,
            request: bytes.try_get_string()?,
            data: bytes.copy_to_bytes(bytes.remaining()).to_vec(),
        })
    }
}
