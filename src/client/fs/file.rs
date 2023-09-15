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
    client::{error::Error, rawsession::SftpResult, RawSftpSession},
    protocol::StatusCode,
};

type StateFn<T> = Option<Pin<Box<dyn Future<Output = io::Result<T>> + Send + Sync + 'static>>>;

pub struct FileState {
    f_read: StateFn<Option<Vec<u8>>>,
    f_seek: StateFn<u64>,
    f_write: StateFn<usize>,
    _f_flush: StateFn<()>,
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
}

impl File {
    pub(crate) fn new(session: Arc<Mutex<RawSftpSession>>, handle: String) -> Self {
        Self {
            session,
            handle,
            state: FileState {
                f_read: None,
                f_seek: None,
                f_write: None,
                _f_flush: None,
                f_shutdown: None,
            },
            pos: 0,
            closed: false,
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
        let file_handle = self.handle.to_owned();
        let remaining = buf.remaining();
        let offset = self.pos;

        let poll = Pin::new(self.state.f_read.get_or_insert(Box::pin(async move {
            let result = session
                .lock()
                .await
                .read(file_handle, offset, remaining as u32)
                .await;

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
        let file_handle = self.handle.to_owned();
        let offset = self.pos;
        let data = buf.to_vec();

        let poll = Pin::new(self.state.f_write.get_or_insert(Box::pin(async move {
            let len = data.len();
            session
                .lock()
                .await
                .write(file_handle, offset, data)
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

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        // todo: fsync@openssh.com required to use
        Poll::Ready(Ok(()))
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
