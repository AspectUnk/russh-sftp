use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::collections::HashMap;

use crate::{
    buf::{PutBuf, TryBuf},
    error, protocol,
};

use super::impl_packet_for;

pub type Version = Init;

#[derive(Debug, PartialEq, Eq)]
pub struct Init {
    pub version: u32,
    pub extensions: HashMap<String, String>,
}

impl Init {
    pub fn new() -> Self {
        Self {
            version: protocol::VERSION,
            extensions: HashMap::new(),
        }
    }
}

impl_packet_for!(Version, protocol::Response);

impl Default for Init {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Init> for Bytes {
    fn from(packet: Init) -> Self {
        let mut bytes = BytesMut::new();
        bytes.put_u32(packet.version);

        for (name, data) in &packet.extensions {
            bytes.put_str(name);
            bytes.put_str(data);
        }

        bytes.freeze()
    }
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

#[cfg(test)]
mod test_init_packet {
    use bytes::Bytes;

    use super::*;

    #[test]
    fn test_bytes_from_init() {
        let mut bytes: Bytes = Init {
            version: 3,
            extensions: HashMap::from([
                ("test1".into(), "data".into()),
                ("test2".into(), "".into()),
            ]),
        }
        .into();

        assert_eq!(bytes.get_u32(), 3);
        assert_eq!(bytes.get_u32(), 5);
    }

    #[test]
    fn test_init_from_bytes() {
        let bytes = &[0x00, 0x00, 0x00, 0x03];
        let mut bytes = Bytes::from(&bytes[..]);

        let packet = Init::try_from(&mut bytes).unwrap();
        assert_eq!(packet, Init::new())
    }
}
