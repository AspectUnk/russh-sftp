use bytes::Bytes;

use crate::{buf::TryBuf, error};

use super::{impl_request_id, RequestId};

/// Implementation for SSH_FXP_REMOVE
#[derive(Debug)]
pub struct Remove {
    pub id: u32,
    pub filename: String,
}

impl_request_id!(Remove);

impl TryFrom<&mut Bytes> for Remove {
    type Error = error::Error;

    fn try_from(bytes: &mut Bytes) -> Result<Self, Self::Error> {
        Ok(Self {
            id: bytes.try_get_u32()?,
            filename: bytes.try_get_string()?,
        })
    }
}
