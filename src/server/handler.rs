use std::collections::HashMap;

use crate::{
    file::FileAttributes,
    protocol::{Attrs, Handle, Name, OpenFlags, Status, StatusCode, Version},
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
    async fn lstat(self, id: u32, path: String) -> Result<Attrs, Self::Error> {
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
    async fn realpath(self, id: u32, path: String) -> Result<Name, Self::Error> {
        Err(self.unimplemented())
    }
}
