use bytes::Bytes;
use jiff::Timestamp;
use std::time::SystemTime;
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::error::Error;

pub(crate) fn unix(time: SystemTime) -> u32 {
    Timestamp::try_from(time)
        .unwrap()
        .as_nanosecond()
        .div_euclid(1_000_000_000) as u32
}

pub(crate) async fn read_packet<S: AsyncRead + Unpin>(
    stream: &mut S,
    max_length: u32,
) -> Result<Bytes, Error> {
    let length = stream.read_u32().await?;
    if length > max_length {
        return Err(Error::BadMessage("packet length limit exceeded".to_owned()));
    }

    let mut buf = vec![0; length as usize];
    stream.read_exact(&mut buf).await?;

    Ok(Bytes::from(buf))
}
