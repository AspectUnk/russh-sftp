use std::sync::Arc;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Mutex,
};

use super::{
    error::Error,
    fs::{File, Metadata, ReadDir},
    rawsession::SftpResult,
    RawSftpSession,
};
use crate::protocol::{FileAttributes, OpenFlags, StatusCode};

/// High-level SFTP implementation for easy interaction with a remote file system.
/// Contains most methods similar to the native [filesystem](std::fs)
pub struct SftpSession {
    session: Arc<Mutex<RawSftpSession>>,
}

impl SftpSession {
    pub async fn new<S>(stream: S) -> SftpResult<Self>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let session = RawSftpSession::new(stream);
        let version = session.init().await?;

        // todo: implement limit request
        if version.extensions["limits@openssh.com"] == "1" {
            println!("has limits");
            // let reply = session
            //     .extended("limits@openssh.com".to_owned(), vec![])
            //     .await?;
            // println!("{:?}", reply);
        }

        Ok(Self {
            session: Arc::new(Mutex::new(session)),
        })
    }

    pub async fn open<T: Into<String>>(&self, filename: T) -> SftpResult<File> {
        self.open_with_flags(filename, OpenFlags::READ).await
    }

    pub async fn create<T: Into<String>>(&self, filename: T) -> SftpResult<File> {
        self.open_with_flags(filename, OpenFlags::CREATE | OpenFlags::READ | OpenFlags::WRITE)
            .await
    }

    pub async fn open_with_flags<T: Into<String>>(
        &self,
        filename: T,
        flags: OpenFlags,
    ) -> SftpResult<File> {
        let handle = self
            .session
            .lock()
            .await
            .open(
                filename,
                flags,
                FileAttributes {
                    permissions: Some(0o755 | flags.bits()),
                    ..Default::default()
                },
            )
            .await?
            .handle;

        Ok(File::new(self.session.clone(), handle))
    }

    pub async fn canonicalize<T: Into<String>>(&self, path: T) -> SftpResult<String> {
        let name = self.session.lock().await.realpath(path).await?;
        match name.files.get(0) {
            Some(file) => Ok(file.filename.to_owned()),
            None => Err(Error::UnexpectedBehavior("no file".to_owned())),
        }
    }

    pub async fn create_dir<T: Into<String>>(&self, path: T) -> SftpResult<()> {
        self.session
            .lock()
            .await
            .mkdir(path, FileAttributes::default())
            .await
            .map(|_| ())
    }

    pub async fn read_dir<P: Into<String>>(&self, path: P) -> SftpResult<ReadDir> {
        let mut files = vec![];
        let handle = self.session.lock().await.opendir(path).await?.handle;

        loop {
            match self.session.lock().await.readdir(handle.as_str()).await {
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

        self.session.lock().await.close(handle).await?;

        Ok(ReadDir {
            entries: files.into(),
        })
    }

    pub async fn read_link<P: Into<String>>(&self, path: P) -> SftpResult<String> {
        let name = self.session.lock().await.readlink(path).await?;
        match name.files.get(0) {
            Some(file) => Ok(file.filename.to_owned()),
            None => Err(Error::UnexpectedBehavior("no file".to_owned())),
        }
    }

    pub async fn remove_dir<P: Into<String>>(&self, path: P) -> SftpResult<()> {
        self.session.lock().await.rmdir(path).await.map(|_| ())
    }

    pub async fn remove_file<T: Into<String>>(&self, filename: T) -> SftpResult<()> {
        self.session.lock().await.remove(filename).await.map(|_| ())
    }

    pub async fn rename<O, N>(&self, oldpath: O, newpath: N) -> SftpResult<()>
    where
        O: Into<String>,
        N: Into<String>,
    {
        self.session
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
        self.session
            .lock()
            .await
            .symlink(path, target)
            .await
            .map(|_| ())
    }

    pub async fn metadata<P: Into<String>>(&self, path: P) -> SftpResult<Metadata> {
        Ok(self.session.lock().await.stat(path).await?.attrs)
    }

    pub async fn set_metadata<P: Into<String>>(
        &self,
        path: P,
        metadata: Metadata,
    ) -> Result<(), Error> {
        self.session
            .lock()
            .await
            .setstat(path, metadata)
            .await
            .map(|_| ())
    }

    pub async fn symlink_metadata<P: Into<String>>(&self, path: P) -> SftpResult<Metadata> {
        Ok(self.session.lock().await.lstat(path).await?.attrs)
    }
}
