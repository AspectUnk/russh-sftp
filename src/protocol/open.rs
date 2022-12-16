use bytes::Bytes;

use crate::{buf::TryBuf, error};

use super::{impl_request_id, RequestId, FileAttributes};

bitflags! {
    /// Opening flags according to the specification
    #[derive(Default)]
    pub struct OpenFlags: u32 {
        const READ = 0x00000001;
        const WRITE = 0x00000002;
        const APPEND = 0x00000004;
        const CREATE = 0x00000008;
        const TRUNCATE = 0x00000010;
        const EXCLUDE = 0x00000020;
    }
}

/// Implementation for SSH_FXP_OPEN
#[derive(Debug)]
pub struct Open {
    pub id: u32,
    pub filename: String,
    pub pflags: OpenFlags,
    pub attrs: FileAttributes,
}

impl_request_id!(Open);

impl TryFrom<&mut Bytes> for Open {
    type Error = error::Error;

    fn try_from(bytes: &mut Bytes) -> Result<Self, Self::Error> {
        Ok(Self {
            id: bytes.try_get_u32()?,
            filename: bytes.try_get_string()?,
            pflags: OpenFlags {
                bits: bytes.try_get_u32()?,
            },
            attrs: FileAttributes::try_from(bytes)?,
        })
    }
}
