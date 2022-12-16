mod attrs;
mod data;
mod extended;
mod extended_reply;
mod file_attrs;
mod handle;
mod handle_attrs;
mod init;
mod name;
mod open;
mod path;
mod path_attrs;
mod read;
mod remove;
mod rename;
mod status;
mod symlink;
mod version;
mod write;

use bytes::{BufMut, Bytes, BytesMut};

use crate::{buf::TryBuf, error::Error};

pub use self::{
    attrs::Attrs,
    data::Data,
    extended::Extended,
    extended_reply::ExtendedReply,
    file_attrs::{FileAttrFlags, FileAttributes, FileMode},
    handle::Handle,
    handle_attrs::HandleAttrs,
    init::Init,
    name::{File, Name},
    open::{Open, OpenFlags},
    path::Path,
    path_attrs::PathAttrs,
    read::Read,
    remove::Remove,
    rename::Rename,
    status::{Status, StatusCode},
    symlink::Symlink,
    version::Version,
    write::Write,
};

pub const VERSION: u32 = 3;

const SSH_FXP_INIT: u8 = 1;
const SSH_FXP_VERSION: u8 = 2;
const SSH_FXP_OPEN: u8 = 3;
const SSH_FXP_CLOSE: u8 = 4;
const SSH_FXP_READ: u8 = 5;
const SSH_FXP_WRITE: u8 = 6;
const SSH_FXP_LSTAT: u8 = 7;
const SSH_FXP_FSTAT: u8 = 8;
const SSH_FXP_SETSTAT: u8 = 9;
const SSH_FXP_FSETSTAT: u8 = 10;
const SSH_FXP_OPENDIR: u8 = 11;
const SSH_FXP_READDIR: u8 = 12;
const SSH_FXP_REMOVE: u8 = 13;
const SSH_FXP_MKDIR: u8 = 14;
const SSH_FXP_RMDIR: u8 = 15;
const SSH_FXP_REALPATH: u8 = 16;
const SSH_FXP_STAT: u8 = 17;
const SSH_FXP_RENAME: u8 = 18;
const SSH_FXP_READLINK: u8 = 19;
const SSH_FXP_SYMLINK: u8 = 20;

const SSH_FXP_STATUS: u8 = 101;
const SSH_FXP_HANDLE: u8 = 102;
const SSH_FXP_DATA: u8 = 103;
const SSH_FXP_NAME: u8 = 104;
const SSH_FXP_ATTRS: u8 = 105;

const SSH_FXP_EXTENDED: u8 = 200;
const SSH_FXP_EXTENDED_REPLY: u8 = 201;

pub(crate) trait RequestId: Sized {
    fn get_request_id(&self) -> u32;
}

macro_rules! impl_request_id {
    ($packet:ty) => {
        impl RequestId for $packet {
            fn get_request_id(&self) -> u32 {
                self.id
            }
        }
    };
}

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
pub(crate) use impl_request_id;

#[derive(Debug)]
pub(crate) enum Request {
    Init(Init),
    Open(Open),
    Close(Handle),
    Read(Read),
    Write(Write),
    Lstat(Path),
    Fstat(Handle),
    SetStat(PathAttrs),
    FSetStat(HandleAttrs),
    OpenDir(Path),
    ReadDir(Handle),
    Remove(Remove),
    Mkdir(PathAttrs),
    Rmdir(Path),
    RealPath(Path),
    Stat(Path),
    Rename(Rename),
    ReadLink(Path),
    Symlink(Symlink),
    Extended(Extended),
}

impl Request {
    pub fn get_request_id(&self) -> u32 {
        match self {
            Self::Open(open) => open.get_request_id(),
            Self::Close(close) => close.get_request_id(),
            Self::Read(read) => read.get_request_id(),
            Self::Write(write) => write.get_request_id(),
            Self::Lstat(lstat) => lstat.get_request_id(),
            Self::Fstat(fstat) => fstat.get_request_id(),
            Self::SetStat(setstat) => setstat.get_request_id(),
            Self::FSetStat(fsetstat) => fsetstat.get_request_id(),
            Self::OpenDir(opendir) => opendir.get_request_id(),
            Self::ReadDir(readdir) => readdir.get_request_id(),
            Self::Remove(remove) => remove.get_request_id(),
            Self::Mkdir(mkdir) => mkdir.get_request_id(),
            Self::Rmdir(rmdir) => rmdir.get_request_id(),
            Self::RealPath(realpath) => realpath.get_request_id(),
            Self::Stat(stat) => stat.get_request_id(),
            Self::Rename(rename) => rename.get_request_id(),
            Self::ReadLink(readlink) => readlink.get_request_id(),
            Self::Symlink(symlink) => symlink.get_request_id(),
            Self::Extended(extended) => extended.get_request_id(),
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
            SSH_FXP_OPEN => Self::Open(Open::try_from(bytes)?),
            SSH_FXP_CLOSE => Self::Close(Handle::try_from(bytes)?),
            SSH_FXP_READ => Self::Read(Read::try_from(bytes)?),
            SSH_FXP_WRITE => Self::Write(Write::try_from(bytes)?),
            SSH_FXP_LSTAT => Self::Lstat(Path::try_from(bytes)?),
            SSH_FXP_FSTAT => Self::Fstat(Handle::try_from(bytes)?),
            SSH_FXP_SETSTAT => Self::SetStat(PathAttrs::try_from(bytes)?),
            SSH_FXP_FSETSTAT => Self::FSetStat(HandleAttrs::try_from(bytes)?),
            SSH_FXP_OPENDIR => Self::OpenDir(Path::try_from(bytes)?),
            SSH_FXP_READDIR => Self::ReadDir(Handle::try_from(bytes)?),
            SSH_FXP_REMOVE => Self::Remove(Remove::try_from(bytes)?),
            SSH_FXP_MKDIR => Self::Mkdir(PathAttrs::try_from(bytes)?),
            SSH_FXP_RMDIR => Self::Rmdir(Path::try_from(bytes)?),
            SSH_FXP_REALPATH => Self::RealPath(Path::try_from(bytes)?),
            SSH_FXP_STAT => Self::Stat(Path::try_from(bytes)?),
            SSH_FXP_RENAME => Self::Rename(Rename::try_from(bytes)?),
            SSH_FXP_READLINK => Self::ReadLink(Path::try_from(bytes)?),
            SSH_FXP_SYMLINK => Self::Symlink(Symlink::try_from(bytes)?),
            SSH_FXP_EXTENDED => Self::Extended(Extended::try_from(bytes)?),
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
    Data(Data),
    Name(Name),
    Attrs(Attrs),
    ExtendedReply(ExtendedReply),
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
            Response::Version(version) => (SSH_FXP_VERSION, version.into()),
            Response::Status(status) => (SSH_FXP_STATUS, status.into()),
            Response::Handle(handle) => (SSH_FXP_HANDLE, handle.into()),
            Response::Data(data) => (SSH_FXP_DATA, data.into()),
            Response::Name(name) => (SSH_FXP_NAME, name.into()),
            Response::Attrs(attrs) => (SSH_FXP_ATTRS, attrs.into()),
            Response::ExtendedReply(reply) => (SSH_FXP_EXTENDED_REPLY, reply.into()),
        };

        let length = payload.len() as u32 + 1;

        let mut bytes = BytesMut::new();
        bytes.put_u32(length);
        bytes.put_u8(r#type);
        bytes.put_slice(&payload);
        bytes.freeze()
    }
}
