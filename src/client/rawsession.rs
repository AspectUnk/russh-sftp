use bytes::Bytes;
use std::{num::Wrapping, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::{mpsc, oneshot, Mutex},
    time::timeout,
};

use super::{error::Error, run, Handler};
use crate::{
    de,
    extensions::{self, FsyncExtension, LimitsExtension, Statvfs, StatvfsExtension},
    protocol::{
        Attrs, Close, Data, Extended, ExtendedReply, FSetStat, FileAttributes, Fstat, Handle, Init,
        Lstat, MkDir, Name, Open, OpenDir, OpenFlags, Packet, Read, ReadDir, ReadLink, RealPath,
        Remove, Rename, RmDir, SetStat, Stat, Status, StatusCode, Symlink, Version, Write,
    },
};

pub type SftpResult<T> = Result<T, Error>;
type SharedData = Mutex<Vec<(Option<u32>, oneshot::Sender<SftpResult<Packet>>)>>;

pub(crate) struct SessionInner {
    version: Option<u32>,
    requests: Arc<SharedData>,
}

impl SessionInner {
    pub async fn reply(&mut self, id: Option<u32>, packet: Packet) -> SftpResult<()> {
        let mut requests = self.requests.lock().await;

        if let Some(idx) = requests.iter().position(|&(i, _)| i == id) {
            let validate = if id.is_some() && self.version.is_none() {
                Err(Error::UnexpectedPacket)
            } else if id.is_none() && self.version.is_some() {
                Err(Error::UnexpectedBehavior("Duplicate version".to_owned()))
            } else {
                Ok(())
            };

            let _ = requests
                .remove(idx)
                .1
                .send(validate.clone().map(|()| packet));

            return validate;
        }

        Err(Error::UnexpectedBehavior(format!(
            "Packet {id:?} for unknown recipient"
        )))
    }
}

#[async_trait]
impl Handler for SessionInner {
    type Error = Error;

    async fn version(&mut self, packet: Version) -> Result<(), Self::Error> {
        let version = packet.version;
        self.reply(None, packet.into()).await?;
        self.version = Some(version);
        Ok(())
    }

    async fn name(&mut self, name: Name) -> Result<(), Self::Error> {
        self.reply(Some(name.id), name.into()).await
    }

    async fn status(&mut self, status: Status) -> Result<(), Self::Error> {
        self.reply(Some(status.id), status.into()).await
    }

    async fn handle(&mut self, handle: Handle) -> Result<(), Self::Error> {
        self.reply(Some(handle.id), handle.into()).await
    }

    async fn data(&mut self, data: Data) -> Result<(), Self::Error> {
        self.reply(Some(data.id), data.into()).await
    }

    async fn attrs(&mut self, attrs: Attrs) -> Result<(), Self::Error> {
        self.reply(Some(attrs.id), attrs.into()).await
    }

    async fn extended_reply(&mut self, reply: ExtendedReply) -> Result<(), Self::Error> {
        self.reply(Some(reply.id), reply.into()).await
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Limits {
    //pub packet_len: Option<u64>,
    pub read_len: Option<u64>,
    pub write_len: Option<u64>,
    pub open_handles: Option<u64>,
}

impl From<LimitsExtension> for Limits {
    fn from(limits: LimitsExtension) -> Self {
        Self {
            read_len: if limits.max_read_len > 0 {
                Some(limits.max_read_len)
            } else {
                None
            },
            write_len: if limits.max_write_len > 0 {
                Some(limits.max_write_len)
            } else {
                None
            },
            open_handles: if limits.max_open_handles > 0 {
                Some(limits.max_open_handles)
            } else {
                None
            },
        }
    }
}

pub(crate) struct Options {
    timeout: u64,
    limits: Arc<Limits>,
}

/// Implements raw work with the protocol in request-response format.
/// If the server returns a `Status` packet and it has the code Ok
/// then the packet is returned as Ok in other error cases
/// the packet is stored as Err.
pub struct RawSftpSession {
    tx: mpsc::UnboundedSender<Bytes>,
    requests: Arc<SharedData>,
    next_req_id: u32,
    handles: Wrapping<u64>,
    options: Options,
}

macro_rules! into_with_status {
    ($result:ident, $packet:ident) => {
        match $result {
            Packet::$packet(p) => Ok(p),
            Packet::Status(p) => Err(p.into()),
            _ => Err(Error::UnexpectedPacket),
        }
    };
}

macro_rules! into_status {
    ($result:ident) => {
        match $result {
            Packet::Status(status) if status.status_code == StatusCode::Ok => Ok(status),
            Packet::Status(status) => Err(status.into()),
            _ => Err(Error::UnexpectedPacket),
        }
    };
}

impl RawSftpSession {
    pub fn new<S>(stream: S) -> Self
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let arc = Arc::new(Mutex::new(Vec::new()));
        let inner = SessionInner {
            version: None,
            requests: arc.clone(),
        };

        Self {
            tx: run(stream, inner),
            requests: arc,
            next_req_id: 0,
            handles: Wrapping(0),
            options: Options {
                timeout: 10,
                limits: Arc::new(Limits::default()),
            },
        }
    }

    /// Set the maximum response time in seconds.
    /// Default: 10 seconds
    pub fn set_timeout(&mut self, secs: u64) {
        self.options.timeout = secs;
    }

    /// Setting limits. For the `limits@openssh.com` extension
    pub fn set_limits(&mut self, limits: Arc<Limits>) {
        self.options.limits = limits;
    }

    async fn send(&self, id: Option<u32>, packet: Packet) -> SftpResult<Packet> {
        let (tx, rx) = oneshot::channel();

        self.requests.lock().await.push((id, tx));
        self.tx.send(Bytes::try_from(packet)?)?;

        match timeout(Duration::from_secs(self.options.timeout), rx).await {
            Ok(result) => result?,
            Err(error) => {
                self.requests.lock().await.retain(|&(i, _)| i != id);
                Err(error.into())
            }
        }
    }

    fn use_next_id(&mut self) -> u32 {
        let id = self.next_req_id;
        self.next_req_id += 1;
        id
    }

    /// Closes the inner channel stream. Called by [`Drop`]
    ///
    /// # Errors
    ///
    /// TODO
    ///
    /// # Returns
    ///
    /// TODO
    pub fn close_session(&self) -> SftpResult<()> {
        if self.tx.is_closed() {
            return Ok(());
        }

        Ok(self.tx.send(Bytes::new())?)
    }

    /// Initialize the session. The server will return the version
    ///
    /// # Errors
    ///
    /// TODO
    ///
    /// # Returns
    ///
    /// TODO
    pub async fn init(&self) -> SftpResult<Version> {
        let result = self.send(None, Init::default().into()).await?;
        if let Packet::Version(version) = result {
            Ok(version)
        } else {
            Err(Error::UnexpectedPacket)
        }
    }

    /// Open a file on the server
    ///
    /// # Errors
    ///
    /// TODO
    ///
    /// # Returns
    ///
    /// TODO
    pub async fn open<T: Into<String>>(
        &mut self,
        filename: T,
        flags: OpenFlags,
        attrs: FileAttributes,
    ) -> SftpResult<Handle> {
        if self
            .options
            .limits
            .open_handles
            .is_some_and(|h| self.handles >= Wrapping(h))
        {
            return Err(Error::Limited("Handle limit reached".to_owned()));
        }

        let id = self.use_next_id();
        let result = self
            .send(
                Some(id),
                Open {
                    id,
                    filename: filename.into(),
                    pflags: flags,
                    attrs,
                }
                .into(),
            )
            .await?;

        if let Packet::Handle(_) = result {
            self.handles += 1;
        }

        into_with_status!(result, Handle)
    }

    /// Close a file on the server
    ///
    /// # Errors
    ///
    /// TODO
    ///
    /// # Returns
    ///
    /// TODO
    pub async fn close<H: Into<String>>(&mut self, handle: H) -> SftpResult<Status> {
        let id = self.use_next_id();
        let result = self
            .send(
                Some(id),
                Close {
                    id,
                    handle: handle.into(),
                }
                .into(),
            )
            .await?;

        if let Packet::Status(_) = &result {
            self.handles -= 1;
        }

        into_status!(result)
    }

    /// Read from a file on the server
    ///
    /// # Errors
    ///
    /// TODO
    ///
    /// # Returns
    ///
    /// TODO
    pub async fn read<H: Into<String>>(
        &mut self,
        handle: H,
        offset: u64,
        len: u32,
    ) -> SftpResult<Data> {
        if self.options.limits.read_len.is_some_and(|r| len as u64 > r) {
            return Err(Error::Limited("Write limit reached".to_owned()));
        }

        let id = self.use_next_id();
        let result = self
            .send(
                Some(id),
                Read {
                    id,
                    handle: handle.into(),
                    offset,
                    len,
                }
                .into(),
            )
            .await?;

        into_with_status!(result, Data)
    }

    /// Write to a file on the server
    ///
    /// # Errors
    ///
    /// TODO
    ///
    /// # Returns
    ///
    /// TODO
    pub async fn write<H: Into<String>>(
        &mut self,
        handle: H,
        offset: u64,
        data: Vec<u8>,
    ) -> SftpResult<Status> {
        if self
            .options
            .limits
            .write_len
            .is_some_and(|w| data.len() as u64 > w)
        {
            return Err(Error::Limited("Write limit reached".to_owned()));
        }

        let id = self.use_next_id();
        let result = self
            .send(
                Some(id),
                Write {
                    id,
                    handle: handle.into(),
                    offset,
                    data,
                }
                .into(),
            )
            .await?;

        into_status!(result)
    }

    /// Get `FSTAT` attributes
    ///
    /// # Errors
    ///
    /// TODO
    ///
    /// # Returns
    ///
    /// TODO
    pub async fn lstat<P: Into<String>>(&mut self, path: P) -> SftpResult<Attrs> {
        let id = self.use_next_id();
        let result = self
            .send(
                Some(id),
                Lstat {
                    id,
                    path: path.into(),
                }
                .into(),
            )
            .await?;

        into_with_status!(result, Attrs)
    }

    /// Get `LSTAT` attributes
    ///
    /// # Errors
    ///
    /// TODO
    ///
    /// # Returns
    ///
    /// TODO
    pub async fn fstat<H: Into<String>>(&mut self, handle: H) -> SftpResult<Attrs> {
        let id = self.use_next_id();
        let result = self
            .send(
                Some(id),
                Fstat {
                    id,
                    handle: handle.into(),
                }
                .into(),
            )
            .await?;

        into_with_status!(result, Attrs)
    }

    /// TODO
    ///
    /// # Errors
    ///
    /// TODO
    ///
    /// # Returns
    ///
    /// TODO
    pub async fn setstat<P: Into<String>>(
        &mut self,
        path: P,
        attrs: FileAttributes,
    ) -> SftpResult<Status> {
        let id = self.use_next_id();
        let result = self
            .send(
                Some(id),
                SetStat {
                    id,
                    path: path.into(),
                    attrs,
                }
                .into(),
            )
            .await?;

        into_status!(result)
    }

    /// TODO
    ///
    /// # Errors
    ///
    /// TODO
    ///
    /// # Returns
    ///
    /// TODO
    pub async fn fsetstat<H: Into<String>>(
        &mut self,
        handle: H,
        attrs: FileAttributes,
    ) -> SftpResult<Status> {
        let id = self.use_next_id();
        let result = self
            .send(
                Some(id),
                FSetStat {
                    id,
                    handle: handle.into(),
                    attrs,
                }
                .into(),
            )
            .await?;

        into_status!(result)
    }

    /// Open a directory on the server
    ///
    /// # Errors
    ///
    /// TODO
    ///
    /// # Returns
    ///
    /// TODO
    pub async fn opendir<P: Into<String>>(&mut self, path: P) -> SftpResult<Handle> {
        if self
            .options
            .limits
            .open_handles
            .is_some_and(|h| self.handles >= Wrapping(h))
        {
            return Err(Error::Limited("Handle limit reached".to_owned()));
        }

        let id = self.use_next_id();
        let result = self
            .send(
                Some(id),
                OpenDir {
                    id,
                    path: path.into(),
                }
                .into(),
            )
            .await?;

        if let Packet::Handle(_) = result {
            self.handles += 1;
        }

        into_with_status!(result, Handle)
    }

    /// Read a directory on the server
    ///
    /// # Errors
    ///
    /// TODO
    ///
    /// # Returns
    ///
    /// TODO
    pub async fn readdir<H: Into<String>>(&mut self, handle: H) -> SftpResult<Name> {
        let id = self.use_next_id();
        let result = self
            .send(
                Some(id),
                ReadDir {
                    id,
                    handle: handle.into(),
                }
                .into(),
            )
            .await?;

        into_with_status!(result, Name)
    }

    /// Remove a file on the server
    ///
    /// # Errors
    ///
    /// TODO
    ///
    /// # Returns
    ///
    /// TODO
    pub async fn remove<T: Into<String>>(&mut self, filename: T) -> SftpResult<Status> {
        let id = self.use_next_id();
        let result = self
            .send(
                Some(id),
                Remove {
                    id,
                    filename: filename.into(),
                }
                .into(),
            )
            .await?;

        into_status!(result)
    }

    /// Create a directory on the server
    ///
    /// # Errors
    ///
    /// TODO
    ///
    /// # Returns
    ///
    /// TODO
    pub async fn mkdir<P: Into<String>>(
        &mut self,
        path: P,
        attrs: FileAttributes,
    ) -> SftpResult<Status> {
        let id = self.use_next_id();
        let result = self
            .send(
                Some(id),
                MkDir {
                    id,
                    path: path.into(),
                    attrs,
                }
                .into(),
            )
            .await?;

        into_status!(result)
    }

    /// Remove a directory on the server
    ///
    /// # Errors
    ///
    /// TODO
    ///
    /// # Returns
    ///
    /// TODO
    pub async fn rmdir<P: Into<String>>(&mut self, path: P) -> SftpResult<Status> {
        let id = self.use_next_id();
        let result = self
            .send(
                Some(id),
                RmDir {
                    id,
                    path: path.into(),
                }
                .into(),
            )
            .await?;

        into_status!(result)
    }

    /// Get the real path of a file on the server
    ///
    /// # Errors
    ///
    /// TODO
    ///
    /// # Returns
    ///
    /// TODO
    pub async fn realpath<P: Into<String>>(&mut self, path: P) -> SftpResult<Name> {
        let id = self.use_next_id();
        let result = self
            .send(
                Some(id),
                RealPath {
                    id,
                    path: path.into(),
                }
                .into(),
            )
            .await?;

        into_with_status!(result, Name)
    }

    /// Get the `STAT` attributes
    ///
    /// # Errors
    ///
    /// TODO
    ///
    /// # Returns
    ///
    /// TODO
    pub async fn stat<P: Into<String>>(&mut self, path: P) -> SftpResult<Attrs> {
        let id = self.use_next_id();
        let result = self
            .send(
                Some(id),
                Stat {
                    id,
                    path: path.into(),
                }
                .into(),
            )
            .await?;

        into_with_status!(result, Attrs)
    }

    /// Rename a file on the server
    ///
    /// # Errors
    ///
    /// TODO
    ///
    /// # Returns
    ///
    /// TODO
    pub async fn rename<O, N>(&mut self, oldpath: O, newpath: N) -> SftpResult<Status>
    where
        O: Into<String>,
        N: Into<String>,
    {
        let id = self.use_next_id();
        let result = self
            .send(
                Some(id),
                Rename {
                    id,
                    oldpath: oldpath.into(),
                    newpath: newpath.into(),
                }
                .into(),
            )
            .await?;

        into_status!(result)
    }

    /// Get the target of a symbolic link on the server
    ///
    /// # Errors
    ///
    /// TODO
    ///
    /// # Returns
    ///
    /// TODO
    pub async fn readlink<P: Into<String>>(&mut self, path: P) -> SftpResult<Name> {
        let id = self.use_next_id();
        let result = self
            .send(
                Some(id),
                ReadLink {
                    id,
                    path: path.into(),
                }
                .into(),
            )
            .await?;

        into_with_status!(result, Name)
    }

    /// Create a symbolic link on the server
    ///
    /// # Errors
    ///
    /// TODO
    ///
    /// # Returns
    ///
    /// TODO
    pub async fn symlink<P, T>(&mut self, path: P, target: T) -> SftpResult<Status>
    where
        P: Into<String>,
        T: Into<String>,
    {
        let id = self.use_next_id();
        let result = self
            .send(
                Some(id),
                Symlink {
                    id,
                    linkpath: path.into(),
                    targetpath: target.into(),
                }
                .into(),
            )
            .await?;

        into_status!(result)
    }

    /// Equivalent to `SSH_FXP_EXTENDED`. Allows protocol expansion.
    /// The extension can return any packet, so it's not specific
    ///
    /// # Errors
    ///
    /// TODO
    ///
    /// # Returns
    ///
    /// TODO
    pub async fn extended<R: Into<String>>(
        &mut self,
        request: R,
        data: Vec<u8>,
    ) -> SftpResult<Packet> {
        let id = self.use_next_id();
        self.send(
            Some(id),
            Extended {
                id,
                request: request.into(),
                data,
            }
            .into(),
        )
        .await
    }

    /// Get the limits of the server
    ///
    /// # Errors
    ///
    /// TODO
    ///
    /// # Returns
    ///
    /// TODO
    pub async fn limits(&mut self) -> SftpResult<LimitsExtension> {
        match self.extended(extensions::LIMITS, vec![]).await? {
            Packet::ExtendedReply(reply) => {
                Ok(de::from_bytes::<LimitsExtension>(&mut reply.data.into())?)
            }
            Packet::Status(status) if status.status_code != StatusCode::Ok => {
                Err(Error::Status(status))
            }
            _ => Err(Error::UnexpectedPacket),
        }
    }

    /// Flushes the file system buffers on the server
    ///
    /// # Errors
    ///
    /// TODO
    ///
    /// # Returns
    ///
    /// TODO
    pub async fn fsync<H: Into<String>>(&mut self, handle: H) -> SftpResult<Status> {
        let result = self
            .extended(
                extensions::FSYNC,
                FsyncExtension {
                    handle: handle.into(),
                }
                .try_into()?,
            )
            .await?;

        into_status!(result)
    }

    /// Get the file system statistics
    ///
    /// # Errors
    ///
    /// TODO
    ///
    /// # Returns
    ///
    /// TODO
    pub async fn statvfs<P>(&mut self, path: P) -> SftpResult<Statvfs>
    where
        P: Into<String>,
    {
        let result = self
            .extended(
                extensions::STATVFS,
                StatvfsExtension { path: path.into() }.try_into()?,
            )
            .await?;

        match result {
            Packet::ExtendedReply(reply) => Ok(de::from_bytes::<Statvfs>(&mut reply.data.into())?),
            Packet::Status(status) if status.status_code != StatusCode::Ok => {
                Err(Error::Status(status))
            }
            _ => Err(Error::UnexpectedPacket),
        }
    }
}

impl Drop for RawSftpSession {
    fn drop(&mut self) {
        let _ = self.close_session();
    }
}
