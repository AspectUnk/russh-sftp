use std::fmt;

use super::{impl_request_id, RequestId};

/// Implementation for SSH_FXP_WRITE
#[derive(Serialize, Deserialize)]
pub struct Write {
    pub id: u32,
    pub handle: String,
    pub offset: u64,
    pub data: Vec<u8>,
}

impl fmt::Debug for Write {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Write")
            .field("id", &self.id)
            .field("handle", &self.handle)
            .field("offset", &self.offset)
            .field("data", &self.data.len())
            .finish()
    }
}

impl_request_id!(Write);