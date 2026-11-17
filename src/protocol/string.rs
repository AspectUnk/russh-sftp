use std::{fmt, ops::Deref};

/// String of paths and handles that keeps the bytes as is,
/// since the remote side is not obliged to use UTF-8.
/// Displayed lossy, the original is available via [`SftpString::as_bytes`]
#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SftpString(#[serde(with = "serde_bytes")] Vec<u8>);

impl SftpString {
    /// Returns the original bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Converts into the original bytes
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    /// Appends a string or bytes to the end
    pub fn push_str<T: AsRef<[u8]>>(&mut self, string: T) {
        self.0.extend_from_slice(string.as_ref());
    }
}

impl Deref for SftpString {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<[u8]> for SftpString {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl<T: AsRef<[u8]> + ?Sized> From<&T> for SftpString {
    fn from(value: &T) -> Self {
        Self(value.as_ref().to_vec())
    }
}

impl From<String> for SftpString {
    fn from(value: String) -> Self {
        Self(value.into_bytes())
    }
}

impl From<Vec<u8>> for SftpString {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

impl PartialEq<str> for SftpString {
    fn eq(&self, other: &str) -> bool {
        self.0 == other.as_bytes()
    }
}

impl PartialEq<&str> for SftpString {
    fn eq(&self, other: &&str) -> bool {
        self.0 == other.as_bytes()
    }
}

impl fmt::Display for SftpString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&String::from_utf8_lossy(&self.0), f)
    }
}

impl fmt::Debug for SftpString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&String::from_utf8_lossy(&self.0), f)
    }
}
