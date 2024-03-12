use super::{impl_packet_for, impl_request_id, FileAttributes, Packet, RequestId};

/// Opening flags according to the specification
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct OpenFlags(u32);

bitflags! {
    impl OpenFlags: u32 {
        const READ = 0x0000_0001;
        const WRITE = 0x0000_0002;
        const APPEND = 0x0000_0004;
        const CREATE = 0x0000_0008;
        const TRUNCATE = 0x0000_0010;
        const EXCLUDE = 0x0000_0020;
    }
}

/// Implementation for `SSH_FXP_OPEN`
#[derive(Debug, Serialize, Deserialize)]
pub struct Open {
    pub id: u32,
    pub filename: String,
    pub pflags: OpenFlags,
    pub attrs: FileAttributes,
}

impl_request_id!(Open);
impl_packet_for!(Open);
