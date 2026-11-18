use super::{impl_packet_for, impl_request_id, Packet, RequestId, SftpString};

/// Implementation for `SSH_FXP_REALPATH`
#[derive(Debug, Serialize, Deserialize)]
pub struct RealPath {
    pub id: u32,
    pub path: SftpString,
}

impl_request_id!(RealPath);
impl_packet_for!(RealPath);
