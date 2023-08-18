use super::{impl_request_id, RequestId, FileAttributes};

/// Implementation for SSH_FXP_... SETSTAT and MKDIR
#[derive(Debug, Serialize, Deserialize)]
pub struct PathAttrs {
    pub id: u32,
    pub path: String,
    pub attrs: FileAttributes,
}

impl_request_id!(PathAttrs);