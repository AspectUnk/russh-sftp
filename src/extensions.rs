use crate::{error::Error, ser};

pub const LIMITS: &str = "limits@openssh.com";
pub const FSYNC: &str = "fsync@openssh.com";
pub const STATVFS: &str = "statvfs@openssh.com";

#[derive(Debug, Serialize, Deserialize)]
pub struct LimitsExtension {
    pub max_packet_len: u64,
    pub max_read_len: u64,
    pub max_write_len: u64,
    pub max_open_handles: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FsyncExtension {
    pub handle: String,
}

impl TryInto<Vec<u8>> for FsyncExtension {
    type Error = Error;

    fn try_into(self) -> Result<Vec<u8>, Self::Error> {
        ser::to_bytes(&self).map(|b| b.to_vec())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatvfsExtension {
    pub path: String,
}

impl TryInto<Vec<u8>> for StatvfsExtension {
    type Error = Error;

    fn try_into(self) -> Result<Vec<u8>, Self::Error> {
        ser::to_bytes(&self).map(|b| b.to_vec())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Statvfs {
    /// The file system block size
    pub block_size: u64,
    /// The fundamental file system block size
    pub fragment_size: u64,
    /// The number of blocks.
    ///
    /// Units are in units of `fragment_size`
    pub blocks: u64,
    /// The number of free blocks in the file system
    pub blocks_free: u64,
    /// The number of free blocks for unprivileged users
    pub blocks_avail: u64,
    /// The total number of file inodes
    pub inodes: u64,
    /// The number of free file inodes
    pub inodes_free: u64,
    /// The number of free file inodes for unprivileged users
    pub inodes_avail: u64,
    /// The file system id
    pub fs_id: u64,
    /// The mount flags
    pub flags: u64,
    /// The maximum filename length
    pub name_max: u64,
}
