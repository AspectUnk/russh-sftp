mod init;

use bytes::Bytes;

pub use self::init::{Init, Version};
use crate::{buf::TryBuf, error::Error, ErrorProtocol};

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

#[derive(Debug)]
pub enum Packet {
    Init(init::Init),
    Version(init::Version),
}

impl TryFrom<&mut Bytes> for Packet {
    type Error = Error;

    fn try_from(bytes: &mut Bytes) -> Result<Self, Self::Error> {
        let r#type = bytes.try_get_u8()?;
        Ok(match r#type {
            SSH_FXP_INIT => Self::Init(Init::try_from(bytes)?),
            SSH_FXP_VERSION => Self::Version(Version::try_from(bytes)?),
            _ => return Err(Error::Protocol(ErrorProtocol::OpUnsupported)),
        })
    }
}
