use super::{impl_request_id, RequestId};

/// Implementation for SSH_FXP_REMOVE
#[derive(Debug, Serialize, Deserialize)]
pub struct Remove {
    pub id: u32,
    pub filename: String,
}

impl_request_id!(Remove);