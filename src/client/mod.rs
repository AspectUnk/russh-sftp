mod error;
pub mod fs;
mod handler;
mod rawsession;
mod session;

pub use handler::Handler;
pub use rawsession::RawSftpSession;
pub use session::SftpSession;

use bytes::Bytes;
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
    sync::mpsc,
};

use crate::{error::Error, protocol::Packet, utils::read_packet};

async fn process_handler<S, H>(stream: &mut S, handler: &mut H) -> Result<(), Error>
where
    S: AsyncRead + AsyncWrite + Unpin,
    H: Handler + Send,
{
    let mut bytes = read_packet(stream).await?;

    // todo: error logging
    let _ = match Packet::try_from(&mut bytes)? {
        Packet::Version(p) => handler.version(p).await,
        Packet::Status(p) => handler.status(p).await,
        Packet::Handle(p) => handler.handle(p).await,
        Packet::Data(p) => handler.data(p).await,
        Packet::Name(p) => handler.name(p).await,
        Packet::Attrs(p) => handler.attrs(p).await,
        Packet::ExtendedReply(p) => handler.extended_reply(p).await,
        _ => Ok(()),
    };

    Ok(())
}

/// Run processing stream as SFTP client. Is a simple handler of incoming
/// and outgoing packets. Can be used for non-standard implementations
pub fn run<S, H>(mut stream: S, mut handler: H) -> mpsc::UnboundedSender<Bytes>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    H: Handler + Send + 'static,
{
    let (tx, mut rx) = mpsc::unbounded_channel::<Bytes>();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                result = process_handler(&mut stream, &mut handler) => {
                    match result {
                        Err(Error::UnexpectedEof) => break,
                        Err(err) => warn!("{}", err),
                        Ok(_) => (),
                    }
                }
                Some(data) = rx.recv() => {
                    let _  = stream.write_all(&data[..]).await;
                }
            }
        }

        debug!("sftp stream ended");
    });

    tx
}
