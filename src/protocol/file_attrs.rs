use bytes::{BufMut, Bytes, BytesMut};
use std::{fs::Metadata, time::UNIX_EPOCH};

use crate::{buf::TryBuf, error, utils};

bitflags! {
    /// Attributes flags according to the specification
    #[derive(Default)]
    pub struct FileAttrFlags: u32 {
        const SIZE = 0x00000001;
        const UIDGID = 0x00000002;
        const PERMISSIONS = 0x00000004;
        const ACMODTIME = 0x00000008;
    }

    /// Modes according to unix
    #[derive(Default)]
    pub struct FileMode: u32 {
        const FIFO = 0x1000;
        const CHR = 0x2000;
        const DIR = 0x4000;
        const BLK = 0x6000;
        const REG = 0x8000;
        const LNK = 0xA000;
        const NAM = 0x5000;
    }
}

/// Used in the implementation of other packages.
///
/// The fields `user` and `group` are string names of users
/// and groups for clients that can be displayed from longname.
/// Can be omitted.
///
/// The `flags` field is omitted because it
/// is set by itself depending on the flags
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

impl FileAttributes {
    /// Returns `true` if is a folder
    pub fn is_dir(&self) -> bool {
        self.permissions
            .map_or(false, |b| FileMode { bits: b }.contains(FileMode::DIR))
    }

    /// Returns `true` if is a regular file
    pub fn is_regular(&self) -> bool {
        self.permissions
            .map_or(false, |b| FileMode { bits: b }.contains(FileMode::REG))
    }

    /// Returns `true` if is a symlink
    pub fn is_symlink(&self) -> bool {
        self.permissions
            .map_or(false, |b| FileMode { bits: b }.contains(FileMode::LNK))
    }

    /// Set mode flag
    pub fn set_type(&mut self, r#type: FileMode) {
        let perms = self.permissions.unwrap_or(0);
        self.permissions = Some(perms | r#type.bits);
    }

    /// Remove mode flag
    pub fn remove_type(&mut self, r#type: FileMode) {
        let perms = self.permissions.unwrap_or(0);
        self.permissions = Some(perms & !r#type.bits);
    }

    /// Set flag if is a dir or not
    pub fn set_dir(&mut self, is_dir: bool) {
        match is_dir {
            true => self.set_type(FileMode::DIR),
            false => self.remove_type(FileMode::DIR),
        }
    }

    /// Set flag if is a regular or not
    pub fn set_regular(&mut self, is_regular: bool) {
        match is_regular {
            true => self.set_type(FileMode::REG),
            false => self.remove_type(FileMode::REG),
        }
    }

    /// Set flag if is a symlink or not
    pub fn set_symlink(&mut self, is_symlink: bool) {
        match is_symlink {
            true => self.set_type(FileMode::LNK),
            false => self.remove_type(FileMode::LNK),
        }
    }
}

/// For packets which require dummy attributes
impl Default for FileAttributes {
    fn default() -> Self {
        Self {
            size: Some(0),
            uid: Some(0),
            user: None,
            gid: Some(0),
            group: None,
            permissions: Some(0o777 | FileMode::DIR.bits),
            atime: Some(0),
            mtime: Some(0),
        }
    }
}

/// For simple conversion of `Metadata` into file attributes
///
/// Support `MetadataExt` will be added later
impl From<&Metadata> for FileAttributes {
    fn from(metadata: &Metadata) -> Self {
        let mut attrs = Self {
            size: Some(metadata.len()),
            permissions: Some(if metadata.permissions().readonly() {
                0o555
            } else {
                0o777
            }),
            atime: Some(utils::unix(metadata.modified().unwrap_or(UNIX_EPOCH))),
            mtime: Some(utils::unix(metadata.accessed().unwrap_or(UNIX_EPOCH))),
            ..Default::default()
        };

        attrs.set_dir(metadata.is_dir());
        attrs.set_regular(!metadata.is_dir());

        attrs
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
