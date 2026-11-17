use std::{borrow::Cow, fmt, ops::Deref, str::Utf8Error};

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

    /// Returns a string slice if the bytes are valid UTF-8
    pub fn to_str(&self) -> Result<&str, Utf8Error> {
        std::str::from_utf8(&self.0)
    }

    /// Converts to a string, replacing invalid UTF-8 bytes
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.0)
    }

    /// Converts into a string if the bytes are valid UTF-8, otherwise returns itself
    pub fn into_string(self) -> Result<String, Self> {
        String::from_utf8(self.0).map_err(|err| Self(err.into_bytes()))
    }

    /// Appends a string or bytes to the end
    pub fn push_str<T: AsRef<[u8]>>(&mut self, string: T) {
        self.0.extend_from_slice(string.as_ref());
    }

    /// Returns a path joined with `/`, like [`std::path::Path::join`] without absolute paths handling
    pub fn join<T: AsRef<[u8]>>(&self, path: T) -> Self {
        let mut joined = self.clone();
        let has_separator = joined.is_empty() || joined.ends_with(b"/");
        joined.push_str(if has_separator { "" } else { "/" });
        joined.push_str(path);
        joined
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
        fmt::Display::fmt(&self.to_string_lossy(), f)
    }
}

impl fmt::Debug for SftpString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("\"")?;
        for chunk in self.0.utf8_chunks() {
            let (valid, invalid) = (chunk.valid(), chunk.invalid());
            write!(f, "{}{}", valid.escape_debug(), invalid.escape_ascii())?;
        }
        f.write_str("\"")
    }
}

/// Lossless conversions with local paths, only Unix keeps them as bytes
#[cfg(unix)]
mod unix {
    use std::{
        ffi::{OsStr, OsString},
        os::unix::ffi::{OsStrExt, OsStringExt},
        path::{Path, PathBuf},
    };

    use super::SftpString;

    impl AsRef<OsStr> for SftpString {
        fn as_ref(&self) -> &OsStr {
            OsStr::from_bytes(&self.0)
        }
    }

    impl AsRef<Path> for SftpString {
        fn as_ref(&self) -> &Path {
            Path::new(OsStr::from_bytes(&self.0))
        }
    }

    impl From<OsString> for SftpString {
        fn from(value: OsString) -> Self {
            Self(value.into_vec())
        }
    }

    impl From<PathBuf> for SftpString {
        fn from(value: PathBuf) -> Self {
            Self(value.into_os_string().into_vec())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_utf8_is_kept() {
        let string = SftpString::from("dir").join(b"\xbf");
        assert_eq!(string.as_bytes(), b"dir/\xbf");
        assert_eq!(string.to_string_lossy(), "dir/\u{fffd}");
        assert_eq!(format!("{string:?}"), r#""dir/\xbf""#);
        assert!(string.to_str().is_err());
        assert_eq!(string.clone().into_string(), Err(string));
    }

    #[cfg(unix)]
    #[test]
    fn local_path_is_lossless() {
        let path = std::path::PathBuf::from(&SftpString::from(b"\xbf"));
        assert_eq!(SftpString::from(path).as_bytes(), b"\xbf");
    }
}
