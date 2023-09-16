use std::{
    future::Future,
    io::{self, SeekFrom},
    pin::Pin,
    sync::Arc,
    task::{ready, Context, Poll},
};
use tokio::{
    io::{AsyncRead, AsyncSeek, AsyncWrite, ReadBuf},
    runtime::Handle,
    sync::Mutex,
};

use super::Metadata;
use crate::{
    client::{error::Error, rawsession::SftpResult, session::Extensions, RawSftpSession},
    error,
    extensions::FsyncExtension,
    protocol::{Packet, StatusCode},
};

type StateFn<T> = Option<Pin<Box<dyn Future<Output = io::Result<T>> + Send + Sync + 'static>>>;

pub struct FileState {
    f_read: StateFn<Option<Vec<u8>>>,
    f_seek: StateFn<u64>,
    f_write: StateFn<usize>,
    f_flush: StateFn<()>,
    f_shutdown: StateFn<()>,
}

/// File implement [`AsyncSeek`].
///
/// # Weakness
/// Using [`SeekFrom::End`] is costly and time-consuming because we need
/// to request the actual file size from the remote server
pub struct File {
    session: Arc<Mutex<RawSftpSession>>,
    handle: String,
    state: FileState,
    pos: u64,
    closed: bool,
    extensions: Arc<Extensions>,
}

impl File {
    pub(crate) fn new(
        session: Arc<Mutex<RawSftpSession>>,
        handle: String,
        extensions: Arc<Extensions>,
    ) -> Self {
        Self {
            session,
            handle,
            state: FileState {
                f_read: None,
                f_seek: None,
                f_write: None,
                f_flush: None,
                f_shutdown: None,
            },
            pos: 0,
            closed: false,
            extensions,
        }
    }

    pub async fn metadata(&self) -> SftpResult<Metadata> {
        Ok(self
            .session
            .lock()
            .await
            .fstat(self.handle.as_str())
            .await?
            .attrs)
    }

    pub async fn set_metadata(&self, metadata: Metadata) -> SftpResult<()> {
        self.session
            .lock()
            .await
            .fsetstat(self.handle.as_str(), metadata)
            .await
            .map(|_| ())
    }

    /// Attempts to sync all data.
    ///
    /// If the server does not support `fsync@openssh.com` sending the request will
    /// be omitted, but will still pseudo-successfully
    pub async fn sync_all(&self) -> SftpResult<()> {
        if !self.extensions.fsync {
            return Ok(());
        }

        let result = self
            .session
            .lock()
            .await
            .extended(
                "fsync@openssh.com",
                FsyncExtension {
                    handle: self.handle.to_owned(),
                }
                .try_into()?,
            )
            .await;

        match result {
            Ok(Packet::Status(status)) if status.status_code == StatusCode::Ok => Ok(()),
            Err(error) => Err(error),
            _ => Err(Error::UnexpectedPacket),
        }
    }
}

impl Drop for File {
    fn drop(&mut self) {
        if self.closed {
            return;
        }

        if let Ok(handle) = Handle::try_current() {
            let session = self.session.to_owned();
            let file_handle = self.handle.to_owned();

            handle.spawn(async move {
                let _ = session.lock().await.close(file_handle).await;
            });
        }
    }
}

impl AsyncRead for File {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let session = self.session.to_owned();
        let limits = self.extensions.limits.to_owned();
        let handle = self.handle.to_owned();
        let remaining = buf.remaining();
        let offset = self.pos;

        let poll = Pin::new(self.state.f_read.get_or_insert(Box::pin(async move {
            let limit = limits.map(|l| l.read_len).flatten().unwrap_or(4) as usize;
            let len = if remaining > limit { limit } else { remaining };

            let result = session.lock().await.read(handle, offset, len as u32).await;

            match result {
                Ok(data) => Ok(Some(data.data)),
                Err(Error::Status(status)) if status.status_code == StatusCode::Eof => Ok(None),
                Err(e) => Err(io::Error::new(io::ErrorKind::Other, e.to_string())),
            }
        })))
        .poll(cx);

        if poll.is_ready() {
            self.state.f_read = None;
        }

        match poll {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Ready(Ok(None)) => Poll::Ready(Ok(())),
            Poll::Ready(Ok(Some(data))) => {
                self.pos += data.len() as u64;
                buf.put_slice(&data[..]);
                Poll::Ready(Ok(()))
            }
        }
    }
}

impl AsyncSeek for File {
    fn start_seek(mut self: Pin<&mut Self>, position: io::SeekFrom) -> io::Result<()> {
        match self.state.f_seek {
            Some(_) => Err(io::Error::new(
                io::ErrorKind::Other,
                "other file operation is pending, call poll_complete before start_seek",
            )),
            None => {
                let session = self.session.clone();
                let file_handle = self.handle.to_owned();
                let cur_pos = self.pos as i64;

                self.state.f_seek = Some(Box::pin(async move {
                    let new_pos = match position {
                        SeekFrom::Start(pos) => pos as i64,
                        SeekFrom::Current(pos) => cur_pos + pos,
                        SeekFrom::End(pos) => {
                            let result =
                                session.lock().await.fstat(file_handle).await.map_err(|e| {
                                    io::Error::new(io::ErrorKind::Other, e.to_string())
                                })?;

                            match result.attrs.size {
                                Some(size) => size as i64 + pos,
                                None => {
                                    return Err(io::Error::new(
                                        io::ErrorKind::Other,
                                        "file size unknown",
                                    ))
                                }
                            }
                        }
                    };

                    if new_pos < 0 {
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            "cannot move file pointer before the beginning",
                        ));
                    }

                    Ok(new_pos as u64)
                }));

                Ok(())
            }
        }
    }

    fn poll_complete(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<u64>> {
        match self.state.f_seek.as_mut() {
            None => Poll::Ready(Ok(self.pos)),
            Some(f) => {
                self.pos = ready!(Pin::new(f).poll(cx))?;
                self.state.f_seek = None;
                Poll::Ready(Ok(self.pos))
            }
        }
    }
}

impl AsyncWrite for File {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        let session = self.session.to_owned();
        let limits = self.extensions.limits.to_owned();
        let handle = self.handle.to_owned();
        let offset = self.pos;
        let data = buf.to_vec();

        let poll = Pin::new(self.state.f_write.get_or_insert(Box::pin(async move {
            let limit = limits.map(|l| l.write_len).flatten().unwrap_or(261120) as usize;
            let len = if data.len() > limit {
                limit
            } else {
                data.len()
            };

            session
                .lock()
                .await
                .write(handle, offset, (&data[..len]).to_vec())
                .await
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

            Ok(len)
        })))
        .poll(cx);

        if poll.is_ready() {
            self.state.f_write = None;
        }

        if let Poll::Ready(Ok(len)) = poll {
            self.pos += len as u64;
        }

        poll
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        if !self.extensions.fsync {
            return Poll::Ready(Ok(()));
        }

        let session = self.session.to_owned();
        let handle = self.handle.to_owned();

        let poll = Pin::new(self.state.f_flush.get_or_insert(Box::pin(async move {
            let result = session
                .lock()
                .await
                .extended(
                    "fsync@openssh.com",
                    FsyncExtension { handle }
                        .try_into()
                        .map_err(|e: error::Error| {
                            io::Error::new(io::ErrorKind::Other, e.to_string())
                        })?,
                )
                .await;

            match result {
                Ok(Packet::Status(status)) if status.status_code == StatusCode::Ok => Ok(()),
                Err(error) => Err(io::Error::new(io::ErrorKind::Other, error.to_string())),
                _ => Err(io::Error::new(io::ErrorKind::Other, "Unexpected packet")),
            }
        })))
        .poll(cx);

        if poll.is_ready() {
            self.state.f_flush = None;
        }

        poll
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        let session = self.session.to_owned();
        let file_handle = self.handle.to_owned();

        let poll = Pin::new(self.state.f_shutdown.get_or_insert(Box::pin(async move {
            session
                .lock()
                .await
                .close(file_handle)
                .await
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
            Ok(())
        })))
        .poll(cx);

        if poll.is_ready() {
            self.state.f_shutdown = None;
            self.closed = true;
        }

        poll
    }
}
