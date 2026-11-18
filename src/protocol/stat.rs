use super::{impl_packet_for, impl_request_id, Packet, RequestId, SftpString};

/// Implementation for `SSH_FXP_STAT`
#[derive(Debug, Serialize, Deserialize)]
pub struct Stat {
    pub id: u32,
    pub path: SftpString,
}

impl_request_id!(Stat);
impl_packet_for!(Stat);
