use russh_sftp::{
    client::SftpSession,
    protocol::{FileAttributes, Handle, OpenFlags, Status, StatusCode},
    server::{self, StatusReply},
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

type Files = Arc<Mutex<HashMap<String, Vec<u8>>>>;

/// In-memory files honouring CREATE and TRUNCATE.
struct Server(Files);

fn ok(id: u32) -> Status {
    Status {
        id,
        status_code: StatusCode::Ok,
        error_message: String::new(),
        language_tag: String::new(),
    }
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
        pflags: OpenFlags,
        _: FileAttributes,
    ) -> Result<Handle, StatusReply> {
        let mut files = self.0.lock().unwrap();
        if pflags.contains(OpenFlags::CREATE) {
            files.entry(filename.clone()).or_default();
        }
        let file = files.get_mut(&filename).ok_or(StatusCode::NoSuchFile)?;
        if pflags.contains(OpenFlags::TRUNCATE) {
            file.clear();
        }
        Ok(Handle {
            id,
            handle: filename,
        })
    }

    async fn write(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        data: Vec<u8>,
    ) -> Result<Status, StatusReply> {
        let mut files = self.0.lock().unwrap();
        let file = files.get_mut(&handle).unwrap();
        let (start, end) = (offset as usize, offset as usize + data.len());
        if file.len() < end {
            file.resize(end, 0);
        }
        file[start..end].copy_from_slice(&data);
        Ok(ok(id))
    }

    async fn close(&mut self, id: u32, _: String) -> Result<Status, StatusReply> {
        Ok(ok(id))
    }
}

#[tokio::test]
async fn write_creates_and_truncates() {
    let files = Files::default();
    files
        .lock()
        .unwrap()
        .insert("existing".into(), b"hello world".to_vec());

    let (client, stream) = tokio::io::duplex(1 << 16);
    server::run(stream, Server(files.clone())).await;
    let sftp = SftpSession::new(client).await.unwrap();

    sftp.write("existing", b"bye").await.unwrap();
    sftp.write("new", b"data").await.unwrap();

    let files = files.lock().unwrap();
    assert_eq!(files["existing"], b"bye");
    assert_eq!(files["new"], b"data");
}
