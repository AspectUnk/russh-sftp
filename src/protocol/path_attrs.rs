use bytes::Bytes;

use crate::{buf::TryBuf, error};

use super::{impl_request_id, FileAttributes, RequestId};

/// Implementation for SSH_FXP_... SETSTAT and MKDIR
#[derive(Debug)]
pub struct PathAttrs {
    pub id: u32,
    pub path: String,
    pub attrs: FileAttributes,
}

impl_request_id!(PathAttrs);

impl TryFrom<&mut Bytes> for PathAttrs {
    type Error = error::Error;

    fn try_from(bytes: &mut Bytes) -> Result<Self, Self::Error> {
        Ok(Self {
            id: bytes.try_get_u32()?,
            path: bytes.try_get_string()?,
            attrs: FileAttributes::try_from(bytes)?,
        })
    }
}
