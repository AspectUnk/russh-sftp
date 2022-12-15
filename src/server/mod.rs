mod handler;

use bytes::Bytes;
use russh::{server::Msg, Channel, ChannelStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

async fn read_buf(stream: &mut ChannelStream) -> Result<Bytes, Error> {
    let length = stream.read_u32().await?;

    let mut buf = vec![0; length as usize];
    stream.read_exact(&mut buf).await?;

    Ok(Bytes::from(buf))
}

async fn process_request<H>(request: Request, handler: H) -> Response
where
    H: Handler + Clone + Send,
{
    let id = request.get_id();

    match request {
        Request::Init(init) => into_wrap!(id, handler.init(init.version, init.extensions)),
        Request::Close(close) => into_wrap!(id, handler.close(close.id, close.handle)),
        Request::OpenDir(opendir) => into_wrap!(id, handler.opendir(opendir.id, opendir.path)),
        Request::ReadDir(readdir) => into_wrap!(id, handler.readdir(readdir.id, readdir.handle)),
        Request::RealPath(realpath) => into_wrap!(id, handler.realpath(realpath.id, realpath.path)),
    }
}

async fn handler<H>(stream: &mut ChannelStream, handler: H) -> Result<(), Error>
where
    H: Handler + Clone + Send,
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

pub async fn run<H>(channel: Channel<Msg>, handle: H)
where
    H: Handler + Clone + Send + Sync + 'static,
{
    let mut stream = channel.into_stream();
    tokio::spawn(async move {
        loop {
            match handler(&mut stream, handle.clone()).await {
                Err(Error::UnexpectedEof) => break,
                Err(err) => warn!("{}", err),
                Ok(_) => (),
            }
        }

        debug!("sftp stream ended");
    });
}
