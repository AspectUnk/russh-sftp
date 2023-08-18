use bytes::{BufMut, Bytes, BytesMut};
use serde::{Serialize, ser::SerializeStruct};
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::{fs::Metadata, time::UNIX_EPOCH};

use crate::{buf::TryBuf, error, utils};

    /// Attributes flags according to the specification
#[derive(Default, Serialize, Deserialize)]
pub struct FileAttr(u32);

    /// Types according to mode unix
#[derive(Default, Serialize, Deserialize)]
pub struct FileType(u32);

bitflags! {
    impl FileAttr: u32 {
        const SIZE = 0x00000001;
        const UIDGID = 0x00000002;
        const PERMISSIONS = 0x00000004;
        const ACMODTIME = 0x00000008;
    }

    impl FileType: u32 {
        const FIFO = 0x1000;
        const CHR = 0x2000;
        const DIR = 0x4000;
        const BLK = 0x6000;
        const REG = 0x8000;
        const LNK = 0xA000;
        const NAM = 0x5000;
    }

    // TODO: Add FilePermission
}

/// Used in the implementation of other packages.
///
/// The fields `user` and `group` are string names of users
/// and groups for clients that can be displayed from longname.
/// Can be omitted.
///
/// The `flags` field is omitted because it
/// is set by itself depending on the flags
#[derive(Debug, Deserialize)]
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

macro_rules! impl_fn_type {
    ($get_name:ident, $set_name:ident, $doc_name:expr, $flag:ident) => {
        #[doc = "Returns `true` if is a "]
        #[doc = $doc_name]
        pub fn $get_name(&self) -> bool {
            self.permissions.map_or(false, |b| {
                FileType::from_bits_truncate(b).contains(FileType::$flag)
            })
        }

        #[doc = "Set flag if is a "]
        #[doc = $doc_name]
        #[doc = " or not"]
        pub fn $set_name(&mut self, $get_name: bool) {
            match $get_name {
                true => self.set_type(FileType::$flag),
                false => self.remove_type(FileType::$flag),
            }
        }
    };
}

impl FileAttributes {
    impl_fn_type!(is_dir, set_dir, "dir", DIR);
    impl_fn_type!(is_regular, set_regular, "regular", REG);
    impl_fn_type!(is_symlink, set_symlink, "symlink", LNK);
    impl_fn_type!(is_character, set_character, "character", CHR);
    impl_fn_type!(is_block, set_block, "block", BLK);
    impl_fn_type!(is_fifo, set_fifo, "fifo", FIFO);

    /// Set type flag
    pub fn set_type(&mut self, r#type: FileType) {
        let perms = self.permissions.unwrap_or(0);
        self.permissions = Some(perms | r#type.bits());
    }

    /// Remove type flag
    pub fn remove_type(&mut self, r#type: FileType) {
        let perms = self.permissions.unwrap_or(0);
        self.permissions = Some(perms & !r#type.bits());
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
            permissions: Some(0o777 | FileType::DIR.bits()),
            atime: Some(0),
            mtime: Some(0),
        }
    }
}

/// For simple conversion of `Metadata` into file attributes
impl From<&Metadata> for FileAttributes {
    fn from(metadata: &Metadata) -> Self {
        let mut attrs = Self {
            size: Some(metadata.len()),
            #[cfg(unix)]
            uid: Some(metadata.uid()),
            #[cfg(unix)]
            gid: Some(metadata.gid()),
            #[cfg(windows)]
            permissions: Some(if metadata.permissions().readonly() {
                0o555
            } else {
                0o777
            }),
            #[cfg(unix)]
            permissions: Some(metadata.mode()),
            atime: Some(utils::unix(metadata.modified().unwrap_or(UNIX_EPOCH))),
            mtime: Some(utils::unix(metadata.accessed().unwrap_or(UNIX_EPOCH))),
            ..Default::default()
        };

        attrs.set_dir(metadata.is_dir());
        attrs.set_regular(!metadata.is_dir());

        attrs
    }
}

impl Serialize for FileAttributes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("FileAttributes", 7)?;
        let mut attrs = FileAttr::default();

        if self.size.is_some() {
            attrs |= FileAttr::SIZE;
        }

        if self.uid.is_some() || self.gid.is_some() {
            attrs |= FileAttr::UIDGID;
        }

        if self.permissions.is_some() {
            attrs |= FileAttr::PERMISSIONS;
        }

        if self.atime.is_some() || self.mtime.is_some() {
            attrs |= FileAttr::ACMODTIME;
        }

        s.serialize_field("attrs", &attrs)?;

        if let Some(size) = self.size {
            s.serialize_field("size", &size)?;
        }

        if self.uid.is_some() || self.gid.is_some() {
            s.serialize_field("uid", &self.uid.unwrap_or(0))?;
            s.serialize_field("gid", &self.gid.unwrap_or(0))?;
        }

        if let Some(permissions) = self.permissions {
            s.serialize_field("permissions", &permissions)?;
        }

        if self.atime.is_some() || self.mtime.is_some() {
            s.serialize_field("atime", &self.atime.unwrap_or(0))?;
            s.serialize_field("mtime", &self.mtime.unwrap_or(0))?;
        }

        s.end()
    }
}

impl From<&FileAttributes> for Bytes {
    fn from(file_attrs: &FileAttributes) -> Self {
        let mut attrs = FileAttr::default();

        if file_attrs.size.is_some() {
            attrs |= FileAttr::SIZE;
        }

        if file_attrs.uid.is_some() || file_attrs.gid.is_some() {
            attrs |= FileAttr::UIDGID;
        }

        if file_attrs.permissions.is_some() {
            attrs |= FileAttr::PERMISSIONS;
        }

        if file_attrs.atime.is_some() || file_attrs.mtime.is_some() {
            attrs |= FileAttr::ACMODTIME;
        }

        let mut bytes = BytesMut::new();

        bytes.put_u32(attrs.bits());

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
            bytes.put_u32(file_attrs.mtime.unwrap_or(0));
        }

        bytes.freeze()
    }
}

impl TryFrom<&mut Bytes> for FileAttributes {
    type Error = error::Error;

    fn try_from(bytes: &mut Bytes) -> Result<Self, Self::Error> {
        let attrs = FileAttr::from_bits_truncate(bytes.try_get_u32()?);

        Ok(Self {
            size: if attrs.contains(FileAttr::SIZE) {
                Some(bytes.try_get_u64()?)
            } else {
                None
            },
            uid: if attrs.contains(FileAttr::UIDGID) {
                Some(bytes.try_get_u32()?)
            } else {
                None
            },
            user: None,
            gid: if attrs.contains(FileAttr::UIDGID) {
                Some(bytes.try_get_u32()?)
            } else {
                None
            },
            group: None,
            permissions: if attrs.contains(FileAttr::PERMISSIONS) {
                Some(bytes.try_get_u32()?)
            } else {
                None
            },
            atime: if attrs.contains(FileAttr::ACMODTIME) {
                Some(bytes.try_get_u32()?)
            } else {
                None
            },
            mtime: if attrs.contains(FileAttr::ACMODTIME) {
                Some(bytes.try_get_u32()?)
            } else {
                None
            },
        })
    }
}
