use super::{impl_request_id, RequestId};

/// Implementation for SSH_FXP_RENAME
#[derive(Debug, Serialize, Deserialize)]
pub struct Rename {
    pub id: u32,
    pub oldpath: String,
    pub newpath: String,
}

impl_request_id!(Rename);