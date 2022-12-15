use crate::protocol::{Handle, Name, StatusCode, Version};

#[async_trait]
pub trait Handler: Sized {
    type Error: Into<StatusCode>;

    fn unimplemented(self) -> Self::Error;

    #[allow(unused_variables)]
    async fn init(self, version: u32) -> Result<Version, Self::Error> {
        Ok(Version::new())
    }

    #[allow(unused_variables)]
    async fn opendir(self, id: u32, path: String) -> Result<Handle, Self::Error> {
        Err(self.unimplemented())
    }

    #[allow(unused_variables)]
    async fn realpath(self, id: u32, path: String) -> Result<Name, Self::Error> {
        Err(self.unimplemented())
    }
}
