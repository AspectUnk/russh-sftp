use super::{impl_request_id, RequestId, Packet, impl_packet_for};

/// Implementation for SSH_FXP_... CLOSE, FSTAT and READDIR
#[derive(Debug, Serialize, Deserialize)]
pub struct Handle {
    pub id: u32,
    pub handle: String,
}

impl_request_id!(Handle);
impl_packet_for!(Handle);