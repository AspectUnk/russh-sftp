use bytes::{BufMut, Bytes, BytesMut};
use thiserror::Error;

use crate::{
    buf::{PutBuf, TryBuf},
    error, protocol,
};

use super::impl_packet_for;

/// Error Codes for SSH_FXP_STATUS
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq, FromPrimitive)]
pub enum StatusCode {
    #[error("Ok")]
    Ok = 0,
    #[error("Eof")]
    Eof = 1,
    #[error("No such file")]
    NoSuchFile = 2,
    #[error("Permission denined")]
    PermissionDenined = 3,
    #[error("Failure")]
    Failure = 4,
    #[error("Bad message")]
    BadMessage = 5,
    #[error("No connection")]
    NoConnection = 6,
    #[error("Connection lost")]
    ConnectionLost = 7,
    #[error("Operation unsupported")]
    OpUnsupported = 8,
}

impl From<u32> for StatusCode {
    fn from(value: u32) -> Self {
        match num::FromPrimitive::from_u32(value) {
            Some(e) => e,
            None => Self::Failure,
        }
    }
}

/// Implementation for SSH_FXP_STATUS
#[derive(Debug, PartialEq, Eq)]
pub struct Status {
    pub id: u32,
    pub status_code: StatusCode,
    pub error_message: String,
    pub language_tag: String,
}

impl_packet_for!(Status, protocol::Response);

impl From<Status> for Bytes {
    fn from(status: Status) -> Self {
        let mut bytes = BytesMut::new();

        bytes.put_u32(status.id);
        bytes.put_u32(status.status_code as u32);
        bytes.put_str(&status.error_message);
        bytes.put_str(&status.language_tag);

        bytes.freeze()
    }
}

impl TryFrom<&mut Bytes> for Status {
    type Error = error::Error;

    fn try_from(bytes: &mut Bytes) -> Result<Self, Self::Error> {
        Ok(Self {
            id: bytes.try_get_u32()?,
            status_code: StatusCode::from(bytes.try_get_u32()?),
            error_message: bytes.try_get_string()?,
            language_tag: bytes.try_get_string()?,
        })
    }
}
