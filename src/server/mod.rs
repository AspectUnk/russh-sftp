mod handler;

use bytes::{BufMut, Bytes, BytesMut};
use russh::{server::Msg, Channel, ChannelStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub use self::handler::Handler;
use crate::{
    buf::TryBuf,
    error::Error,
    packet::{Init, Status, Version, SSH_FXP_INIT, SSH_FXP_STATUS, SSH_FXP_VERSION},
    ErrorProtocol,
};

pub enum Packet {
    Version(Version),
    Status(Status),
}

impl From<Packet> for Bytes {
    fn from(packet: Packet) -> Self {
        let (r#type, payload): (u8, Bytes) = match packet {
            Packet::Version(p) => (SSH_FXP_VERSION, p.into()),
            Packet::Status(p) => (SSH_FXP_STATUS, p.into()),
        };

        let length = payload.len() as u32 + 1;

        let mut bytes = BytesMut::new();
        bytes.put_u32(length);
        bytes.put_u8(r#type);
        bytes.put_slice(&payload);
        bytes.freeze()
    }
}

impl From<Error> for Packet {
    fn from(err: Error) -> Self {
        Packet::Status(Status::new(
            0,
            match err {
                Error::Protocol(e) => e,
                _ => ErrorProtocol::Failure,
            },
            &err.to_string(),
        ))
    }
}

async fn read_buf(stream: &mut ChannelStream) -> Result<Bytes, Error> {
    let length = stream.read_u32().await.map_err(Error::from)?;

    let mut buf = vec![0; length as usize];
    stream.read_exact(&mut buf).await.map_err(Error::from)?;

    Ok(Bytes::from(buf))
}

async fn handler<H>(stream: &mut ChannelStream, mut handler: H) -> Result<(), Error>
where
    H: Handler + Clone + Send,
{
    let mut buffer = read_buf(stream).await?;

    let r#type = buffer.try_get_u8()?;
    debug!("packet type {}", r#type);
    let packet = match r#type {
        SSH_FXP_INIT => {
            let init = Init::try_from(&mut buffer)?;
            handler
                .init(init)
                .await
                .map_err(|e| Error::Custom(e.to_string()))
        }
        _ => Err(Error::Protocol(ErrorProtocol::OpUnsupported)),
    };

    let bytes: Bytes = match packet {
        Ok(packet) => packet,
        Err(err) => Packet::from(err),
    }
    .into();

    stream.write_all(&bytes).await?;

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
                Err(err) => error!("{:?}", err),
                Ok(_) => (),
            }
        }

        debug!("sftp stream ended");
    });
}
