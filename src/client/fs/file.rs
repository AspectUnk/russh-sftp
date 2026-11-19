use std::{
    collections::VecDeque,
    future::{self, Future},
    io::{self, SeekFrom},
    pin::Pin,
    sync::Arc,
    task::{ready, Context, Poll},
};
use tokio::io::{AsyncRead, AsyncSeek, AsyncWrite, AsyncWriteExt, ReadBuf};

use super::Metadata;
use crate::{
    client::{
        error::Error,
        rawsession::{Request, SftpResult},
        session::Features,
        RawSftpSession,
    },
    protocol::{Packet, SftpHandle, StatusCode},
};

type StateFn<T> = Option<Pin<Box<dyn Future<Output = io::Result<T>> + Send + Sync + 'static>>>;

struct PendingRead {
    offset: u64,
    len: u32,
    rx: Request,
}

#[derive(Default)]
struct ReadState {
    pending: VecDeque<PendingRead>,
    buffer: Vec<u8>,
    pos: usize,
    offset: u64,
    chunk_len: Option<u32>,
    eof: bool,
}

impl ReadState {
    fn reset(&mut self, offset: u64) {
        self.pending.clear();
        self.buffer.clear();
        self.pos = 0;
        self.offset = offset;
        self.eof = false;
        // A short read near EOF must not limit later reads
        self.chunk_len = None;
    }

    fn poll_read(
        &mut self,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
        session: &RawSftpSession,
        handle: &SftpHandle,
        features: Features,
    ) -> Poll<io::Result<usize>> {
        if buf.remaining() == 0 {
            return Poll::Ready(Ok(0));
        }

        if self.pos == self.buffer.len() {
            if self.eof {
                return Poll::Ready(Ok(0));
            }

            self.request(session, handle, features)?;

            let request = self.pending.front_mut().expect("read request is queued");
            let result = ready!(Pin::new(&mut request.rx).poll(cx));
            let offset = request.offset;
            let len = request.len;
            self.pending.pop_front();

            match check_read_result(result) {
                Ok(Some(data)) => {
                    self.chunk_len.get_or_insert(data.len() as u32);
                    if data.len() < len as usize {
                        // Discard requests beyond a short read and retry from the gap.
                        self.pending.clear();
                        self.offset = offset + data.len() as u64;
                    }
                    self.buffer = data;
                    self.pos = 0;
                }
                Ok(None) => {
                    self.pending.clear();
                    self.eof = true;
                    return Poll::Ready(Ok(0));
                }
                Err(error) => {
                    self.reset(offset);
                    return Poll::Ready(Err(error));
                }
            }
        }

        let len = buf.remaining().min(self.buffer.len() - self.pos);
        buf.put_slice(&self.buffer[self.pos..self.pos + len]);
        self.pos += len;
        Poll::Ready(Ok(len))
    }

    fn request(
        &mut self,
        session: &RawSftpSession,
        handle: &SftpHandle,
        features: Features,
    ) -> io::Result<()> {
        let max_len = features.max_packet_len.saturating_sub(READ_OVERHEAD_LENGTH) as u64;
        let max_len = features
            .limits
            .and_then(|l| l.read_len)
            .unwrap_or(max_len)
            .min(max_len);
        let len = self.chunk_len.unwrap_or(max_len.max(1) as u32);

        // Probe the server's actual read size before filling the queue.
        let count = if self.chunk_len.is_some() {
            features.max_concurrent_reads
        } else {
            1
        };
        while self.pending.len() < count {
            let rx = session
                .read_nowait(handle, self.offset, len)
                .map_err(io::Error::from)?;
            self.pending.push_back(PendingRead {
                offset: self.offset,
                len,
                rx,
            });
            self.offset = self.offset.saturating_add(len as u64);
        }
        Ok(())
    }
}

// read packet overhead: packet_len(4) + type(1) + id(4) + data_len(4)
const READ_OVERHEAD_LENGTH: u32 = 13;
// write packet overhead excluding handle: packet_len(4) + type(1) + id(4) +
// handle_len(4) + offset(8) + data_len(4)
const WRITE_OVERHEAD_LENGTH: u32 = 25;

struct FileState {
    read: ReadState,
    f_seek: StateFn<u64>,
    f_flush: StateFn<()>,
    f_shutdown: StateFn<()>,
    write_acks: VecDeque<Request>,
}

/// Provides high-level methods for interaction with a remote file.
///
/// In order to properly close the handle, [`File::close`] or
/// [`shutdown`](tokio::io::AsyncWriteExt::shutdown) on a file should be called.
/// Also implement [`AsyncSeek`] and other async i/o implementations.
///
/// On drop the handle is closed as well, but the reply is not awaited, so
/// pending write errors and the close status are silently discarded
///
/// # Weakness
/// Using [`SeekFrom::End`] is costly and time-consuming because we need to
/// request the actual file size from the remote server.
pub struct File {
    session: Arc<RawSftpSession>,
    handle: SftpHandle,
    state: FileState,
    pos: u64,
    closed: bool,
    features: Features,
}

impl File {
    pub(crate) fn new(
        session: Arc<RawSftpSession>,
        handle: SftpHandle,
        features: Features,
    ) -> Self {
        Self {
            session,
            handle,
            state: FileState {
                read: ReadState {
                    pending: VecDeque::with_capacity(features.max_concurrent_reads),
                    ..ReadState::default()
                },
                f_seek: None,
                f_flush: None,
                f_shutdown: None,
                write_acks: VecDeque::with_capacity(features.max_concurrent_writes),
            },
            pos: 0,
            closed: false,
            features,
        }
    }

    /// Queries metadata about the remote file.
    pub async fn metadata(&self) -> SftpResult<Metadata> {
        Ok(self.session.fstat(self.handle.clone()).await?.attrs)
    }

    /// Sets metadata for a remote file.
    pub async fn set_metadata(&self, metadata: Metadata) -> SftpResult<()> {
        self.session
            .fsetstat(self.handle.clone(), metadata)
            .await
            .map(|_| ())
    }

    /// Attempts to sync all data.
    ///
    /// If the server does not support `fsync@openssh.com` sending the request will
    /// be omitted, but will still pseudo-successfully
    pub async fn sync_all(&self) -> SftpResult<()> {
        if !self.features.fsync {
            return Ok(());
        }

        self.session.fsync(self.handle.clone()).await.map(|_| ())
    }

    /// Closes the file waiting for all pending writes and the close itself
    /// to be confirmed by the remote party.
    /// Equivalent to [`shutdown`](tokio::io::AsyncWriteExt::shutdown)
    pub async fn close(mut self) -> io::Result<()> {
        self.shutdown().await
    }

    /// Streams the remote file from the current position to `writer` using up
    /// to `max_inflight` concurrent SFTP `READ` requests, hiding per-request
    /// RTT. Each request asks for up to the negotiated `read_len`
    /// (or `max_packet_len - READ_OVERHEAD_LENGTH` when no limit is advertised).
    ///
    /// The [`AsyncRead`] impl issues one `READ` at a time and waits for the
    /// reply before sending the next, so sustained throughput is bounded by
    /// `chunk_size / RTT`. This helper mirrors how OpenSSH's `sftp` client
    /// keeps ~64 outstanding requests by default, so on a long-RTT link
    /// (e.g. transcontinental SSH) it can saturate the channel.
    ///
    /// Chunks are reassembled in offset order before being written to `writer`,
    /// so the output is byte-identical to a sequential read. Stops cleanly on
    /// the first server-signalled EOF (either an `Eof` status or a short
    /// read).
    ///
    /// Returns the number of bytes streamed. Updates `self.pos` to the new
    /// read offset. Memory usage is bounded by `max_inflight * chunk_size`
    /// (chunks held in an in-order reassembly buffer plus in-flight requests).
    pub async fn read_to_writer_pipelined<W>(
        &mut self,
        writer: &mut W,
        max_inflight: usize,
    ) -> SftpResult<u64>
    where
        W: tokio::io::AsyncWrite + Unpin,
    {
        use futures::stream::{FuturesUnordered, StreamExt};
        use std::collections::BTreeMap;
        use tokio::io::AsyncWriteExt;

        if max_inflight == 0 {
            return Err(Error::UnexpectedBehavior(
                "max_inflight must be at least 1".to_owned(),
            ));
        }

        let chunk_size = self
            .features
            .limits
            .and_then(|l| l.read_len)
            .unwrap_or_else(|| {
                self.features
                    .max_packet_len
                    .saturating_sub(READ_OVERHEAD_LENGTH) as u64
            }) as usize;

        let mut total: u64 = 0;
        let mut next_offset = self.pos;
        let mut next_to_write = self.pos;
        let mut pending: BTreeMap<u64, Vec<u8>> = BTreeMap::new();
        let mut in_flight = FuturesUnordered::new();
        let mut eof = false;

        loop {
            // Schedule new read requests until we hit the cap or have observed EOF.
            while !eof && in_flight.len() < max_inflight {
                let session = self.session.clone();
                let handle = self.handle.clone();
                let off = next_offset;
                let len = chunk_size as u32;

                in_flight.push(async move {
                    match session.read(handle, off, len).await {
                        Ok(data) => Ok::<(u64, Option<Vec<u8>>), Error>((off, Some(data.data))),
                        Err(Error::Status(s)) if s.status_code == StatusCode::Eof => {
                            Ok((off, None))
                        }
                        Err(e) => Err(e),
                    }
                });

                next_offset += chunk_size as u64;
            }

            match in_flight.next().await {
                Some(Ok((off, Some(data)))) => {
                    let data: Vec<u8> = data;
                    if data.is_empty() {
                        eof = true;
                    } else {
                        pending.insert(off, data);
                    }
                }
                Some(Ok((_, None))) => {
                    eof = true;
                }
                Some(Err(e)) => return Err(e),
                None => break,
            }

            // Flush in-order chunks to `writer` as they become available.
            while let Some(chunk) = pending.remove(&next_to_write) {
                let n = chunk.len() as u64;
                writer.write_all(&chunk).await?;
                next_to_write += n;
                total += n;
            }
        }

        self.pos = next_to_write;
        Ok(total)
    }
}

fn check_write_result(result: SftpResult<Packet>) -> io::Result<()> {
    match result {
        Ok(Packet::Status(s)) if s.status_code == StatusCode::Ok => Ok(()),
        Ok(Packet::Status(s)) => Err(io::Error::other(s.error_message)),
        Ok(_) => Err(io::Error::other("unexpected response packet")),
        Err(e) => Err(e.into()),
    }
}

fn check_read_result(result: SftpResult<Packet>) -> io::Result<Option<Vec<u8>>> {
    match result {
        Ok(Packet::Data(data)) if data.data.is_empty() => Ok(None),
        Ok(Packet::Data(data)) => Ok(Some(data.data)),
        Ok(Packet::Status(status)) if status.status_code == StatusCode::Eof => Ok(None),
        Ok(Packet::Status(status)) => Err(io::Error::other(status.error_message)),
        Ok(_) => Err(io::Error::other("unexpected response packet")),
        Err(Error::Status(status)) if status.status_code == StatusCode::Eof => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn poll_oldest_write(
    pending: &mut VecDeque<Request>,
    cx: &mut Context<'_>,
) -> Option<Poll<io::Result<()>>> {
    let rx = pending.front_mut()?;
    Some(match Pin::new(rx).poll(cx) {
        Poll::Pending => Poll::Pending,
        Poll::Ready(r) => {
            pending.pop_front();
            Poll::Ready(check_write_result(r))
        }
    })
}

fn poll_drain_writes(
    pending: &mut VecDeque<Request>,
    cx: &mut Context<'_>,
) -> Poll<io::Result<()>> {
    while let Some(poll) = poll_oldest_write(pending, cx) {
        ready!(poll)?;
    }
    Poll::Ready(Ok(()))
}

impl Drop for File {
    fn drop(&mut self) {
        if self.closed {
            return;
        }

        let _ = self.session.close_nowait(std::mem::take(&mut self.handle));
    }
}

impl AsyncRead for File {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let file = self.get_mut();
        let len =
            ready!(file
                .state
                .read
                .poll_read(cx, buf, &file.session, &file.handle, file.features,))?;
        file.pos += len as u64;
        Poll::Ready(Ok(()))
    }
}

impl AsyncSeek for File {
    fn start_seek(mut self: Pin<&mut Self>, position: io::SeekFrom) -> io::Result<()> {
        if self.state.f_seek.is_some() {
            return Err(io::Error::other(
                "other file operation is pending, call poll_complete before start_seek",
            ));
        }

        self.state.f_seek = Some(match position {
            SeekFrom::Start(pos) => Box::pin(future::ready(Ok(pos))),
            SeekFrom::Current(pos) => {
                let new_pos = self.pos as i64 + pos;
                if new_pos < 0 {
                    return Err(io::Error::other(
                        "cannot move file pointer before the beginning",
                    ));
                }
                Box::pin(future::ready(Ok(new_pos as u64)))
            }
            SeekFrom::End(pos) => {
                let session = self.session.clone();
                let file_handle = self.handle.clone();

                Box::pin(async move {
                    let result = session.fstat(file_handle).await.map_err(io::Error::from)?;
                    match result.attrs.size {
                        Some(size) => {
                            let new_pos = size as i64 + pos;
                            if new_pos < 0 {
                                return Err(io::Error::other(
                                    "cannot move file pointer before the beginning",
                                ));
                            }
                            Ok(new_pos as u64)
                        }
                        None => Err(io::Error::other("file size unknown")),
                    }
                })
            }
        });

        Ok(())
    }

    fn poll_complete(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<u64>> {
        match self.state.f_seek.as_mut() {
            None => Poll::Ready(Ok(self.pos)),
            Some(f) => {
                let result = ready!(Pin::new(f).poll(cx));
                self.state.f_seek = None;
                self.pos = result?;
                let pos = self.pos;
                self.state.read.reset(pos);
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
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }

        if self.state.write_acks.len() >= self.features.max_concurrent_writes {
            if let Some(poll) = poll_oldest_write(&mut self.state.write_acks, cx) {
                ready!(poll)?;
            }
        }

        let packet_write_len = self
            .features
            .max_packet_len
            .saturating_sub(WRITE_OVERHEAD_LENGTH + self.handle.as_bytes().len() as u32)
            as usize;
        let server_write_len = self
            .features
            .limits
            .and_then(|limits| limits.write_len)
            .unwrap_or(u32::MAX as u64)
            .min(usize::MAX as u64) as usize;
        let preferred_write_len = self
            .features
            .max_write_packet_len
            .saturating_sub(WRITE_OVERHEAD_LENGTH + self.handle.as_bytes().len() as u32)
            .max(1) as usize;

        let len = buf
            .len()
            .min(packet_write_len)
            .min(server_write_len)
            .min(preferred_write_len);
        if len == 0 && !buf.is_empty() {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "configured SFTP packet limit is too small for a write",
            )));
        }
        let offset = self.pos;

        match self
            .session
            .write_nowait_from_slice(self.handle.as_bytes(), offset, &buf[..len])
        {
            Ok(rx) => {
                self.pos += len as u64;
                let pos = self.pos;
                self.state.read.reset(pos);
                self.state.write_acks.push_back(rx);
                Poll::Ready(Ok(len))
            }
            Err(e) => Poll::Ready(Err(e.into())),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        ready!(poll_drain_writes(&mut self.state.write_acks, cx))?;

        if !self.features.fsync {
            return Poll::Ready(Ok(()));
        }

        let poll = Pin::new(match self.state.f_flush.as_mut() {
            Some(f) => f,
            None => {
                let session = self.session.clone();
                let file_handle = self.handle.clone();

                self.state.f_flush.get_or_insert(Box::pin(async move {
                    session
                        .fsync(file_handle)
                        .await
                        .map(|_| ())
                        .map_err(io::Error::from)
                }))
            }
        })
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
        let poll = Pin::new(match self.state.f_shutdown.as_mut() {
            Some(f) => f,
            None => {
                if self.closed {
                    return Poll::Ready(Ok(()));
                }

                ready!(poll_drain_writes(&mut self.state.write_acks, cx))?;

                let request = self
                    .session
                    .close_nowait(self.handle.clone())
                    .map_err(io::Error::from)?;
                // The handle is invalid once CLOSE is sent, even if its reply is lost.
                self.closed = true;
                self.state
                    .f_shutdown
                    .get_or_insert(Box::pin(async move { check_write_result(request.await) }))
            }
        })
        .poll(cx);

        if poll.is_ready() {
            self.state.f_shutdown = None;
        }

        poll
    }
}
