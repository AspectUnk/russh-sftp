use super::{impl_packet_for, impl_request_id, Packet, RequestId};

/// Implementation for SSH_FXP_EXTENDED
#[derive(Debug, Serialize, Deserialize)]
pub struct Extended {
    pub id: u32,
    pub request: String,
    pub data: Vec<u8>,
}

impl_request_id!(Extended);
impl_packet_for!(Extended);

/// Implementation for SSH_FXP_EXTENDED_REPLY
#[derive(Debug, Serialize, Deserialize)]
pub struct ExtendedReply {
    pub id: u32,
    pub data: Vec<u8>,
}

impl_request_id!(ExtendedReply);
impl_packet_for!(ExtendedReply);
