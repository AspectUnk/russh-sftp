use std::sync::Arc;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Mutex,
};

use super::{
    error::Error,
    fs::{File, Metadata, ReadDir},
    rawsession::{RawSftpSession, SftpResult},
};
use crate::protocol::{FileAttributes, OpenFlags, StatusCode};

/// High-level SFTP implementation for easy interaction with a remote file system.
/// Contains most methods similar to the native [filesystem](std::fs)
pub struct SftpSession {
    raw: Arc<Mutex<RawSftpSession>>,
}

impl SftpSession {
    pub async fn new<S>(stream: S) -> SftpResult<Self>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let raw = RawSftpSession::new(stream);
        let version = raw.init().await?;

        // todo: implement limit request
        if version.extensions["limits@openssh.com"] == "1" {
            println!("has limits");
            // let reply = raw
            //     .extended("limits@openssh.com".to_owned(), vec![])
            //     .await?;
            // println!("{:?}", reply);
        }

        Ok(Self {
            raw: Arc::new(Mutex::new(raw)),
        })
    }

    pub async fn open<T: Into<String>>(&self, filename: T) -> SftpResult<File> {
        let handle = self
            .raw
            .lock()
            .await
            .open(filename, OpenFlags::READ, FileAttributes::default())
            .await?
            .handle;
        
        Ok(File {
            raw: self.raw.clone(),
            handle,
        })
    }

    pub async fn canonicalize<T: Into<String>>(&self, path: T) -> SftpResult<String> {
        let name = self.raw.lock().await.realpath(path).await?;
        match name.files.get(0) {
            Some(file) => Ok(file.filename.to_owned()),
            None => Err(Error::UnexpectedBehavior("no file".to_owned())),
        }
    }

    pub async fn create_dir<T: Into<String>>(&self, path: T) -> SftpResult<()> {
        self.raw
            .lock()
            .await
            .mkdir(path, FileAttributes::default())
            .await
            .map(|_| ())
    }

    pub async fn read_dir<P: Into<String>>(&self, path: P) -> SftpResult<ReadDir> {
        let mut files = vec![];
        let handle = self.raw.lock().await.opendir(path).await?.handle;

        loop {
            match self.raw.lock().await.readdir(handle.as_str()).await {
                Ok(name) => {
                    files = name
                        .files
                        .into_iter()
                        .map(|f| (f.filename, f.attrs))
                        .chain(files.into_iter())
                        .collect();
                }
                Err(Error::Status(status)) if status.status_code == StatusCode::Eof => break,
                Err(err) => return Err(err),
            }
        }

        self.raw.lock().await.close(handle).await?;

        Ok(ReadDir {
            entries: files.into(),
        })
    }

    pub async fn read_link<P: Into<String>>(&self, path: P) -> SftpResult<String> {
        let name = self.raw.lock().await.readlink(path).await?;
        match name.files.get(0) {
            Some(file) => Ok(file.filename.to_owned()),
            None => Err(Error::UnexpectedBehavior("no file".to_owned())),
        }
    }

    pub async fn remove_dir<P: Into<String>>(&self, path: P) -> SftpResult<()> {
        self.raw.lock().await.rmdir(path).await.map(|_| ())
    }

    pub async fn remove_file<T: Into<String>>(&self, filename: T) -> SftpResult<()> {
        self.raw.lock().await.remove(filename).await.map(|_| ())
    }

    pub async fn rename<O, N>(&self, oldpath: O, newpath: N) -> SftpResult<()>
    where
        O: Into<String>,
        N: Into<String>,
    {
        self.raw
            .lock()
            .await
            .rename(oldpath, newpath)
            .await
            .map(|_| ())
    }

    pub async fn symlink<P, T>(&self, path: P, target: T) -> SftpResult<()>
    where
        P: Into<String>,
        T: Into<String>,
    {
        self.raw
            .lock()
            .await
            .symlink(path, target)
            .await
            .map(|_| ())
    }

    pub async fn metadata<P: Into<String>>(&self, path: P) -> SftpResult<Metadata> {
        Ok(self.raw.lock().await.stat(path).await?.attrs)
    }

    pub async fn set_metadata<P: Into<String>>(
        &self,
        path: P,
        metadata: Metadata,
    ) -> Result<(), Error> {
        self.raw
            .lock()
            .await
            .setstat(path, metadata)
            .await
            .map(|_| ())
    }

    pub async fn symlink_metadata<P: Into<String>>(&self, path: P) -> SftpResult<Metadata> {
        Ok(self.raw.lock().await.lstat(path).await?.attrs)
    }
}
