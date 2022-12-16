use bytes::Bytes;

use crate::{buf::TryBuf, error};

use super::{impl_request_id, RequestId};

#[derive(Debug)]
pub struct Rename {
    pub id: u32,
    pub oldpath: String,
    pub newpath: String,
}

impl_request_id!(Rename);

impl TryFrom<&mut Bytes> for Rename {
    type Error = error::Error;

    fn try_from(bytes: &mut Bytes) -> Result<Self, Self::Error> {
        Ok(Self {
            id: bytes.try_get_u32()?,
            oldpath: bytes.try_get_string()?,
            newpath: bytes.try_get_string()?,
        })
    }
}
