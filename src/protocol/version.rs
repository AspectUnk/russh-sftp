use bytes::{BufMut, Bytes, BytesMut};
use std::collections::HashMap;

use crate::{buf::PutBuf, protocol};

use super::impl_packet_for;

/// Implementation for SSH_FXP_VERSION
#[derive(Debug, PartialEq, Eq)]
pub struct Version {
    pub version: u32,
    pub extensions: HashMap<String, String>,
}

impl Version {
    pub fn new() -> Self {
        Self {
            version: protocol::VERSION,
            extensions: HashMap::new(),
        }
    }
}

impl_packet_for!(Version, protocol::Response);

impl Default for Version {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Version> for Bytes {
    fn from(version: Version) -> Self {
        let mut bytes = BytesMut::new();
        bytes.put_u32(version.version);

        for (name, data) in &version.extensions {
            bytes.put_str(name);
            bytes.put_str(data);
        }

        bytes.freeze()
    }
}
