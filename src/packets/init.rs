use bytes::{BufMut, Bytes, BytesMut};

use crate::{buf::TryBuf, Error, PROTOCOL_VERSION};

pub type Version = Init;

#[derive(Debug, PartialEq, Eq)]
pub struct Init {
    pub version: u32,
    // ...extension data
}

impl Init {
    pub fn new() -> Self {
        Self {
            version: PROTOCOL_VERSION,
        }
    }
}

impl From<&Init> for Bytes {
    fn from(packet: &Init) -> Self {
        let mut bytes = BytesMut::new();
        bytes.put_u32(packet.version);
        bytes.freeze()
    }
}

impl TryFrom<&mut Bytes> for Init {
    type Error = Error;

    fn try_from(bytes: &mut Bytes) -> Result<Self, Self::Error> {
        Ok(Self {
            version: bytes.try_get_u32()?,
        })
    }
}

#[cfg(test)]
mod test_init_packet {
    use bytes::Bytes;

    use super::*;

    #[test]
    fn test_bytes_from_init() {
        let packet = &Init { version: 3 };
        let bytes: Bytes = packet.into();
        assert_eq!(&bytes.to_vec(), &[0x00, 0x00, 0x00, 0x03]);
    }

    #[test]
    fn test_init_from_bytes() {
        let bytes = &[0x00, 0x00, 0x00, 0x03];
        let mut bytes = Bytes::from(&bytes[..]);

        let packet = Init::try_from(&mut bytes).unwrap();
        assert_eq!(packet, Init { version: 3 })
    }
}
