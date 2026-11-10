use serde::{Deserialize, Serialize};

use super::{impl_packet_for, impl_request_id, File, Packet, RequestId};

/// Implementation for `SSH_FXP_NAME`
#[derive(Debug, Serialize, Deserialize)]
pub struct Name {
    pub id: u32,
    pub files: Vec<File>,
}

impl_request_id!(Name);
impl_packet_for!(Name);

#[cfg(test)]
mod tests {
    use bytes::{BufMut, BytesMut};

    use crate::protocol::Packet;

    fn put_str(buf: &mut BytesMut, s: &str) {
        buf.put_u32(s.len() as u32);
        buf.put_slice(s.as_bytes());
    }

    #[test]
    fn extended_attrs_are_skipped() {
        let mut wire = BytesMut::new();
        wire.put_u8(104); // NAME
        wire.put_u32(1);
        wire.put_u32(2);

        put_str(&mut wire, "a");
        put_str(&mut wire, "");
        wire.put_u32(0x8000_0001); // EXTENDED | SIZE
        wire.put_u64(10);
        wire.put_u32(2);
        for s in ["k1", "v1", "k2", "v2"] {
            put_str(&mut wire, s);
        }

        put_str(&mut wire, "b");
        put_str(&mut wire, "");
        wire.put_u32(0x1); // SIZE
        wire.put_u64(20);

        let Packet::Name(name) = Packet::try_from(&mut wire.freeze()).unwrap() else {
            panic!("expected NAME");
        };
        let files: Vec<_> = name
            .files
            .iter()
            .map(|f| (f.filename.as_str(), f.attrs.size))
            .collect();
        assert_eq!(files, [("a", Some(10)), ("b", Some(20))]);
    }
}
