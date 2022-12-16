use bytes::{BufMut, Bytes, BytesMut};

use crate::{file::FileAttributes, protocol};

use super::impl_packet_for;

/// Implementation for SSH_FXP_ATTRS
#[derive(Debug)]
pub struct Attrs {
    pub id: u32,
    pub attrs: FileAttributes,
}

impl_packet_for!(Attrs, protocol::Response);

impl From<Attrs> for Bytes {
    fn from(attrs: Attrs) -> Self {
        let mut bytes = BytesMut::new();
        bytes.put_u32(attrs.id);
        bytes.put_slice(&Bytes::from(&attrs.attrs));
        bytes.freeze()
    }
}
