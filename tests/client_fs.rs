use russh_sftp::{
    client::SftpSession,
    protocol::{Attrs, Data, FileAttributes, Handle, Name, OpenFlags, Status, StatusCode},
    server::{self, StatusReply},
};
use std::{
    io::SeekFrom,
    sync::{Arc, Mutex},
};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

const FILE_LEN: usize = 1 << 20;

/// Serves one read-only file of `FILE_LEN` bytes and a directory whose listing always fails.
#[derive(Clone, Default)]
struct Server {
    read_requests: Arc<Mutex<usize>>,
    closed: Arc<Mutex<Vec<String>>>,
}

impl server::Handler for Server {
    type Error = StatusReply;

    fn unimplemented(&self) -> StatusReply {
        StatusCode::OpUnsupported.into()
    }

    async fn open(
        &mut self,
        id: u32,
        filename: String,
        _: OpenFlags,
        _: FileAttributes,
    ) -> Result<Handle, StatusReply> {
        Ok(Handle {
            id,
            handle: filename,
        })
    }

    async fn opendir(&mut self, id: u32, path: String) -> Result<Handle, StatusReply> {
        Ok(Handle { id, handle: path })
    }

    async fn readdir(&mut self, _: u32, _: String) -> Result<Name, StatusReply> {
        Err(StatusCode::Failure.into())
    }

    async fn close(&mut self, id: u32, handle: String) -> Result<Status, StatusReply> {
        self.closed.lock().unwrap().push(handle);
        Ok(Status {
            id,
            status_code: StatusCode::Ok,
            error_message: String::new(),
            language_tag: String::new(),
        })
    }

    async fn fstat(&mut self, id: u32, _: String) -> Result<Attrs, StatusReply> {
        let attrs = FileAttributes {
            size: Some(FILE_LEN as u64),
            ..Default::default()
        };
        Ok(Attrs { id, attrs })
    }

    async fn read(
        &mut self,
        id: u32,
        _: String,
        offset: u64,
        len: u32,
    ) -> Result<Data, StatusReply> {
        *self.read_requests.lock().unwrap() += 1;
        let start = offset as usize;
        if start >= FILE_LEN {
            return Err(StatusCode::Eof.into());
        }
        let end = (start + len as usize).min(FILE_LEN);
        Ok(Data {
            id,
            data: vec![7; end - start],
        })
    }
}

async fn connect(server: Server) -> SftpSession {
    let (client, stream) = tokio::io::duplex(1 << 20);
    server::run(stream, server).await;
    SftpSession::new(client).await.unwrap()
}

#[tokio::test]
async fn short_read_near_eof_does_not_shrink_later_reads() {
    let server = Server::default();
    let sftp = connect(server.clone()).await;
    let mut file = sftp.open("f").await.unwrap();

    file.seek(SeekFrom::End(-10)).await.unwrap();
    let mut tail = vec![];
    file.read_to_end(&mut tail).await.unwrap();
    assert_eq!(tail.len(), 10);

    file.seek(SeekFrom::Start(0)).await.unwrap();
    *server.read_requests.lock().unwrap() = 0;
    let mut all = vec![];
    file.read_to_end(&mut all).await.unwrap();
    assert_eq!(all.len(), FILE_LEN);

    // Full-size chunks need ~5 requests plus the pipelined ones past EOF.
    let requests = *server.read_requests.lock().unwrap();
    assert!(requests < 100, "{requests} read requests for 1 MiB");
}

#[tokio::test]
async fn read_dir_error_closes_handle() {
    let server = Server::default();
    let sftp = connect(server.clone()).await;

    assert!(sftp.read_dir("dir").await.is_err());
    assert_eq!(*server.closed.lock().unwrap(), ["dir"]);
}
