use bytes::{BufMut, Bytes, BytesMut};

use super::impl_packet_for;
use crate::{buf::PutBuf, server, ErrorProtocol};

#[derive(Debug, PartialEq, Eq)]
pub struct Status {
    pub id: u32,
    pub status_code: ErrorProtocol,
    pub error_message: String,
    pub language_tag: String,
}

impl Status {
    pub fn new(id: u32, error: ErrorProtocol, msg: &str) -> Self {
        Self {
            id: id,
            status_code: error,
            error_message: msg.to_string(),
            language_tag: "en-US".to_string(),
        }
    }
}

impl_packet_for!(Status, server::Packet);

impl From<Status> for Bytes {
    fn from(status: Status) -> Self {
        let mut bytes = BytesMut::new();
        
        bytes.put_u32(status.id);
        bytes.put_u32(status.status_code as u32);
        bytes.put_str(&status.error_message);
        bytes.put_str(&status.language_tag);

        bytes.freeze()
    }
}
