use bytes::Bytes;

use crate::{buf::TryBuf, error};

use super::{impl_request_id, RequestId};

/// Implementation for SSH_FXP_SYMLINK
#[derive(Debug)]
pub struct Symlink {
    pub id: u32,
    pub linkpath: String,
    pub targetpath: String,
}

impl_request_id!(Symlink);

impl TryFrom<&mut Bytes> for Symlink {
    type Error = error::Error;

    fn try_from(bytes: &mut Bytes) -> Result<Self, Self::Error> {
        Ok(Self {
            id: bytes.try_get_u32()?,
            linkpath: bytes.try_get_string()?,
            targetpath: bytes.try_get_string()?,
        })
    }
}
