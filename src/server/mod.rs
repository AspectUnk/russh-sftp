mod handler;

use bytes::{BufMut, Bytes, BytesMut};
use russh::{server::Msg, Channel, ChannelStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub use self::handler::Handler;
use crate::{
    buf::TryBuf,
    error::Error,
    protocol::{
        Init, Name, Path, Status, StatusCode, Version, SSH_FXP_INIT, SSH_FXP_NAME,
        SSH_FXP_REALPATH, SSH_FXP_STATUS, SSH_FXP_VERSION,
    },
};

pub enum Packet {
    Version(Version),
    Status(Status),
    Name(Name),
}

impl From<Packet> for Bytes {
    fn from(packet: Packet) -> Self {
        let (r#type, payload): (u8, Bytes) = match packet {
            Packet::Version(p) => (SSH_FXP_VERSION, p.into()),
            Packet::Status(p) => (SSH_FXP_STATUS, p.into()),
            Packet::Name(p) => (SSH_FXP_NAME, p.into()),
        };

        let length = payload.len() as u32 + 1;
        let mut bytes = BytesMut::new();

        bytes.put_u32(length);
        bytes.put_u8(r#type);
        bytes.put_slice(&payload);

        bytes.freeze()
    }
}

// impl TryFrom<&mut Bytes> for Packet {
//     type Error = Error;

//     fn try_from(bytes: &mut Bytes) -> Result<Self, Self::Error> {
//         let r#type = bytes.try_get_u8()?;
//         let packet = match r#type {
//             SSH_FXP_INIT => Self::
//             _ => Status::from(StatusCode::OpUnsupported).into(),
//         };

//         Ok(packet)
//     }
// }

impl From<StatusCode> for Packet {
    fn from(err: StatusCode) -> Self {
        Packet::Status(Status::new(0, err, &err.to_string()))
    }
}

async fn read_buf(stream: &mut ChannelStream) -> Result<Bytes, Error> {
    let length = stream.read_u32().await?;

    let mut buf = vec![0; length as usize];
    stream.read_exact(&mut buf).await?;

    Ok(Bytes::from(buf))
}

async fn handler<H>(stream: &mut ChannelStream, handler: H) -> Result<(), Error>
where
    H: Handler + Clone + Send,
{
    let mut buffer = read_buf(stream).await?;

    let r#type = buffer.try_get_u8()?;
    debug!("packet type {}", r#type);
    let packet: Bytes = match r#type {
        SSH_FXP_INIT => {
            let init = Init::try_from(&mut buffer)?;
            match handler.init(init.version).await {
                Ok(p) => Packet::from(p).into(),
                Err(e) => Packet::from(e.into()).into(),
            }
        }
        SSH_FXP_REALPATH => {
            let path = Path::try_from(&mut buffer)?;
            match handler.realpath(path.id, path.path).await {
                Ok(p) => Packet::from(p).into(),
                Err(e) => Packet::from(e.into()).into(),
            }
        }
        _ => Packet::from(StatusCode::OpUnsupported).into(),
    };

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
                Err(e) => warn!("{}", e),
                Ok(_) => (),
            }
        }

        debug!("sftp stream ended");
    });
}
