use std::collections::VecDeque;

use super::Metadata;

#[derive(Debug)]
pub struct DirEntry {
    file: String,
    metadata: Metadata,
}

impl DirEntry {
    pub fn file_name(&self) -> String {
        self.file.to_owned()
    }

    pub fn metadata(&self) -> Metadata {
        self.metadata.to_owned()
    }
}

pub struct ReadDir {
    entries: VecDeque<(String, Metadata)>,
}

impl ReadDir {
    pub(crate) fn new(entries: VecDeque<(String, Metadata)>) -> Self {
        Self { entries }
    }
}

impl Iterator for ReadDir {
    type Item = DirEntry;

    fn next(&mut self) -> Option<Self::Item> {
        match self.entries.pop_front() {
            None => None,
            Some(entry) if entry.0 == "." || entry.0 == ".." => self.next(),
            Some(entry) => Some(DirEntry {
                file: entry.0,
                metadata: entry.1,
            }),
        }
    }
}
