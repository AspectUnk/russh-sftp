mod handle;
mod init;
mod name;
mod path;
mod status;

use bytes::{BufMut, Bytes, BytesMut};

use crate::{buf::TryBuf, error::Error};

pub use self::{
    handle::Handle,
    init::{Init, Version},
    name::{File, Name},
    path::Path,
    status::{Status, StatusCode},
};

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

pub(crate) trait RequestId: Sized {
    fn get_id(&self) -> u32;
}

macro_rules! impl_request_id {
    ($packet:ty) => {
        impl RequestId for $packet {
            fn get_id(&self) -> u32 {
                self.id
            }
        }
    };
}

pub(crate) use impl_request_id;

#[derive(Debug)]
pub(crate) enum Request {
    Init(Init),
    Close(Handle),
    OpenDir(Path),
    ReadDir(Handle),
    RealPath(Path),
}

impl Request {
    pub fn get_id(&self) -> u32 {
        match self {
            Self::Close(close) => close.get_id(),
            Self::OpenDir(opendir) => opendir.get_id(),
            Self::ReadDir(readdir) => readdir.get_id(),
            Self::RealPath(realpath) => realpath.get_id(),
            _ => 0,
        }
    }
}

impl TryFrom<&mut Bytes> for Request {
    type Error = Error;

    fn try_from(bytes: &mut Bytes) -> Result<Self, Self::Error> {
        let r#type = bytes.try_get_u8()?;
        debug!("packet type {}", r#type);

        let request = match r#type {
            SSH_FXP_INIT => Self::Init(Init::try_from(bytes)?),
            SSH_FXP_CLOSE => Self::Close(Handle::try_from(bytes)?),
            SSH_FXP_OPENDIR => Self::OpenDir(Path::try_from(bytes)?),
            SSH_FXP_READDIR => Self::ReadDir(Handle::try_from(bytes)?),
            SSH_FXP_REALPATH => Self::RealPath(Path::try_from(bytes)?),
            _ => return Err(Error::BadMessage),
        };

        Ok(request)
    }
}

#[derive(Debug)]
pub(crate) enum Response {
    Version(Version),
    Status(Status),
    Handle(Handle),
    Name(Name),
}

impl Response {
    pub fn status(id: u32, status_code: StatusCode, msg: &str, tag: &str) -> Self {
        Response::Status(Status {
            id,
            status_code,
            error_message: msg.to_string(),
            language_tag: tag.to_string(),
        })
    }

    pub fn error(id: u32, status_code: StatusCode) -> Self {
        Self::status(id, status_code, &status_code.to_string(), "en-US")
    }
}

impl From<Response> for Bytes {
    fn from(response: Response) -> Self {
        let (r#type, payload): (u8, Bytes) = match response {
            Response::Version(p) => (SSH_FXP_VERSION, p.into()),
            Response::Status(p) => (SSH_FXP_STATUS, p.into()),
            Response::Handle(p) => (SSH_FXP_HANDLE, p.into()),
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
