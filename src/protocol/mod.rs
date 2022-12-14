mod init;
mod name;
mod path;
mod status;

use bytes::{BufMut, Bytes, BytesMut};
use thiserror::Error;

use crate::{buf::TryBuf, error};

pub use self::{
    init::{Init, Version},
    name::{File, Name},
    path::Path,
    status::Status,
};

macro_rules! impl_packet_for {
    ($name:ident, $packet:ty) => {
        impl From<$name> for $packet {
            fn from(input: $name) -> Self {
                Self::$name(input)
            }
        }
    };
}

pub(crate) use impl_packet_for;

pub const VERSION: u32 = 3;

pub const SSH_FXP_INIT: u8 = 1;
pub const SSH_FXP_VERSION: u8 = 2;
pub const SSH_FXP_OPEN: u8 = 3;
pub const SSH_FXP_CLOSE: u8 = 4;
pub const SSH_FXP_READ: u8 = 5;
pub const SSH_FXP_WRITE: u8 = 6;
pub const SSH_FXP_LSTAT: u8 = 7;
pub const SSH_FXP_FSTAT: u8 = 8;
pub const SSH_FXP_SETSTAT: u8 = 9;
pub const SSH_FXP_FSETSTAT: u8 = 10;
pub const SSH_FXP_OPENDIR: u8 = 11;
pub const SSH_FXP_READDIR: u8 = 12;
pub const SSH_FXP_REMOVE: u8 = 13;
pub const SSH_FXP_MKDIR: u8 = 14;
pub const SSH_FXP_RMDIR: u8 = 15;
pub const SSH_FXP_REALPATH: u8 = 16;
pub const SSH_FXP_STAT: u8 = 17;
pub const SSH_FXP_RENAME: u8 = 18;
pub const SSH_FXP_READLINK: u8 = 19;
pub const SSH_FXP_SYMLINK: u8 = 20;

pub const SSH_FXP_STATUS: u8 = 101;
pub const SSH_FXP_HANDLE: u8 = 102;
pub const SSH_FXP_DATA: u8 = 103;
pub const SSH_FXP_NAME: u8 = 104;
pub const SSH_FXP_ATTRS: u8 = 105;

pub const SSH_FXP_EXTENDED: u8 = 200;
pub const SSH_FXP_EXTENDED_REPLY: u8 = 201;

#[derive(Debug)]
pub(crate) enum Request {
    Init(Init),
    RealPath(Path),
}

impl TryFrom<&mut Bytes> for Request {
    type Error = error::Error;

    fn try_from(bytes: &mut Bytes) -> Result<Self, Self::Error> {
        let r#type = bytes.try_get_u8()?;
        debug!("packet type {}", r#type);

        let packet = match r#type {
            SSH_FXP_INIT => Self::Init(Init::try_from(bytes)?),
            SSH_FXP_REALPATH => Self::RealPath(Path::try_from(bytes)?),
            _ => return Err(StatusCode::OpUnsupported.into()),
        };

        Ok(packet)
    }
}

#[derive(Debug)]
pub(crate) enum Response {
    Version(Version),
    Status(Status),
    Name(Name),
}

impl From<error::Error> for Response {
    fn from(err: error::Error) -> Self {
        let status = match err {
            error::Error::Protocol(p) => p,
            _ => StatusCode::OpUnsupported,
        };

        Self::Status(Status::new(0, status, &status.to_string()))
    }
}

impl From<Response> for Bytes {
    fn from(response: Response) -> Self {
        let (r#type, payload): (u8, Bytes) = match response {
            Response::Version(p) => (SSH_FXP_VERSION, p.into()),
            Response::Status(p) => (SSH_FXP_STATUS, p.into()),
            Response::Name(p) => (SSH_FXP_NAME, p.into()),
        };

        let length = payload.len() as u32 + 1;
        let mut bytes = BytesMut::new();

        bytes.put_u32(length);
        bytes.put_u8(r#type);
        bytes.put_slice(&payload);

        bytes.freeze()
    }
}

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
