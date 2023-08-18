use super::{impl_request_id, RequestId};

/// Implementation for SSH_FXP_... LSTAT, OPENDIR,
/// RMDIR, REALPATH, STAT and READLINK
#[derive(Debug, Serialize, Deserialize)]
pub struct Path {
    pub id: u32,
    pub path: String,
}

impl_request_id!(Path);