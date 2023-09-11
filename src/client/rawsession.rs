use std::{sync::Arc, time::Duration};

use bytes::Bytes;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::{mpsc, oneshot, Mutex},
    time::timeout,
};

use crate::protocol::{
    Attrs, Close, Data, Extended, ExtendedReply, FSetStat, FileAttributes, Fstat, Handle, Init,
    Lstat, MkDir, Name, Open, OpenDir, OpenFlags, Packet, Read, ReadDir, ReadLink, RealPath,
    Remove, Rename, RmDir, SetStat, Stat, Status, StatusCode, Symlink, Version, Write,
};

use super::{error::Error, run, Handler};

pub(crate) type SharedData = Mutex<Vec<(Option<u32>, oneshot::Sender<Result<Packet, Error>>)>>;

pub(crate) struct SessionInner {
    version: Option<u32>,
    requests: Arc<SharedData>,
}

impl SessionInner {
    pub async fn reply(&mut self, id: Option<u32>, packet: Packet) -> Result<(), Error> {
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
                .send(validate.clone().map(|_| packet));

            return validate;
        }

        Err(Error::UnexpectedBehavior(format!(
            "Packet {:?} for unknown recipient",
            id
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

/// Implements raw work with the protocol in request-response format.
/// If the server returns a `Status` packet and it has the code Ok
/// then the packet is returned as Ok in other error cases
/// the packet is stored as Err.
pub struct RawSftpSession {
    tx: mpsc::UnboundedSender<Bytes>,
    requests: Arc<SharedData>,
    next_req_id: u32,
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
        }
    }

    async fn send(&self, id: Option<u32>, packet: Packet) -> Result<Packet, Error> {
        let (tx, rx) = oneshot::channel();

        self.requests.lock().await.push((id, tx));
        self.tx.send(Bytes::try_from(packet)?)?;

        // todo: remove from requests
        timeout(Duration::from_secs(10), rx).await??
    }

    fn use_next_id(&mut self) -> u32 {
        let id = self.next_req_id;
        self.next_req_id += 1;
        id
    }

    pub async fn init(&self) -> Result<Version, Error> {
        let result = self.send(None, Init::default().into()).await?;
        if let Packet::Version(version) = result {
            Ok(version)
        } else {
            Err(Error::UnexpectedPacket)
        }
    }

    pub async fn open<T: Into<String>>(
        &mut self,
        filename: T,
        flags: OpenFlags,
        attrs: FileAttributes,
    ) -> Result<Handle, Error> {
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

        into_with_status!(result, Handle)
    }

    pub async fn close<T: Into<String>>(&mut self, handle: T) -> Result<Status, Error> {
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

        into_status!(result)
    }

    pub async fn read<T: Into<String>>(
        &mut self,
        handle: T,
        offset: u64,
        len: u32,
    ) -> Result<Data, Error> {
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

    pub async fn write<T: Into<String>>(
        &mut self,
        handle: T,
        offset: u64,
        data: Vec<u8>,
    ) -> Result<Status, Error> {
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

    pub async fn lstat<T: Into<String>>(&mut self, path: T) -> Result<Attrs, Error> {
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

    pub async fn fstat<T: Into<String>>(&mut self, handle: T) -> Result<Attrs, Error> {
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

    pub async fn setstat<T: Into<String>>(
        &mut self,
        path: T,
        attrs: FileAttributes,
    ) -> Result<Status, Error> {
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

    pub async fn fsetstat<T: Into<String>>(
        &mut self,
        handle: T,
        attrs: FileAttributes,
    ) -> Result<Status, Error> {
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

    pub async fn opendir<T: Into<String>>(&mut self, path: T) -> Result<Handle, Error> {
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

        into_with_status!(result, Handle)
    }

    pub async fn readdir<T: Into<String>>(&mut self, handle: T) -> Result<Name, Error> {
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

    pub async fn remove<T: Into<String>>(&mut self, filename: T) -> Result<Status, Error> {
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

    pub async fn mkdir<T: Into<String>>(
        &mut self,
        path: T,
        attrs: FileAttributes,
    ) -> Result<Status, Error> {
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

    pub async fn rmdir<T: Into<String>>(&mut self, path: T) -> Result<Status, Error> {
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

    pub async fn realpath<T: Into<String>>(&mut self, path: T) -> Result<Name, Error> {
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

    pub async fn stat<T: Into<String>>(&mut self, path: T) -> Result<Attrs, Error> {
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

    pub async fn rename<O, N>(&mut self, oldpath: O, newpath: N) -> Result<Status, Error>
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

    pub async fn readlink<T: Into<String>>(&mut self, path: T) -> Result<Name, Error> {
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

    pub async fn symlink<P, T>(&mut self, path: P, target: T) -> Result<Status, Error>
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

    pub async fn extended(
        &mut self,
        request: String,
        data: Vec<u8>,
    ) -> Result<ExtendedReply, Error> {
        let id = self.use_next_id();
        let result = self
            .send(Some(id), Extended { id, request, data }.into())
            .await?;

        into_with_status!(result, ExtendedReply)
    }
}
