pub mod error;
pub mod fs;
mod handler;
pub mod rawsession;
mod session;

pub use handler::Handler;
pub use rawsession::RawSftpSession;
pub use session::SftpSession;

use bytes::Bytes;
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt, split},
    sync::mpsc,
};

use crate::{error::Error, protocol::Packet, utils::read_packet};

macro_rules! into_wrap {
    ($handler:expr) => {
        match $handler.await {
            Err(error) => Err(error.into()),
            Ok(()) => Ok(()),
        }
    };
}

async fn execute_handler<H>(bytes: &mut Bytes, handler: &mut H) -> Result<(), error::Error>
where
    H: Handler + Send,
{
    match Packet::try_from(bytes)? {
        Packet::Version(p) => into_wrap!(handler.version(p)),
        Packet::Status(p) => into_wrap!(handler.status(p)),
        Packet::Handle(p) => into_wrap!(handler.handle(p)),
        Packet::Data(p) => into_wrap!(handler.data(p)),
        Packet::Name(p) => into_wrap!(handler.name(p)),
        Packet::Attrs(p) => into_wrap!(handler.attrs(p)),
        Packet::ExtendedReply(p) => into_wrap!(handler.extended_reply(p)),
        _ => Err(error::Error::UnexpectedBehavior(
            "A packet was received that could not be processed.".to_owned(),
        )),
    }
}

async fn process_handler<S, H>(stream: &mut S, handler: &mut H) -> Result<(), Error>
where
    S: AsyncRead + Unpin,
    H: Handler + Send,
{
    let mut bytes = read_packet(stream).await?;
    Ok(execute_handler(&mut bytes, handler).await?)
}

/// Run processing stream as SFTP client. Is a simple handler of incoming
/// and outgoing packets. Can be used for non-standard implementations
pub fn run<S, H>(stream: S, mut handler: H) -> mpsc::UnboundedSender<Bytes>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    H: Handler + Send + 'static,
{
    let (tx, mut rx) = mpsc::unbounded_channel::<Bytes>();

    let (mut rd, mut wr) = split(stream);
    {
        tokio::spawn(async move {
            loop {
                match process_handler(&mut rd, &mut handler).await {
                    Err(Error::UnexpectedEof) => break,
                    Err(err) => warn!("{}", err),
                    Ok(_) => (),
                }
            }

            debug!("read half of sftp stream ended");
        });
    }

    tokio::spawn(async move {
        loop {
            if let Some(data) = rx.recv().await {
                if data.is_empty() {
                    let _ = wr.shutdown().await;
                    break;
                }

                let _  = wr.write_all(&data[..]).await;
            }
        }

        debug!("write half of sftp stream ended");
    });

    tx
}
