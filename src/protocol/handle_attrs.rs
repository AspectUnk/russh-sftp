use bytes::Bytes;

use crate::{buf::TryBuf, error};

use super::{impl_request_id, RequestId, FileAttributes};

/// Implementation for SSH_FXP_FSETSTAT
#[derive(Debug)]
pub struct HandleAttrs {
    pub id: u32,
    pub handle: String,
    pub attrs: FileAttributes,
}

impl_request_id!(HandleAttrs);

impl TryFrom<&mut Bytes> for HandleAttrs {
    type Error = error::Error;

    fn try_from(bytes: &mut Bytes) -> Result<Self, Self::Error> {
        Ok(Self {
            id: bytes.try_get_u32()?,
            handle: bytes.try_get_string()?,
            attrs: FileAttributes::try_from(bytes)?,
        })
    }
}
