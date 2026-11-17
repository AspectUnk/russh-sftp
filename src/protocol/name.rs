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
    use super::*;
    use crate::{de, ser};

    #[test]
    fn extended_attrs_are_skipped() {
        // SIZE | EXTENDED with one pair, then SIZE
        let mut bytes = ser::to_bytes(&(
            (1_u32, 2_u32),
            ("a", "", 0x8000_0001_u32, 10_u64, 1_u32, "k", "v"),
            ("b", "", 1_u32, 20_u64),
        ))
        .unwrap();
        let name: Name = de::from_bytes(&mut bytes).unwrap();

        let sizes: Vec<_> = name.files.iter().map(|f| f.attrs.size).collect();
        assert_eq!(sizes, [Some(10), Some(20)]);
    }
}
