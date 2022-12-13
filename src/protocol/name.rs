use std::time::{Duration, UNIX_EPOCH};

use bytes::{BufMut, Bytes, BytesMut};
use chrono::{DateTime, Utc};

use super::impl_packet_for;
use crate::{buf::PutBuf, file::FileAttributes, server};

#[derive(Debug)]
pub struct Name {
    pub id: u32,
    pub files: Vec<File>,
}

impl_packet_for!(Name, server::Packet);

impl From<Name> for Bytes {
    fn from(name: Name) -> Self {
        let mut bytes = BytesMut::new();

        bytes.put_u32(name.id);
        bytes.put_u32(name.files.len() as u32);

        for file in &name.files {
            let file = Bytes::from(file);
            bytes.put_slice(&file);
        }

        bytes.freeze()
    }
}

#[derive(Debug)]
pub struct File {
    pub filename: String,
    pub attrs: FileAttributes,
}

impl File {
    pub fn longname(&self) -> String {
        let date = UNIX_EPOCH + Duration::from_secs(0);
        let datetime = DateTime::<Utc>::from(date);
        let delayed = datetime.format("%b %d %Y %H:%M");

        format!(
            "drwxrwxrwx 0 container container 0 {} {}",
            delayed, self.filename
        )
    }
}

impl From<&File> for Bytes {
    fn from(file: &File) -> Self {
        let mut bytes = BytesMut::new();

        bytes.put_str(&file.filename);
        bytes.put_str(&file.longname());

        let attrs = Bytes::from(&file.attrs);
        bytes.put_slice(&attrs);

        bytes.freeze()
    }
}
