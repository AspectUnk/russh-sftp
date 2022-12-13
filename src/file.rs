use bytes::{BufMut, Bytes, BytesMut};

const SSH_FILEXFER_ATTR_SIZE: u32 = 0x00000001;
const SSH_FILEXFER_ATTR_UIDGID: u32 = 0x00000002;
const SSH_FILEXFER_ATTR_PERMISSIONS: u32 = 0x00000004;
const SSH_FILEXFER_ATTR_ACMODTIME: u32 = 0x00000008;

const S_IFDIR: u32 = 0x4000;
const S_IFREG: u32 = 0x8000;
const S_IFLNK: u32 = 0xA000;

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
        let mut attrs: u32 = 0;

        if file_attrs.size.is_some() {
            attrs |= SSH_FILEXFER_ATTR_SIZE;
        }

        if file_attrs.uid.is_some() || file_attrs.gid.is_some() {
            attrs |= SSH_FILEXFER_ATTR_UIDGID;
        }

        if file_attrs.permissions.is_some() {
            attrs |= SSH_FILEXFER_ATTR_PERMISSIONS;
        }

        if file_attrs.atime.is_some() || file_attrs.mtime.is_some() {
            attrs |= SSH_FILEXFER_ATTR_ACMODTIME;
        }

        let mut bytes = BytesMut::new();

        bytes.put_u32(attrs);

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
