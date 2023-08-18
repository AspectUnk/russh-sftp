use super::{impl_request_id, RequestId};

/// Implementation for SSH_FXP_READ
#[derive(Debug, Serialize, Deserialize)]
pub struct Read {
    pub id: u32,
    pub handle: String,
    pub offset: u64,
    pub len: u32,
}

impl_request_id!(Read);