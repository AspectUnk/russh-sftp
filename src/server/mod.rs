mod handler;

use bytes::Bytes;
use russh::{server::Msg, Channel, ChannelStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub use self::handler::Handler;

use crate::{
    error::Error,
    protocol::{Request, Response},
};

macro_rules! into_wrap {
    ($handler:expr) => {
        $handler.await.map_err(|e| e.into())?.into()
    };
}

async fn read_buf(stream: &mut ChannelStream) -> Result<Bytes, Error> {
    let length = stream.read_u32().await?;

    let mut buf = vec![0; length as usize];
    stream.read_exact(&mut buf).await?;

    Ok(Bytes::from(buf))
}

async fn process_packet<H>(bytes: &mut Bytes, handler: H) -> Result<Response, Error>
where
    H: Handler + Clone + Send,
{
    let response = match Request::try_from(bytes)? {
        Request::Init(p) => into_wrap!(handler.init(p.version)),
        Request::RealPath(p) => into_wrap!(handler.realpath(p.id, p.path)),
    };

    Ok(response)
}

async fn handler<H>(stream: &mut ChannelStream, handler: H) -> Result<(), Error>
where
    H: Handler + Clone + Send,
{
    let mut bytes = read_buf(stream).await?;

    let response: Bytes = match process_packet(&mut bytes, handler).await {
        Err(err) => Response::from(err).into(),
        Ok(response) => response.into(),
    };

    stream.write_all(&response).await?;

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
