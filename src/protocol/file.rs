use jiff::Timestamp;
use std::time::Duration;

use super::{FileAttributes, SftpString};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    pub filename: SftpString,
    pub longname: String,
    pub attrs: FileAttributes,
}

impl File {
    /// Omits `longname` and set dummy `attributes`. This is mainly used for [`crate::server::Handler::realpath`] as per the standard
    pub fn dummy<S: Into<SftpString>>(filename: S) -> Self {
        Self {
            filename: filename.into(),
            longname: "".to_string(),
            attrs: FileAttributes::dummy(),
        }
    }

    /// Implies the use of longname
    pub fn new<S: Into<SftpString>>(filename: S, attrs: FileAttributes) -> Self {
        let mut file = Self {
            filename: filename.into(),
            longname: "".to_string(),
            attrs,
        };
        file.longname = file.longname();
        file
    }

    /// Get formed longname
    pub fn longname(&self) -> String {
        const SIX_MONTHS: i64 = 15_778_476;

        let directory = if self.attrs.is_dir() { "d" } else { "-" };
        let permissions = self.attrs.permissions().to_string();

        let size = self.attrs.size.unwrap_or(0);
        let mtime = self.attrs.mtime.unwrap_or(0);

        let datetime = Timestamp::UNIX_EPOCH + Duration::from_secs(mtime as u64);
        let now = Timestamp::now().as_second();
        let age = now - (mtime as i64);
        let delayed = if !(0..=SIX_MONTHS).contains(&age) {
            datetime.strftime("%b %e  %Y").to_string()
        } else {
            datetime.strftime("%b %e %H:%M").to_string()
        };

        format!(
            "{directory}{permissions} 0 {} {} {size} {delayed} {}",
            if let Some(user) = &self.attrs.user {
                user.to_string()
            } else {
                self.attrs.uid.unwrap_or(0).to_string()
            },
            if let Some(group) = &self.attrs.group {
                group.to_string()
            } else {
                self.attrs.gid.unwrap_or(0).to_string()
            },
            self.filename
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn longname_formats_utc_date() {
        let file = File::new("foo", FileAttributes::dummy());
        assert_eq!(file.longname, "drwxrwxrwx 0 0 0 0 Jan  1  1970 foo");
    }
}
