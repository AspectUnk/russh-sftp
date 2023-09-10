use async_trait::async_trait;
use russh::*;
use russh_keys::*;
use russh_sftp::client::SftpSession;
use std::sync::Arc;

struct Client;

#[async_trait]
impl client::Handler for Client {
    type Error = anyhow::Error;

    async fn check_server_key(
        self,
        server_public_key: &key::PublicKey,
    ) -> Result<(Self, bool), Self::Error> {
        println!("check_server_key: {:?}", server_public_key);
        Ok((self, true))
    }

    async fn data(
        self,
        channel: ChannelId,
        data: &[u8],
        session: client::Session,
    ) -> Result<(Self, client::Session), Self::Error> {
        println!("data on channel {:?}: {}", channel, data.len());
        Ok((self, session))
    }
}

#[tokio::main]
async fn main() {
    let config = russh::client::Config::default();
    let sh = Client {};
    let mut session = russh::client::connect(Arc::new(config), ("localhost", 22), sh)
        .await
        .unwrap();
    if session
        .authenticate_password("root", "pass")
        .await
        .unwrap()
    {
        let mut channel = session.channel_open_session().await.unwrap();
        channel.request_subsystem(true, "sftp").await.unwrap();
        let _session = SftpSession::new(channel.into_stream()).await.unwrap();
        //println!("{:?}", session);
    }
}
