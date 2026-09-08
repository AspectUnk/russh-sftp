use bytes::Bytes;

use super::{impl_packet_for, impl_request_id, Packet, RequestId};
use crate::{error::Error, protocol::SSH_FXP_WRITE, ser};

/// Implementation for `SSH_FXP_WRITE`
#[derive(Debug, Serialize, Deserialize)]
pub struct Write {
    pub id: u32,
    pub handle: String,
    pub offset: u64,
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

impl_request_id!(Write);
impl_packet_for!(Write);

#[derive(Serialize)]
pub(crate) struct WriteRef<'a> {
    pub id: u32,
    pub handle: &'a str,
    pub offset: u64,
    #[serde(with = "serde_bytes")]
    pub data: &'a [u8],
}

impl TryFrom<WriteRef<'_>> for Bytes {
    type Error = Error;

    fn try_from(write: WriteRef<'_>) -> Result<Self, Self::Error> {
        ser::to_packet_bytes(SSH_FXP_WRITE, &write)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn borrowed_write_matches_owned_packet() {
        for size in [0, 1, 32739, 65536] {
            let data = vec![0x5a; size];
            let borrowed = Bytes::try_from(WriteRef {
                id: 42,
                handle: "test-handle",
                offset: 123_456,
                data: &data,
            })
            .unwrap();
            let owned = Bytes::try_from(Packet::Write(Write {
                id: 42,
                handle: "test-handle".to_owned(),
                offset: 123_456,
                data,
            }))
            .unwrap();
            assert_eq!(borrowed, owned);
        }
    }
}
