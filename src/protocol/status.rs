use bytes::{BufMut, Bytes, BytesMut};

use super::impl_packet_for;
use crate::{
    buf::{PutBuf, TryBuf},
    error,
    protocol::StatusCode,
    server,
};

#[derive(Debug, PartialEq, Eq)]
pub struct Status {
    pub id: u32,
    pub status_code: StatusCode,
    pub error_message: String,
    pub language_tag: String,
}

impl Status {
    pub fn new(id: u32, status: StatusCode, msg: &str) -> Self {
        Self {
            id: id,
            status_code: status,
            error_message: msg.to_string(),
            language_tag: "en-US".to_string(),
        }
    }
}

impl From<StatusCode> for Status {
    fn from(status: StatusCode) -> Self {
        Self::new(0, status, &status.to_string())
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

impl TryFrom<&mut Bytes> for Status {
    type Error = error::Error;

    fn try_from(bytes: &mut Bytes) -> Result<Self, Self::Error> {
        Ok(Self {
            id: bytes.try_get_u32()?,
            status_code: StatusCode::from(bytes.try_get_u32()?),
            error_message: bytes.try_get_string()?,
            language_tag: bytes.try_get_string()?,
        })
    }
}
