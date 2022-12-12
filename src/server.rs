use bytes::Bytes;
use russh::{server::Msg, Channel, ChannelStream};
use tokio::io::AsyncReadExt;

use crate::{error::Error, packets::Packet};

async fn handler(stream: &mut ChannelStream) -> Result<(), Error> {
    let length = stream.read_u32().await.map_err(Error::from)?;

    let mut buf = vec![0; length as usize];
    stream.read_exact(&mut buf).await.map_err(Error::from)?;

    let mut bytes = Bytes::from(buf);
    let packet = Packet::try_from(&mut bytes)?;

    info!("packet: {:?}", packet);

    Ok(())
}

pub async fn run(channel: Channel<Msg>) {
    let mut stream = channel.into_stream();
    tokio::spawn(async move {
        loop {
            match handler(&mut stream).await {
                Err(Error::UnexpectedEof) => break,
                Err(err) => error!("{:?}", err),
                Ok(_) => (),
            }
        }

        debug!("sftp stream ended");
    });
}
