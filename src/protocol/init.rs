use bytes::{Buf, Bytes};
use std::collections::HashMap;

use crate::{buf::TryBuf, error};

#[derive(Debug, PartialEq, Eq)]
pub struct Init {
    pub version: u32,
    pub extensions: HashMap<String, String>,
}

impl TryFrom<&mut Bytes> for Init {
    type Error = error::Error;

    fn try_from(bytes: &mut Bytes) -> Result<Self, Self::Error> {
        let version = bytes.try_get_u32()?;
        let mut extensions = HashMap::new();

        while bytes.has_remaining() {
            let name = bytes.try_get_string()?;
            let data = bytes.try_get_string()?;

            extensions.insert(name, data);
        }

        Ok(Self {
            version,
            extensions,
        })
    }
}
