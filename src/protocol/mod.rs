mod init;
mod name;
mod path;
mod status;

use thiserror::Error;

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

pub const SSH_FX_OK: u32 = 0;
pub const SSH_FX_EOF: u32 = 1;
pub const SSH_FX_NO_SUCH_FILE: u32 = 2;
pub const SSH_FX_PERMISSION_DENIED: u32 = 3;
pub const SSH_FX_FAILURE: u32 = 4;
pub const SSH_FX_BAD_MESSAGE: u32 = 5;
pub const SSH_FX_NO_CONNECTION: u32 = 6;
pub const SSH_FX_CONNECTION_LOST: u32 = 7;
pub const SSH_FX_OP_UNSUPPORTED: u32 = 8;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
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
        match value {
            SSH_FX_OK => Self::Ok,
            SSH_FX_EOF => Self::Eof,
            SSH_FX_NO_SUCH_FILE => Self::NoSuchFile,
            SSH_FX_PERMISSION_DENIED => Self::PermissionDenined,
            SSH_FX_FAILURE => Self::Failure,
            SSH_FX_BAD_MESSAGE => Self::BadMessage,
            SSH_FX_NO_CONNECTION => Self::NoConnection,
            SSH_FX_CONNECTION_LOST => Self::ConnectionLost,
            SSH_FX_OP_UNSUPPORTED => Self::OpUnsupported,
            _ => Self::Failure,
        }
    }
}
