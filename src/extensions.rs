use crate::{error::Error, ser};

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
