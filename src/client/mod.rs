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
    println!("{:?}", bytes);

    let packet = Packet::try_from(&mut bytes)?;
    match packet {
        Packet::Version(p) => match handler.version(p).await {
            _ => (),
        },
        Packet::ExtendedReply(p) => match handler.extended_reply(p).await {
            _ => (),
        },
        _ => (),
    }

    Ok(())
}

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
                        Err(err) => error!("{}", err),
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
