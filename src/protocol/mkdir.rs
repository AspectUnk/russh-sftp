use super::{impl_packet_for, impl_request_id, FileAttributes, Packet, RequestId, SftpString};

/// Implementation for `SSH_FXP_MKDIR`
#[derive(Debug, Serialize, Deserialize)]
pub struct MkDir {
    pub id: u32,
    pub path: SftpString,
    pub attrs: FileAttributes,
}

impl_request_id!(MkDir);
impl_packet_for!(MkDir);
