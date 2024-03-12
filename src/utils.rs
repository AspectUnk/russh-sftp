use bytes::Bytes;
use chrono::{DateTime, Utc};
use std::time::SystemTime;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};

use crate::error::Error;

pub fn unix(time: SystemTime) -> u32 {
    DateTime::<Utc>::from(time).timestamp() as u32
}

pub async fn read_packet<S: AsyncRead + AsyncWrite + Unpin + Send>(
    stream: &mut S,
) -> Result<Bytes, Error> {
    let length = stream.read_u32().await?;

    let mut buf = vec![0; length as usize];
    _ = stream.read_exact(&mut buf).await?;

    Ok(Bytes::from(buf))
}
