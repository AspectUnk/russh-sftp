use std::collections::HashMap;

use crate::protocol::{
    Attrs, Data, FileAttributes, Handle, Name, OpenFlags, Packet, Status, StatusCode, Version,
};

/// Server handler for each client. This is `async_trait`
#[async_trait]
pub trait Handler: Sized {
    /// The type must have an `Into<StatusCode>`
    /// implementation because a response must be sent
    /// to any request, even if completed by error.
    type Error: Into<StatusCode>;

    /// Called by the handler when the packet is not implemented
    fn unimplemented(&self) -> Self::Error;

    /// The default is to send an SSH_FXP_VERSION response with
    /// the protocol version and ignore any extensions.
    #[allow(unused_variables)]
    async fn init(
        &mut self,
        version: u32,
        extensions: HashMap<String, String>,
    ) -> Result<Version, Self::Error> {
        Ok(Version::new())
    }

    /// Called on SSH_FXP_OPEN
    #[allow(unused_variables)]
    async fn open(
        &mut self,
        id: u32,
        filename: String,
        pflags: OpenFlags,
        attrs: FileAttributes,
    ) -> Result<Handle, Self::Error> {
        Err(self.unimplemented())
    }

    /// Called on SSH_FXP_CLOSE.
    /// The status can be returned as Ok or as Err
    #[allow(unused_variables)]
    async fn close(&mut self, id: u32, handle: String) -> Result<Status, Self::Error> {
        Err(self.unimplemented())
    }

    /// Called on SSH_FXP_READ
    #[allow(unused_variables)]
    async fn read(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        len: u32,
    ) -> Result<Data, Self::Error> {
        Err(self.unimplemented())
    }

    /// Called on SSH_FXP_WRITE
    #[allow(unused_variables)]
    async fn write(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        data: Vec<u8>,
    ) -> Result<Status, Self::Error> {
        Err(self.unimplemented())
    }

    /// Called on SSH_FXP_LSTAT
    #[allow(unused_variables)]
    async fn lstat(&mut self, id: u32, path: String) -> Result<Attrs, Self::Error> {
        Err(self.unimplemented())
    }

    /// Called on SSH_FXP_FSTAT
    #[allow(unused_variables)]
    async fn fstat(&mut self, id: u32, handle: String) -> Result<Attrs, Self::Error> {
        Err(self.unimplemented())
    }

    /// Called on SSH_FXP_SETSTAT
    #[allow(unused_variables)]
    async fn setstat(
        &mut self,
        id: u32,
        path: String,
        attrs: FileAttributes,
    ) -> Result<Status, Self::Error> {
        Err(self.unimplemented())
    }

    /// Called on SSH_FXP_FSETSTAT
    #[allow(unused_variables)]
    async fn fsetstat(
        &mut self,
        id: u32,
        handle: String,
        attrs: FileAttributes,
    ) -> Result<Status, Self::Error> {
        Err(self.unimplemented())
    }

    /// Called on SSH_FXP_OPENDIR
    #[allow(unused_variables)]
    async fn opendir(&mut self, id: u32, path: String) -> Result<Handle, Self::Error> {
        Err(self.unimplemented())
    }

    /// Called on SSH_FXP_READDIR.
    /// EOF error should be returned at the end of reading the directory
    #[allow(unused_variables)]
    async fn readdir(&mut self, id: u32, handle: String) -> Result<Name, Self::Error> {
        Err(self.unimplemented())
    }

    /// Called on SSH_FXP_REMOVE.
    /// The status can be returned as Ok or as Err
    #[allow(unused_variables)]
    async fn remove(&mut self, id: u32, filename: String) -> Result<Status, Self::Error> {
        Err(self.unimplemented())
    }

    /// Called on SSH_FXP_MKDIR
    #[allow(unused_variables)]
    async fn mkdir(
        &mut self,
        id: u32,
        path: String,
        attrs: FileAttributes,
    ) -> Result<Status, Self::Error> {
        Err(self.unimplemented())
    }

    /// Called on SSH_FXP_RMDIR.
    /// The status can be returned as Ok or as Err
    #[allow(unused_variables)]
    async fn rmdir(&mut self, id: u32, path: String) -> Result<Status, Self::Error> {
        Err(self.unimplemented())
    }

    /// Called on SSH_FXP_REALPATH.
    /// Must contain only one name and a dummy attributes
    #[allow(unused_variables)]
    async fn realpath(&mut self, id: u32, path: String) -> Result<Name, Self::Error> {
        Err(self.unimplemented())
    }

    /// Called on SSH_FXP_STAT
    #[allow(unused_variables)]
    async fn stat(&mut self, id: u32, path: String) -> Result<Attrs, Self::Error> {
        Err(self.unimplemented())
    }

    /// Called on SSH_FXP_RENAME.
    /// The status can be returned as Ok or as Err
    #[allow(unused_variables)]
    async fn rename(
        &mut self,
        id: u32,
        oldpath: String,
        newpath: String,
    ) -> Result<Status, Self::Error> {
        Err(self.unimplemented())
    }

    /// Called on SSH_FXP_READLINK
    #[allow(unused_variables)]
    async fn readlink(&mut self, id: u32, path: String) -> Result<Name, Self::Error> {
        Err(self.unimplemented())
    }

    /// Called on SSH_FXP_SYMLINK.
    /// The status can be returned as Ok or as Err
    #[allow(unused_variables)]
    async fn symlink(
        &mut self,
        id: u32,
        linkpath: String,
        targetpath: String,
    ) -> Result<Status, Self::Error> {
        Err(self.unimplemented())
    }

    /// Called on SSH_FXP_EXTENDED.
    /// The extension can return any packet, so it's not specific.
    /// If the server does not recognize the `request' name
    /// the server must respond with an SSH_FX_OP_UNSUPPORTED error
    #[allow(unused_variables)]
    async fn extended(
        &mut self,
        id: u32,
        request: String,
        data: Vec<u8>,
    ) -> Result<Packet, Self::Error> {
        Err(self.unimplemented())
    }
}
