use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use super::{
    error::Error,
    fs::{File, Metadata, ReadDir},
    rawsession::{Limits, SftpResult},
    RawSftpSession,
};
use crate::{
    client::Config,
    extensions::{self, Statvfs},
    protocol::{FileAttributes, OpenFlags, Status, StatusCode},
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Features {
    pub hardlink: bool,
    pub posix_rename: bool,
    pub fsync: bool,
    pub statvfs: bool,
    pub expand_path: bool,
    pub limits: Option<Limits>,
    pub max_concurrent_reads: usize,
    pub max_concurrent_writes: usize,
    pub max_write_packet_len: u32,
    pub max_packet_len: u32,
}

/// High-level SFTP implementation for easy interaction with a remote file system.
/// Contains most methods similar to the native [filesystem](std::fs)
pub struct SftpSession {
    session: Arc<RawSftpSession>,
    features: Features,
}

impl SftpSession {
    /// Creates a new session by initializing the protocol and extensions
    pub async fn new<S>(stream: S) -> SftpResult<Self>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        Self::new_with_config(stream, Config::default()).await
    }

    /// Creates a new session with custom configuration
    pub async fn new_with_config<S>(stream: S, cfg: Config) -> SftpResult<Self>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let max_concurrent_reads = cfg.max_concurrent_reads.max(1);
        let max_concurrent_writes = cfg.max_concurrent_writes.max(1);
        let max_write_packet_len = cfg.max_write_packet_len;
        let max_packet_len = cfg.max_packet_len;
        let mut session = RawSftpSession::new_with_config(stream, cfg);

        let version = session.init().await?;
        let has_extension = |name, ver| version.extensions.get(name).is_some_and(|v| v == ver);

        let mut features = Features {
            hardlink: has_extension(extensions::HARDLINK, "1"),
            posix_rename: has_extension(extensions::POSIX_RENAME, "1"),
            fsync: has_extension(extensions::FSYNC, "1"),
            statvfs: has_extension(extensions::STATVFS, "2"),
            expand_path: has_extension(extensions::EXPAND_PATH, "1"),
            limits: None,
            max_concurrent_reads,
            max_concurrent_writes,
            max_write_packet_len,
            max_packet_len,
        };

        if has_extension(extensions::LIMITS, "1") {
            let limits = Limits::from(session.limits().await?);
            session.set_limits(limits);
            features.limits = Some(limits);
            if let Some(plen) = limits.packet_len {
                features.max_packet_len = (plen as u32).min(max_packet_len);
            }
        }

        Ok(Self {
            session: Arc::new(session),
            features,
        })
    }

    /// Set the maximum response time in seconds.
    /// Default: 10 seconds
    pub fn set_timeout(&self, secs: u64) {
        self.session.set_timeout(secs);
    }

    /// Closes the inner channel stream.
    pub async fn close(&self) -> SftpResult<()> {
        self.session.close_session()
    }

    /// Attempts to open a file in read-only mode.
    pub async fn open<T: Into<String>>(&self, filename: T) -> SftpResult<File> {
        self.open_with_flags(filename, OpenFlags::READ).await
    }

    /// Opens a file in write-only mode.
    ///
    /// This function will create a file if it does not exist, and will truncate it if it does.
    pub async fn create<T: Into<String>>(&self, filename: T) -> SftpResult<File> {
        self.open_with_flags(
            filename,
            OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE,
        )
        .await
    }

    /// Attempts to open or create the file in the specified mode
    pub async fn open_with_flags<T: Into<String>>(
        &self,
        filename: T,
        flags: OpenFlags,
    ) -> SftpResult<File> {
        self.open_with_flags_and_attributes(filename, flags, FileAttributes::empty())
            .await
    }

    /// Attempts to open or create the file in the specified mode and with specified file attributes
    pub async fn open_with_flags_and_attributes<T: Into<String>>(
        &self,
        filename: T,
        flags: OpenFlags,
        attributes: FileAttributes,
    ) -> SftpResult<File> {
        let handle = self.session.open(filename, flags, attributes).await?.handle;
        Ok(File::new(self.session.clone(), handle, self.features))
    }

    /// Requests the remote party for the absolute from the relative path.
    pub async fn canonicalize<T: Into<String>>(&self, path: T) -> SftpResult<String> {
        let name = self.session.realpath(path).await?;
        match name.files.first() {
            Some(file) => Ok(file.filename.to_owned()),
            None => Err(Error::UnexpectedBehavior("no file".to_owned())),
        }
    }

    /// Creates a new empty directory.
    pub async fn create_dir<T: Into<String>>(&self, path: T) -> SftpResult<()> {
        self.session
            .mkdir(path, FileAttributes::empty())
            .await
            .map(|_| ())
    }

    /// Reads the contents of a file located at the specified path to the end.
    pub async fn read<P: Into<String>>(&self, path: P) -> SftpResult<Vec<u8>> {
        let mut file = self.open(path).await?;
        let mut buffer = Vec::new();

        file.read_to_end(&mut buffer).await?;
        file.close().await?;

        Ok(buffer)
    }

    /// Writes the contents to a file whose path is specified.
    pub async fn write<P: Into<String>>(&self, path: P, data: &[u8]) -> SftpResult<()> {
        let mut file = self.open_with_flags(path, OpenFlags::WRITE).await?;
        file.write_all(data).await?;
        file.close().await?;
        Ok(())
    }

    /// Checks a file or folder exists at the specified path
    pub async fn try_exists<P: Into<String>>(&self, path: P) -> SftpResult<bool> {
        match self.metadata(path).await {
            Ok(_) => Ok(true),
            Err(Error::Status(status)) if status.status_code == StatusCode::NoSuchFile => Ok(false),
            Err(error) => Err(error),
        }
    }

    /// Returns an iterator over the entries within a directory.
    pub async fn read_dir<P: Into<String>>(&self, path: P) -> SftpResult<ReadDir> {
        let path: String = path.into();
        let parent = Arc::from(path.as_str());

        let handle = self.session.opendir(path).await?.handle;
        let mut files = vec![];

        loop {
            match self.session.readdir(handle.as_str()).await {
                Ok(name) => {
                    files = name
                        .files
                        .into_iter()
                        .map(|f| (f.filename, f.attrs))
                        .chain(files)
                        .collect();
                }
                Err(Error::Status(status)) if status.status_code == StatusCode::Eof => break,
                Err(err) => return Err(err),
            }
        }

        self.session.close(handle).await?;

        Ok(ReadDir {
            parent,
            entries: files.into(),
        })
    }

    /// Reads a symbolic link, returning the file that the link points to.
    pub async fn read_link<P: Into<String>>(&self, path: P) -> SftpResult<String> {
        let name = self.session.readlink(path).await?;
        match name.files.first() {
            Some(file) => Ok(file.filename.to_owned()),
            None => Err(Error::UnexpectedBehavior("no file".to_owned())),
        }
    }

    /// Removes the specified folder.
    pub async fn remove_dir<P: Into<String>>(&self, path: P) -> SftpResult<()> {
        self.session.rmdir(path).await.map(|_| ())
    }

    /// Removes the specified file.
    pub async fn remove_file<T: Into<String>>(&self, filename: T) -> SftpResult<()> {
        self.session.remove(filename).await.map(|_| ())
    }

    /// Rename a file or directory to a new name.
    pub async fn rename<O, N>(&self, oldpath: O, newpath: N) -> SftpResult<()>
    where
        O: Into<String>,
        N: Into<String>,
    {
        self.session.rename(oldpath, newpath).await.map(|_| ())
    }

    /// Creates a symlink of the specified target.
    pub async fn symlink<P, T>(&self, path: P, target: T) -> SftpResult<()>
    where
        P: Into<String>,
        T: Into<String>,
    {
        self.session.symlink(path, target).await.map(|_| ())
    }

    /// Queries metadata about the remote file.
    pub async fn metadata<P: Into<String>>(&self, path: P) -> SftpResult<Metadata> {
        Ok(self.session.stat(path).await?.attrs)
    }

    /// Sets metadata for a remote file.
    pub async fn set_metadata<P: Into<String>>(
        &self,
        path: P,
        metadata: Metadata,
    ) -> Result<(), Error> {
        self.session.setstat(path, metadata).await.map(|_| ())
    }

    pub async fn symlink_metadata<P: Into<String>>(&self, path: P) -> SftpResult<Metadata> {
        Ok(self.session.lstat(path).await?.attrs)
    }

    pub async fn hardlink<O, N>(&self, oldpath: O, newpath: N) -> SftpResult<bool>
    where
        O: Into<String>,
        N: Into<String>,
    {
        if !self.features.hardlink {
            return Ok(false);
        }

        self.session.hardlink(oldpath, newpath).await.map(|_| true)
    }

    /// Whether the server advertises `posix-rename@openssh.com` version 1.
    pub fn supports_posix_rename(&self) -> bool {
        self.features.posix_rename
    }

    /// Renames `oldpath` to `newpath`, atomically replacing `newpath` if it exists.
    ///
    /// Unlike [`SftpSession::rename`], which on OpenSSH fails when the target exists,
    /// this uses `posix-rename@openssh.com` and has POSIX `rename(2)` semantics.
    /// Returns [`Error::Status`] with [`StatusCode::OpUnsupported`] without sending
    /// anything if the server does not advertise the extension; check
    /// [`SftpSession::supports_posix_rename`].
    pub async fn posix_rename<O, N>(&self, oldpath: O, newpath: N) -> SftpResult<()>
    where
        O: Into<String>,
        N: Into<String>,
    {
        if !self.features.posix_rename {
            return Err(Error::Status(Status {
                id: 0,
                status_code: StatusCode::OpUnsupported,
                error_message: "server does not support posix-rename@openssh.com".into(),
                language_tag: "en-US".into(),
            }));
        }

        self.session
            .posix_rename(oldpath, newpath)
            .await
            .map(|_| ())
    }

    /// Performs a statvfs on the remote file system path.
    /// Returns `Ok(None)` if the remote SFTP server does not support `statvfs@openssh.com` extension v2.
    pub async fn fs_info<P: Into<String>>(&self, path: P) -> SftpResult<Option<Statvfs>> {
        if !self.features.statvfs {
            return Ok(None);
        }

        self.session.statvfs(path).await.map(Some)
    }

    /// Expands a `~`/`~user`-prefixed or relative path and returns its canonicalized absolute form.
    /// Returns `Ok(None)` if the remote SFTP server does not support `expand-path@openssh.com` extension v1.
    pub async fn expand_path<P: Into<String>>(&self, path: P) -> SftpResult<Option<String>> {
        if !self.features.expand_path {
            return Ok(None);
        }

        let name = self.session.expand_path(path).await?;
        match name.files.first() {
            Some(file) => Ok(Some(file.filename.to_owned())),
            None => Err(Error::UnexpectedBehavior("no file".to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use bytes::{Buf, Bytes};
    use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

    use super::*;
    use crate::{
        extensions::PosixRenameExtension,
        protocol::{Extended, Handle, Packet, Version},
    };

    #[derive(Clone, Copy)]
    enum Reply {
        Status(StatusCode),
        Handle,
    }

    async fn read_packet(stream: &mut DuplexStream) -> Option<Bytes> {
        let len = stream.read_u32().await.ok()?;
        let mut buf = vec![0; len as usize];
        stream.read_exact(&mut buf).await.ok()?;
        Some(Bytes::from(buf))
    }

    async fn write_packet(stream: &mut DuplexStream, packet: Packet) {
        let bytes = Bytes::try_from(packet).unwrap();
        stream.write_all(&bytes).await.unwrap();
    }

    /// Minimal server: answers INIT with the given extensions and every EXTENDED
    /// request with `reply`. Returns the raw bodies of all EXTENDED packets received.
    fn spawn_server(
        mut stream: DuplexStream,
        extensions: &[(&str, &str)],
        reply: Reply,
    ) -> tokio::task::JoinHandle<Vec<Bytes>> {
        let extensions: HashMap<String, String> = extensions
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        tokio::spawn(async move {
            let mut extended = Vec::new();
            while let Some(raw) = read_packet(&mut stream).await {
                match Packet::try_from(&mut raw.clone()).unwrap() {
                    Packet::Init(_) => {
                        let version = Version {
                            extensions: extensions.clone(),
                            ..Version::new()
                        };
                        write_packet(&mut stream, version.into()).await;
                    }
                    Packet::Extended(Extended { id, .. }) => {
                        extended.push(raw);
                        let packet = match reply {
                            Reply::Status(status_code) => Status {
                                id,
                                status_code,
                                error_message: String::new(),
                                language_tag: "en-US".into(),
                            }
                            .into(),
                            Reply::Handle => Handle {
                                id,
                                handle: "h".into(),
                            }
                            .into(),
                        };
                        write_packet(&mut stream, packet).await;
                    }
                    other => panic!("unexpected packet {other:?}"),
                }
            }
            extended
        })
    }

    async fn session_with(
        extensions: &[(&str, &str)],
        reply: Reply,
    ) -> (SftpSession, tokio::task::JoinHandle<Vec<Bytes>>) {
        let (client, server) = tokio::io::duplex(64 * 1024);
        let server = spawn_server(server, extensions, reply);
        (SftpSession::new(client).await.unwrap(), server)
    }

    /// Waits for the server task, failing instead of hanging if the stream stays open.
    async fn finish(server: tokio::task::JoinHandle<Vec<Bytes>>) -> Vec<Bytes> {
        tokio::time::timeout(std::time::Duration::from_secs(5), server)
            .await
            .expect("server did not shut down")
            .unwrap()
    }

    const ADVERTISED: &[(&str, &str)] = &[(extensions::POSIX_RENAME, "1")];

    fn sftp_string(s: &str) -> Vec<u8> {
        let mut out = (s.len() as u32).to_be_bytes().to_vec();
        out.extend_from_slice(s.as_bytes());
        out
    }

    #[test]
    fn posix_rename_extension_wire_encoding() {
        let data: Vec<u8> = PosixRenameExtension {
            oldpath: "a/old".into(),
            newpath: "new".into(),
        }
        .try_into()
        .unwrap();
        assert_eq!(data, [sftp_string("a/old"), sftp_string("new")].concat());

        let packet = Bytes::try_from(Packet::Extended(Extended {
            id: 7,
            request: extensions::POSIX_RENAME.into(),
            data: data.clone(),
        }))
        .unwrap();
        let body = [
            vec![200],
            7u32.to_be_bytes().to_vec(),
            sftp_string("posix-rename@openssh.com"),
            data,
        ]
        .concat();
        let expected = [(body.len() as u32).to_be_bytes().to_vec(), body].concat();
        assert_eq!(packet.as_ref(), expected.as_slice());
    }

    #[tokio::test]
    async fn posix_rename_sends_extended_request() {
        let (sftp, server) = session_with(ADVERTISED, Reply::Status(StatusCode::Ok)).await;
        assert!(sftp.supports_posix_rename());
        sftp.posix_rename("from", "to").await.unwrap();
        drop(sftp);

        let sent = finish(server).await;
        assert_eq!(sent.len(), 1);
        let mut body = sent[0].clone();
        assert_eq!(body.get_u8(), 200);
        let _id = body.get_u32();
        assert_eq!(
            body.as_ref(),
            [
                sftp_string("posix-rename@openssh.com"),
                sftp_string("from"),
                sftp_string("to")
            ]
            .concat()
            .as_slice()
        );
    }

    #[tokio::test]
    async fn posix_rename_maps_error_statuses() {
        for code in [StatusCode::Failure, StatusCode::OpUnsupported] {
            let (sftp, _server) = session_with(ADVERTISED, Reply::Status(code)).await;
            match sftp.posix_rename("from", "to").await {
                Err(Error::Status(status)) => assert_eq!(status.status_code, code),
                other => panic!("expected status error for {code:?}, got {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn posix_rename_rejects_non_status_reply() {
        let (sftp, _server) = session_with(ADVERTISED, Reply::Handle).await;
        assert!(matches!(
            sftp.posix_rename("from", "to").await,
            Err(Error::UnexpectedPacket)
        ));
    }

    #[tokio::test]
    async fn posix_rename_unadvertised_sends_nothing() {
        for extensions in [&[][..], &[(extensions::POSIX_RENAME, "2")][..]] {
            let (sftp, server) = session_with(extensions, Reply::Status(StatusCode::Ok)).await;
            assert!(!sftp.supports_posix_rename());
            match sftp.posix_rename("from", "to").await {
                Err(Error::Status(status)) => {
                    assert_eq!(status.status_code, StatusCode::OpUnsupported)
                }
                other => panic!("expected OpUnsupported, got {other:?}"),
            }
            drop(sftp);
            assert!(finish(server).await.is_empty());
        }
    }
}
