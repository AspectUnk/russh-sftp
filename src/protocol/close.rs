use super::{impl_packet_for, impl_request_id, Packet, RequestId, SftpHandle};

/// Implementation for `SSH_FXP_CLOSE`
#[derive(Debug, Serialize, Deserialize)]
pub struct Close {
    pub id: u32,
    pub handle: SftpHandle,
}

impl_request_id!(Close);
impl_packet_for!(Close);
