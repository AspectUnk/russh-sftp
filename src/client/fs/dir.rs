use std::{collections::VecDeque, sync::Arc};

use super::Metadata;
use crate::protocol::{FileType, SftpString};

/// Entries returned by the [`ReadDir`] iterator.
#[derive(Debug)]
pub struct DirEntry {
    parent: Arc<SftpString>,
    file: SftpString,
    metadata: Metadata,
}

impl DirEntry {
    /// Returns the file name for the file that this entry points at.
    pub fn file_name(&self) -> SftpString {
        self.file.to_owned()
    }

    /// Returns the file type for the file that this entry points at.
    pub fn file_type(&self) -> FileType {
        self.metadata.file_type()
    }

    /// Returns the metadata for the file that this entry points at.
    pub fn metadata(&self) -> Metadata {
        self.metadata.to_owned()
    }

    /// Returns the full path of the file that this entry points at.
    ///
    /// The returned path is built by joining the path originally passed to
    /// [`SftpSession::read_dir`](crate::client::SftpSession::read_dir) with
    /// [`DirEntry::file_name`] using `/` as the separator (SFTP always uses
    /// POSIX-style paths on the wire). No canonicalization is performed, so a
    /// relative input yields a relative result — mirroring the behaviour of
    /// [`std::fs::DirEntry::path`].
    pub fn path(&self) -> SftpString {
        let mut path = SftpString::clone(&self.parent);
        let has_separator = path.is_empty() || path.as_bytes().ends_with(b"/");
        path.push_str(if has_separator { "" } else { "/" });
        path.push_str(&self.file);
        path
    }
}

/// Iterator over the entries in a remote directory.
pub struct ReadDir {
    pub(crate) parent: Arc<SftpString>,
    pub(crate) entries: VecDeque<(SftpString, Metadata)>,
}

impl Iterator for ReadDir {
    type Item = DirEntry;

    fn next(&mut self) -> Option<Self::Item> {
        match self.entries.pop_front() {
            None => None,
            Some(entry) if entry.0 == "." || entry.0 == ".." => self.next(),
            Some(entry) => Some(DirEntry {
                parent: self.parent.clone(),
                file: entry.0,
                metadata: entry.1,
            }),
        }
    }
}
