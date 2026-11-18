use std::fmt;

use super::{impl_packet_for, impl_request_id, Packet, RequestId};

/// Opaque handle of an open file or directory. The contents are up
/// to the server, so the bytes are kept as is and must not be modified
#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SftpHandle(#[serde(with = "serde_bytes")] Vec<u8>);

impl SftpHandle {
    /// Returns the original bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Converts into the original bytes
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}

impl AsRef<[u8]> for SftpHandle {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl<T: AsRef<[u8]> + ?Sized> From<&T> for SftpHandle {
    fn from(value: &T) -> Self {
        Self(value.as_ref().to_vec())
    }
}

impl From<String> for SftpHandle {
    fn from(value: String) -> Self {
        Self(value.into_bytes())
    }
}

impl From<Vec<u8>> for SftpHandle {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

impl fmt::Debug for SftpHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\"", self.0.escape_ascii())
    }
}

/// Implementation for `SSH_FXP_HANDLE`
#[derive(Debug, Serialize, Deserialize)]
pub struct Handle {
    pub id: u32,
    pub handle: SftpHandle,
}

impl_request_id!(Handle);
impl_packet_for!(Handle);
