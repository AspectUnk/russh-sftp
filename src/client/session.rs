use tokio::io::{AsyncRead, AsyncWrite};

use crate::error::Error;

use super::rawsession::RawSftpSession;

pub struct SftpSession {
    raw: RawSftpSession,
}

impl SftpSession {
    pub async fn new<S>(stream: S) -> Result<Self, Error>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let mut raw = RawSftpSession::new(stream);
        let version = raw.init().await?;

        if version.extensions["limits@openssh.com"] == "1" {
            println!("has limits");
            let reply = raw
                .extended("limits@openssh.com".to_owned(), vec![])
                .await?;
            println!("{:?}", reply);
        }

        Ok(Self { raw })
    }
}
