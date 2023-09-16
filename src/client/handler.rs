use super::error::Error;
use crate::protocol::{Attrs, Data, ExtendedReply, Handle, Name, Status, Version};

/// Client stream handler. This is `async_trait`
#[async_trait]
pub trait Handler: Sized {
    type Error: Into<Error>;

    /// Called on SSH_FXP_VERSION.
    #[allow(unused_variables)]
    async fn version(&mut self, version: Version) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called on SSH_FXP_STATUS.
    #[allow(unused_variables)]
    async fn status(&mut self, status: Status) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called on SSH_FXP_HANDLE.
    #[allow(unused_variables)]
    async fn handle(&mut self, handle: Handle) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called on SSH_FXP_DATA.
    #[allow(unused_variables)]
    async fn data(&mut self, data: Data) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called on SSH_FXP_NAME.
    #[allow(unused_variables)]
    async fn name(&mut self, name: Name) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called on SSH_FXP_ATTRS.
    #[allow(unused_variables)]
    async fn attrs(&mut self, attrs: Attrs) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called on SSH_EXTENDED_REPLY.
    #[allow(unused_variables)]
    async fn extended_reply(&mut self, reply: ExtendedReply) -> Result<(), Self::Error> {
        Ok(())
    }
}
