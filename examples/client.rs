use async_trait::async_trait;
use russh::*;
use russh_keys::*;
use russh_sftp::client::SftpSession;
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
    if session.authenticate_password("root", "pass").await.unwrap() {
        let mut channel = session.channel_open_session().await.unwrap();
        channel.request_subsystem(true, "sftp").await.unwrap();
        let sftp = SftpSession::new(channel.into_stream()).await.unwrap();
        println!("current path: {:?}", sftp.canonicalize(".").await.unwrap());

        // create dir and symlink
        let path = "./some_kind_of_dir";
        let symlink = "./symlink";

        sftp.create_dir(path).await.unwrap();
        sftp.symlink(path, symlink).await.unwrap();

        println!("dir info: {:?}", sftp.metadata(path).await.unwrap());
        println!(
            "symlink info: {:?}",
            sftp.symlink_metadata(path).await.unwrap()
        );

        // scanning directory
        for entry in sftp.read_dir(".").await.unwrap() {
            println!("file in directory: {:?}", entry.file_name());
        }

        sftp.remove_file(symlink).await.unwrap();
        sftp.remove_dir(path).await.unwrap();

        // interaction with i/o
        let filename = "test_new.txt";
        let mut file = sftp.create("test_new.txt").await.unwrap();
        println!("metadata by handle: {:?}", file.metadata().await.unwrap());

        file.write_all(b"magic text").await.unwrap();

        let mut str = String::new();
        file.read_to_string(&mut str).await.unwrap();
        println!(
            "our magical contents: {:?}, cursor position: {:?}",
            str,
            file.stream_position().await
        );

        file.shutdown().await.unwrap();
        sftp.remove_file(filename).await.unwrap();

        // should fail because handle was closed
        println!("should fail: {:?}", file.read_u8().await);
    }
}
