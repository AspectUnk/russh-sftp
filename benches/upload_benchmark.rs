use async_trait::async_trait;
use criterion::{criterion_group, criterion_main, Criterion};
use log::debug;
use russh::{client, ChannelId};
use russh_keys::ssh_key;
use russh_sftp::client::SftpSession;
use std::sync::Arc;
use tokio::{
    io::AsyncWriteExt,
    task::{self},
    time::Instant,
};
struct Client;

#[async_trait]
impl client::Handler for Client {
    type Error = anyhow::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        debug!("check_server_key: {:?}", server_public_key);
        Ok(true)
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        debug!("data on channel {:?}: {}", channel, data.len());
        Ok(())
    }
}

async fn test_upload_data(sftp: SftpSession, file_count: i32, file_size: i32) {
    let data_vec: Vec<u8> = vec![0; file_size as usize];
    let mut handler_vec = Vec::new();
    let start_time = Instant::now();
    for i in 0..file_count {
        let path = format!("test_{i}.txt");
        let mut file = sftp.create(path).await.unwrap();
        let data_vec_bk = data_vec.clone();
        let handler = task::spawn(async move {
            let start_time = Instant::now();
            file.write_all(&data_vec_bk).await.unwrap();
            let elapsed_time = start_time.elapsed();
            println!("write_all Time elapsed: {:?}", elapsed_time);
        });
        handler_vec.push(handler);
    }

    futures::future::join_all(handler_vec).await;
    let elapsed_time = start_time.elapsed();
    println!("Time elapsed: {:?}", elapsed_time);
    for i in 0..file_count {
        let path = format!("test_{i}.txt");
        sftp.remove_file(path).await.unwrap();
    }
}

async fn upload_file(file_count: i32, file_size: i32) {
    let config = russh::client::Config::default();
    let sh = Client {};
    let mut session = russh::client::connect(Arc::new(config), ("localhost", 22), sh)
        .await
        .unwrap();
    if session
        .authenticate_password("root", "password")
        .await
        .unwrap()
    {
        let channel = session.channel_open_session().await.unwrap();
        channel.request_subsystem(true, "sftp").await.unwrap();
        let sftp = SftpSession::new(channel.into_stream()).await.unwrap();
        test_upload_data(sftp, file_count, file_size).await;
    }
}

fn criterion_benchmark_call(c: &mut Criterion) {
    c.bench_function("call", move |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                //If higher concurrency is required, please set the semaphore for channel in the fn connect_stream()
                upload_file(8, 1024 * 1024 * 10).await;
            })
    });
}

criterion_group!(
    name = instructions_bench;
    config = Criterion::default().sample_size(10);
    targets = criterion_benchmark_call
);
criterion_main!(instructions_bench);
