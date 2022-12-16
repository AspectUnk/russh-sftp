mod attrs;
mod data;
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

pub const SSH_FXP_EXTENDED: u8 = 200;
pub const SSH_FXP_EXTENDED_REPLY: u8 = 201;

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

macro_rules! impl_packet_for {
    ($name:ident, $packet:ty) => {
        impl From<$name> for $packet {
            fn from(input: $name) -> Self {
                Self::$name(input)
            }
        }
    };
}

pub(crate) use impl_request_id;
pub(crate) use impl_packet_for;

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
}

impl Request {
    pub fn get_id(&self) -> u32 {
        match self {
            Self::Open(open) => open.get_id(),
            Self::Close(close) => close.get_id(),
            Self::Read(read) => read.get_id(),
            Self::Write(write) => write.get_id(),
            Self::Lstat(lstat) => lstat.get_id(),
            Self::Fstat(fstat) => fstat.get_id(),
            Self::SetStat(setstat) => setstat.get_id(),
            Self::FSetStat(fsetstat) => fsetstat.get_id(),
            Self::OpenDir(opendir) => opendir.get_id(),
            Self::ReadDir(readdir) => readdir.get_id(),
            Self::Remove(remove) => remove.get_id(),
            Self::Mkdir(mkdir) => mkdir.get_id(),
            Self::Rmdir(rmdir) => rmdir.get_id(),
            Self::RealPath(realpath) => realpath.get_id(),
            Self::Stat(stat) => stat.get_id(),
            Self::Rename(rename) => rename.get_id(),
            Self::ReadLink(readlink) => readlink.get_id(),
            Self::Symlink(symlink) => symlink.get_id(),
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
            1 => Self::Init(Init::try_from(bytes)?),
            3 => Self::Open(Open::try_from(bytes)?),
            4 => Self::Close(Handle::try_from(bytes)?),
            5 => Self::Read(Read::try_from(bytes)?),
            6 => Self::Write(Write::try_from(bytes)?),
            7 => Self::Lstat(Path::try_from(bytes)?),
            8 => Self::Fstat(Handle::try_from(bytes)?),
            9 => Self::SetStat(PathAttrs::try_from(bytes)?),
            10 => Self::FSetStat(HandleAttrs::try_from(bytes)?),
            11 => Self::OpenDir(Path::try_from(bytes)?),
            12 => Self::ReadDir(Handle::try_from(bytes)?),
            13 => Self::Remove(Remove::try_from(bytes)?),
            14 => Self::Mkdir(PathAttrs::try_from(bytes)?),
            15 => Self::Rmdir(Path::try_from(bytes)?),
            16 => Self::RealPath(Path::try_from(bytes)?),
            17 => Self::Stat(Path::try_from(bytes)?),
            18 => Self::Rename(Rename::try_from(bytes)?),
            19 => Self::ReadLink(Path::try_from(bytes)?),
            20 => Self::Symlink(Symlink::try_from(bytes)?),
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
            Response::Version(version) => (2, version.into()),
            Response::Status(status) => (101, status.into()),
            Response::Handle(handle) => (102, handle.into()),
            Response::Data(data) => (103, data.into()),
            Response::Name(name) => (104, name.into()),
            Response::Attrs(attrs) => (105, attrs.into()),
        };

        let length = payload.len() as u32 + 1;

        let mut bytes = BytesMut::new();
        bytes.put_u32(length);
        bytes.put_u8(r#type);
        bytes.put_slice(&payload);
        bytes.freeze()
    }
}
