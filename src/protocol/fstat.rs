use super::{impl_packet_for, impl_request_id, Packet, RequestId, SftpString};

/// Implementation for `SSH_FXP_FSTAT`
#[derive(Debug, Serialize, Deserialize)]
pub struct Fstat {
    pub id: u32,
    pub handle: SftpString,
}

impl_request_id!(Fstat);
impl_packet_for!(Fstat);
