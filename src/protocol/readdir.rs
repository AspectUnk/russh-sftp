use super::{impl_packet_for, impl_request_id, Packet, RequestId, SftpHandle};

/// Implementation for `SSH_FXP_READDIR`
#[derive(Debug, Serialize, Deserialize)]
pub struct ReadDir {
    pub id: u32,
    pub handle: SftpHandle,
}

impl_request_id!(ReadDir);
impl_packet_for!(ReadDir);
