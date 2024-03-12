use async_trait::async_trait;
use log::{error, info, LevelFilter};
use russh::{client, ChannelId};
use russh_keys::key;
use russh_sftp::{client::SftpSession, protocol::OpenFlags};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

struct Client;

#[async_trait]
impl client::Handler for Client {
    type Error = anyhow::Error;

    async fn check_server_key(
        self,
        server_public_key: &key::PublicKey,
    ) -> Result<(Self, bool), Self::Error> {
        info!("check_server_key: {:?}", server_public_key);
        Ok((self, true))
    }

    async fn data(
        self,
        channel: ChannelId,
        data: &[u8],
        session: client::Session,
    ) -> Result<(Self, client::Session), Self::Error> {
        info!("data on channel {:?}: {}", channel, data.len());
        Ok((self, session))
    }
}

#[tokio::main]
#[allow(clippy::expect_used)]
async fn main() {
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .init();

    let config = russh::client::Config::default();

    let sh = Client {};

    let mut session = russh::client::connect(Arc::new(config), ("localhost", 22), sh)
        .await
        .expect("connection failed");

    if session
        .authenticate_password("root", "pass")
        .await
        .expect("auth failed")
    {
        let channel = session
            .channel_open_session()
            .await
            .expect("channel open failed");

        channel
            .request_subsystem(true, "sftp")
            .await
            .expect("subsystem failed");

        let sftp = SftpSession::new(channel.into_stream())
            .await
            .expect("sftp failed");

        info!(
            "current path: {:?}",
            sftp.canonicalize(".").await.expect("canonicalize failed")
        );

        // create dir and symlink
        let path = "./some_kind_of_dir";
        let symlink = "./symlink";

        sftp.create_dir(path).await.expect("create dir failed");

        sftp.symlink(path, symlink).await.expect("symlink failed");

        info!(
            "dir info: {:?}",
            sftp.metadata(path).await.expect("metadata failed")
        );

        info!(
            "symlink info: {:?}",
            sftp.symlink_metadata(path)
                .await
                .expect("symlink metadata failed")
        );

        // scanning directory
        for entry in sftp.read_dir(".").await.expect("read dir failed") {
            info!("file in directory: {:?}", entry.file_name());
        }

        sftp.remove_file(symlink)
            .await
            .expect("remove symlink failed");

        sftp.remove_dir(path).await.expect("remove dir failed");

        // interaction with i/o
        let filename = "test_new.txt";
        let mut file = sftp
            .open_with_flags(
                filename,
                OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE | OpenFlags::READ,
            )
            .await
            .expect("open file failed");

        info!(
            "metadata by handle: {:?}",
            file.metadata().await.expect("metadata failed")
        );

        file.write_all(b"magic text").await.expect("write failed");

        info!("flush: {:?}", file.flush().await); // or file.sync_all()

        info!(
            "current cursor position: {:?}",
            file.stream_position().await
        );

        let mut str = String::new();

        let _res = file.rewind().await.expect("rewind failed");

        let _res = file.read_to_string(&mut str).await.expect("read failed");

        let _res = file.rewind().await.expect("rewind failed");

        info!(
            "our magical contents: {}, after rewind: {:?}",
            str,
            file.stream_position().await
        );

        file.shutdown().await.expect("shutdown failed");
        sftp.remove_file(filename)
            .await
            .expect("remove file failed");

        // should fail because handle was closed
        error!("should fail: {:?}", file.read_u8().await);
    }
}
