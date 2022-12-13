use bytes::Bytes;

use crate::{buf::TryBuf, error};

#[derive(Debug, PartialEq, Eq)]
pub struct Path {
    pub id: u32,
    pub path: String,
}

impl TryFrom<&mut Bytes> for Path {
    type Error = error::Error;

    fn try_from(bytes: &mut Bytes) -> Result<Self, Self::Error> {
        Ok(Self {
            id: bytes.try_get_u32()?,
            path: bytes.try_get_string()?,
        })
    }
}
