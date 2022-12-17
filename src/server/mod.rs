mod handler;

use bytes::Bytes;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub use self::handler::Handler;

use crate::{
    error::Error,
    protocol::{Request, Response, StatusCode},
};

macro_rules! into_wrap {
    ($id:expr, $handler:expr) => {
        match $handler.await {
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
        Request::Init(init) => into_wrap!(id, handler.init(init.version, init.extensions)),
        Request::Open(open) => into_wrap!(
            id,
            handler.open(open.id, open.filename, open.pflags, open.attrs)
        ),
        Request::Close(close) => into_wrap!(id, handler.close(close.id, close.handle)),
        Request::Read(read) => into_wrap!(
            id,
            handler.read(read.id, read.handle, read.offset, read.len)
        ),
        Request::Write(write) => into_wrap!(
            id,
            handler.write(write.id, write.handle, write.offset, write.data)
        ),
        Request::Lstat(lstat) => into_wrap!(id, handler.lstat(lstat.id, lstat.path)),
        Request::Fstat(fstat) => into_wrap!(id, handler.fstat(fstat.id, fstat.handle)),
        Request::SetStat(setstat) => {
            into_wrap!(id, handler.setstat(setstat.id, setstat.path, setstat.attrs))
        }
        Request::FSetStat(fsetstat) => into_wrap!(
            id,
            handler.fsetstat(fsetstat.id, fsetstat.handle, fsetstat.attrs)
        ),
        Request::OpenDir(opendir) => into_wrap!(id, handler.opendir(opendir.id, opendir.path)),
        Request::ReadDir(readdir) => into_wrap!(id, handler.readdir(readdir.id, readdir.handle)),
        Request::Remove(remove) => into_wrap!(id, handler.remove(remove.id, remove.filename)),
        Request::Mkdir(mkdir) => into_wrap!(id, handler.mkdir(mkdir.id, mkdir.path, mkdir.attrs)),
        Request::Rmdir(rmdir) => into_wrap!(id, handler.rmdir(rmdir.id, rmdir.path)),
        Request::RealPath(realpath) => into_wrap!(id, handler.realpath(realpath.id, realpath.path)),
        Request::Stat(stat) => into_wrap!(id, handler.stat(stat.id, stat.path)),
        Request::Rename(rename) => into_wrap!(
            id,
            handler.rename(rename.id, rename.oldpath, rename.newpath)
        ),
        Request::ReadLink(readlink) => into_wrap!(id, handler.readlink(readlink.id, readlink.path)),
        Request::Symlink(symlink) => into_wrap!(
            id,
            handler.symlink(symlink.id, symlink.linkpath, symlink.targetpath)
        ),
        Request::Extended(extended) => into_wrap!(
            id,
            handler.extended(extended.id, extended.request, extended.data)
        ),
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

/// Run processing channel as SFTP
pub async fn run<H, S>(mut stream: S, mut handler: H)
where
    H: Handler + Send + 'static,
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
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
