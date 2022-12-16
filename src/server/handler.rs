use std::collections::HashMap;

use crate::{
    file::FileAttributes,
    protocol::{Attrs, Data, Handle, Name, OpenFlags, Status, StatusCode, Version},
};

#[async_trait]
pub trait Handler: Sized {
    type Error: Into<StatusCode>;

    /// Called by the handler when the packet is not implemented
    fn unimplemented(self) -> Self::Error;

    #[allow(unused_variables)]
    async fn init(
        self,
        version: u32,
        extensions: HashMap<String, String>,
    ) -> Result<Version, Self::Error> {
        Ok(Version::new())
    }

    #[allow(unused_variables)]
    async fn open(
        self,
        id: u32,
        filename: String,
        pflags: OpenFlags,
        attrs: FileAttributes,
    ) -> Result<Handle, Self::Error> {
        Err(self.unimplemented())
    }

    ///
    #[allow(unused_variables)]
    async fn close(self, id: u32, handle: String) -> Result<Status, Self::Error> {
        Err(self.unimplemented())
    }

    #[allow(unused_variables)]
    async fn read(
        self,
        id: u32,
        handle: String,
        offset: u64,
        len: u32,
    ) -> Result<Data, Self::Error> {
        Err(self.unimplemented())
    }

    #[allow(unused_variables)]
    async fn write(
        self,
        id: u32,
        handle: String,
        offset: u64,
        data: Vec<u8>,
    ) -> Result<Status, Self::Error> {
        Err(self.unimplemented())
    }

    #[allow(unused_variables)]
    async fn lstat(self, id: u32, path: String) -> Result<Attrs, Self::Error> {
        Err(self.unimplemented())
    }

    #[allow(unused_variables)]
    async fn fstat(self, id: u32, handle: String) -> Result<Attrs, Self::Error> {
        Err(self.unimplemented())
    }

    #[allow(unused_variables)]
    async fn setstat(
        self,
        id: u32,
        path: String,
        attrs: FileAttributes,
    ) -> Result<Attrs, Self::Error> {
        Err(self.unimplemented())
    }

    #[allow(unused_variables)]
    async fn fsetstat(
        self,
        id: u32,
        handle: String,
        attrs: FileAttributes,
    ) -> Result<Attrs, Self::Error> {
        Err(self.unimplemented())
    }

    #[allow(unused_variables)]
    async fn opendir(self, id: u32, path: String) -> Result<Handle, Self::Error> {
        Err(self.unimplemented())
    }

    #[allow(unused_variables)]
    async fn readdir(self, id: u32, handle: String) -> Result<Name, Self::Error> {
        Err(self.unimplemented())
    }

    #[allow(unused_variables)]
    async fn remove(self, id: u32, filename: String) -> Result<Status, Self::Error> {
        Err(self.unimplemented())
    }

    #[allow(unused_variables)]
    async fn mkdir(
        self,
        id: u32,
        path: String,
        attrs: FileAttributes,
    ) -> Result<Status, Self::Error> {
        Err(self.unimplemented())
    }

    #[allow(unused_variables)]
    async fn rmdir(self, id: u32, path: String) -> Result<Status, Self::Error> {
        Err(self.unimplemented())
    }

    #[allow(unused_variables)]
    async fn realpath(self, id: u32, path: String) -> Result<Name, Self::Error> {
        Err(self.unimplemented())
    }

    #[allow(unused_variables)]
    async fn stat(self, id: u32, path: String) -> Result<Attrs, Self::Error> {
        Err(self.unimplemented())
    }

    #[allow(unused_variables)]
    async fn rename(
        self,
        id: u32,
        oldpath: String,
        newpath: String,
    ) -> Result<Status, Self::Error> {
        Err(self.unimplemented())
    }

    #[allow(unused_variables)]
    async fn readlink(self, id: u32, path: String) -> Result<Name, Self::Error> {
        Err(self.unimplemented())
    }

    #[allow(unused_variables)]
    async fn symlink(
        self,
        id: u32,
        linkpath: String,
        targetpath: String,
    ) -> Result<Status, Self::Error> {
        Err(self.unimplemented())
    }
}
