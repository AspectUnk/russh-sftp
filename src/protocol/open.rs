use super::{impl_request_id, RequestId, FileAttributes};

/// Opening flags according to the specification
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct OpenFlags(u32);

bitflags! {
    impl OpenFlags: u32 {
        const READ = 0x00000001;
        const WRITE = 0x00000002;
        const APPEND = 0x00000004;
        const CREATE = 0x00000008;
        const TRUNCATE = 0x00000010;
        const EXCLUDE = 0x00000020;
    }
}

/// Implementation for SSH_FXP_OPEN
#[derive(Debug, Serialize, Deserialize)]
pub struct Open {
    pub id: u32,
    pub filename: String,
    pub pflags: OpenFlags,
    pub attrs: FileAttributes,
}

impl_request_id!(Open);