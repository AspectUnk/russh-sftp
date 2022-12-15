use bytes::{BufMut, Bytes, BytesMut};

use crate::{buf::TryBuf, error};

bitflags! {
    #[derive(Default)]
    pub struct FileAttrFlags: u32 {
        const SIZE = 0x00000001;
        const UIDGID = 0x00000002;
        const PERMISSIONS = 0x00000004;
        const ACMODTIME = 0x00000008;
    }
}

const S_IFDIR: u32 = 0x4000;
// const S_IFREG: u32 = 0x8000;
// const S_IFLNK: u32 = 0xA000;

#[derive(Debug)]
pub struct FileAttributes {
    pub size: Option<u64>,
    pub uid: Option<u32>,
    pub user: Option<String>,
    pub gid: Option<u32>,
    pub group: Option<String>,
    pub permissions: Option<u32>,
    pub atime: Option<u32>,
    pub mtime: Option<u32>,
}

impl Default for FileAttributes {
    fn default() -> Self {
        Self {
            size: Some(0),
            uid: Some(1),
            user: None,
            gid: Some(1),
            group: None,
            permissions: Some(0o777 | S_IFDIR),
            atime: Some(0),
            mtime: Some(0),
        }
    }
}

impl From<&FileAttributes> for Bytes {
    fn from(file_attrs: &FileAttributes) -> Self {
        let mut attrs = FileAttrFlags::default();

        if file_attrs.size.is_some() {
            attrs |= FileAttrFlags::SIZE;
        }

        if file_attrs.uid.is_some() || file_attrs.gid.is_some() {
            attrs |= FileAttrFlags::UIDGID;
        }

        if file_attrs.permissions.is_some() {
            attrs |= FileAttrFlags::PERMISSIONS;
        }

        if file_attrs.atime.is_some() || file_attrs.mtime.is_some() {
            attrs |= FileAttrFlags::ACMODTIME;
        }

        let mut bytes = BytesMut::new();

        bytes.put_u32(attrs.bits);

        if let Some(size) = file_attrs.size {
            bytes.put_u64(size);
        }

        if file_attrs.uid.is_some() || file_attrs.gid.is_some() {
            bytes.put_u32(file_attrs.uid.unwrap_or(0));
            bytes.put_u32(file_attrs.gid.unwrap_or(0));
        }

        if let Some(permissions) = file_attrs.permissions {
            bytes.put_u32(permissions);
        }

        if file_attrs.atime.is_some() || file_attrs.mtime.is_some() {
            bytes.put_u32(file_attrs.atime.unwrap_or(0));
            bytes.put_u32(file_attrs.mtime.unwrap_or(0))
        }

        bytes.freeze()
    }
}

impl TryFrom<&mut Bytes> for FileAttributes {
    type Error = error::Error;

    fn try_from(bytes: &mut Bytes) -> Result<Self, Self::Error> {
        let attrs = FileAttrFlags {
            bits: bytes.try_get_u32()?,
        };

        Ok(Self {
            size: if attrs.contains(FileAttrFlags::SIZE) {
                Some(bytes.try_get_u64()?)
            } else {
                None
            },
            uid: if attrs.contains(FileAttrFlags::UIDGID) {
                Some(bytes.try_get_u32()?)
            } else {
                None
            },
            user: None,
            gid: if attrs.contains(FileAttrFlags::UIDGID) {
                Some(bytes.try_get_u32()?)
            } else {
                None
            },
            group: None,
            permissions: if attrs.contains(FileAttrFlags::PERMISSIONS) {
                Some(bytes.try_get_u32()?)
            } else {
                None
            },
            atime: if attrs.contains(FileAttrFlags::ACMODTIME) {
                Some(bytes.try_get_u32()?)
            } else {
                None
            },
            mtime: if attrs.contains(FileAttrFlags::ACMODTIME) {
                Some(bytes.try_get_u32()?)
            } else {
                None
            },
        })
    }
}
