use anyhow::Result;
use async_trait::async_trait;
use log::{error, info, LevelFilter};
use russh::*;
use russh_keys::*;
use russh_sftp::{client::SftpSession, protocol::OpenFlags};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

struct Client;

#[async_trait]
impl client::Handler for Client {
    type Error = anyhow::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        info!("check_server_key: {:?}", server_public_key);
        Ok(true)
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        info!("data on channel {:?}: {}", channel, data.len());
        Ok(())
    }
}

async fn sftp_client(sh: &Client, config: Arc<Config>) -> Result<()> {
    let mut session = russh::client::connect(config, ("localhost", 22), sh)
        .await?;
    if session
        .authenticate_password("root", "password")
        .await?
    {
        let channel = session.channel_open_session().await?;
        channel.request_subsystem(true, "sftp").await?;
        let sftp = SftpSession::new(channel.into_stream()).await?;
        info!("current path: {:?}", sftp.canonicalize(".").await?);

        // create dir and symlink
        let path = "./some_kind_of_dir";
        let symlink = "./symlink";

        sftp.create_dir(path).await?;
        sftp.symlink(path, symlink).await?;

        info!("dir info: {:?}", sftp.metadata(path).await?);
        info!(
            "symlink info: {:?}",
            sftp.symlink_metadata(path).await?
        );

        // scanning directory
        for entry in sftp.read_dir(".").await? {
            info!("file in directory: {:?}", entry.file_name());
        }

        sftp.remove_file(symlink).await?;
        sftp.remove_dir(path).await?;

        // interaction with i/o
        let filename = "test_new.txt";
        let mut file = sftp
            .open_with_flags(
                filename,
                OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE | OpenFlags::READ,
            )
            .await?;
        info!("metadata by handle: {:?}", file.metadata().await?);

        file.write_all(b"magic text").await?;
        info!("flush: {:?}", file.flush().await); // or file.sync_all()
        info!(
            "current cursor position: {:?}",
            file.stream_position().await
        );

        let mut str = String::new();

        file.rewind().await?;
        file.read_to_string(&mut str).await?;
        file.rewind().await?;

        info!(
            "our magical contents: {}, after rewind: {:?}",
            str,
            file.stream_position().await
        );

        file.shutdown().await?;
        sftp.remove_file(filename).await?;

        // should fail because handle was closed
        error!("should fail: {:?}", file.read_u8().await);
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .init();

    let config = russh::client::Config::default();
    let sh = Client {};

    if let Err(e) = sftp_client(&sh, Arc::new(config)) {
        error!("SFTP client failed: {}", e)
    }
}
