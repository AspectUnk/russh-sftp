mod handler;

use bytes::Bytes;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub use self::handler::Handler;

use crate::{
    error::Error,
    protocol::{Request, Response, StatusCode},
};

macro_rules! into_wrap {
    ($id:expr, $handler:expr, $var:ident; $($arg:ident),*) => {
        match $handler.$var($($var.$arg),*).await {
            Err(err) => Response::error($id, err.into()),
            Ok(packet) => packet.into(),
        }
    };
}

async fn read_buf<S>(stream: &mut S) -> Result<Bytes, Error>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let length = stream.read_u32().await?;

    let mut buf = vec![0; length as usize];
    stream.read_exact(&mut buf).await?;

    Ok(Bytes::from(buf))
}

async fn process_request<H>(request: Request, handler: &mut H) -> Response
where
    H: Handler + Send,
{
    let id = request.get_request_id();

    match request {
        Request::Init(init) => into_wrap!(id, handler, init; version, extensions),
        Request::Open(open) => into_wrap!(id, handler, open; id, filename, pflags, attrs),
        Request::Close(close) => into_wrap!(id, handler, close; id, handle),
        Request::Read(read) => into_wrap!(id, handler, read; id, handle, offset, len),
        Request::Write(write) => into_wrap!(id, handler, write; id, handle, offset, data),
        Request::Lstat(lstat) => into_wrap!(id, handler, lstat; id, path),
        Request::Fstat(fstat) => into_wrap!(id, handler, fstat; id, handle),
        Request::SetStat(setstat) => into_wrap!(id, handler, setstat; id, path, attrs),
        Request::FSetStat(fsetstat) => into_wrap!(id, handler, fsetstat; id, handle, attrs),
        Request::OpenDir(opendir) => into_wrap!(id, handler, opendir; id, path),
        Request::ReadDir(readdir) => into_wrap!(id, handler, readdir; id, handle),
        Request::Remove(remove) => into_wrap!(id, handler, remove; id, filename),
        Request::Mkdir(mkdir) => into_wrap!(id, handler, mkdir; id, path, attrs),
        Request::Rmdir(rmdir) => into_wrap!(id, handler, rmdir; id, path),
        Request::RealPath(realpath) => into_wrap!(id, handler, realpath; id, path),
        Request::Stat(stat) => into_wrap!(id, handler, stat; id, path),
        Request::Rename(rename) => into_wrap!(id, handler, rename; id, oldpath, newpath),
        Request::ReadLink(readlink) => into_wrap!(id, handler, readlink; id, path),
        Request::Symlink(symlink) => into_wrap!(id, handler, symlink; id, linkpath, targetpath),
        Request::Extended(extended) => into_wrap!(id, handler, extended; id, request, data),
    }
}

async fn process_handler<H, S>(stream: &mut S, handler: &mut H) -> Result<(), Error>
where
    H: Handler + Send,
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut bytes = read_buf(stream).await?;

    let response = match Request::try_from(&mut bytes) {
        Ok(request) => process_request(request, handler).await,
        Err(_) => Response::error(0, StatusCode::BadMessage),
    };

    let packet = Bytes::from(response);
    stream.write_all(&packet).await?;

    Ok(())
}

/// Run processing stream as SFTP
pub async fn run<S, H>(mut stream: S, mut handler: H)
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    H: Handler + Send + 'static,
{
    tokio::spawn(async move {
        loop {
            match process_handler(&mut stream, &mut handler).await {
                Err(Error::UnexpectedEof) => break,
                Err(err) => warn!("{}", err),
                Ok(_) => (),
            }
        }

        debug!("sftp stream ended");
    });
}
