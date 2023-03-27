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
    /// Indicates successful completion of the operation.
    #[error("Ok")]
    Ok = 0,
    /// Indicates end-of-file condition; for SSH_FX_READ it means that no more data is available in the file,
    /// and for SSH_FX_READDIR it indicates that no more files are contained in the directory.
    #[error("Eof")]
    Eof = 1,
    /// A reference is made to a file which should exist but doesn't.
    #[error("No such file")]
    NoSuchFile = 2,
    /// Authenticated user does not have sufficient permissions to perform the operation.
    #[error("Permission denied")]
    PermissionDenied = 3,
    /// A generic catch-all error message;
    /// it should be returned if an error occurs for which there is no more specific error code defined.
    #[error("Failure")]
    Failure = 4,
    /// May be returned if a badly formatted packet or protocol incompatibility is detected.
    #[error("Bad message")]
    BadMessage = 5,
    /// A pseudo-error which indicates that the client has no connection to the server
    /// (it can only be generated locally by the client, and MUST NOT be returned by servers).
    #[error("No connection")]
    NoConnection = 6,
    /// A pseudo-error which indicates that the connection to the server has been lost
    /// (it can only be generated locally by the client, and MUST NOT be returned by servers).
    #[error("Connection lost")]
    ConnectionLost = 7,
    /// Indicates that an attempt was made to perform an operation which is not supported for the server
    /// (it may be generated locally by the client if e.g. the version number exchange indicates that a required feature is not supported by the server,
    /// or it may be returned by the server if the server does not implement an operation).
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

/// Implementation for SSH_FXP_STATUS as defined in the specification draft
/// https://filezilla-project.org/specs/draft-ietf-secsh-filexfer-02.txt on page 19 (part 7)
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
