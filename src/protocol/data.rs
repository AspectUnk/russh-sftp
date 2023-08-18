use super::{impl_packet_for, impl_request_id, Packet, RequestId};

/// Implementation for SSH_FXP_DATA
#[derive(Debug, Serialize, Deserialize)]
pub struct Data {
    pub id: u32,
    pub data: Vec<u8>,
}

impl_request_id!(Data);
impl_packet_for!(Data);
