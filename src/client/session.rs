use tokio::io::{AsyncRead, AsyncWrite};

use super::{error::Error, fs::Metadata, rawsession::RawSftpSession};
use crate::protocol::{FileAttributes, StatusCode};

/// High-level SFTP implementation for easy interaction with a remote file system
pub struct SftpSession {
    raw: RawSftpSession,
}

impl SftpSession {
    pub async fn new<S>(stream: S) -> Result<Self, Error>
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

        Ok(Self { raw })
    }

    pub async fn canonicalize<T: Into<String>>(&mut self, path: T) -> Result<String, Error> {
        let name = self.raw.realpath(path).await?;
        match name.files.get(0) {
            Some(file) => Ok(file.filename.to_owned()),
            None => Err(Error::UnexpectedBehavior("no file".to_owned())),
        }
    }

    pub async fn create_dir<T: Into<String>>(&mut self, path: T) -> Result<(), Error> {
        self.raw
            .mkdir(path, FileAttributes::default())
            .await
            .map(|_| ())
    }

    pub async fn read_dir<T: Into<String>>(
        &mut self,
        path: T,
    ) -> Result<Vec<(String, Metadata)>, Error> {
        let mut files = vec![];

        let handle = self.raw.opendir(path).await?.handle;

        loop {
            match self.raw.readdir(handle.as_str()).await {
                Ok(name) => {
                    files = name
                        .files
                        .into_iter()
                        .map(|f| (f.filename.into(), f.attrs))
                        .chain(files.into_iter())
                        .collect();
                }
                Err(Error::Status(status)) if status.status_code == StatusCode::Eof => break,
                Err(err) => return Err(err),
            }
        }

        self.raw.close(handle).await?;
        Ok(files)
    }

    pub async fn read_link<T: Into<String>>(&mut self, path: T) -> Result<String, Error> {
        let name = self.raw.readlink(path).await?;
        match name.files.get(0) {
            Some(file) => Ok(file.filename.to_owned()),
            None => Err(Error::UnexpectedBehavior("no file".to_owned())),
        }
    }

    pub async fn remove_dir<T: Into<String>>(&mut self, path: T) -> Result<(), Error> {
        self.raw.rmdir(path).await.map(|_| ())
    }

    pub async fn remove_file<T: Into<String>>(&mut self, filename: T) -> Result<(), Error> {
        self.raw.remove(filename).await.map(|_| ())
    }

    pub async fn rename<O, N>(&mut self, oldpath: O, newpath: N) -> Result<(), Error>
    where
        O: Into<String>,
        N: Into<String>,
    {
        self.raw.rename(oldpath, newpath).await.map(|_| ())
    }

    pub async fn symlink<P, T>(&mut self, path: P, target: T) -> Result<(), Error>
    where
        P: Into<String>,
        T: Into<String>,
    {
        self.raw.symlink(path, target).await.map(|_| ())
    }

    pub async fn metadata<T: Into<String>>(&mut self, path: T) -> Result<Metadata, Error> {
        Ok(self.raw.stat(path).await?.attrs)
    }

    pub async fn set_metadata<T: Into<String>>(
        &mut self,
        path: T,
        metadata: Metadata,
    ) -> Result<(), Error> {
        self.raw.setstat(path, metadata).await.map(|_| ())
    }

    pub async fn symlink_metadata<T: Into<String>>(&mut self, path: T) -> Result<Metadata, Error> {
        Ok(self.raw.lstat(path).await?.attrs)
    }
}
