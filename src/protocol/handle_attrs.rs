/// Implementation for SSH_FXP_FSETSTAT
use super::{impl_request_id, RequestId, FileAttributes};

#[derive(Debug, Serialize, Deserialize)]
pub struct HandleAttrs {
    pub id: u32,
    pub handle: String,
    pub attrs: FileAttributes,
}

impl_request_id!(HandleAttrs);