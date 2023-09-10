use crate::{error::Error, protocol::{Version, ExtendedReply}};

#[async_trait]
pub trait Handler: Sized {
    type Error: From<Error>;

    #[allow(unused_variables)]
    async fn version(&mut self, version: Version) -> Result<(), Self::Error> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn extended_reply(&mut self, reply: ExtendedReply) -> Result<(), Self::Error> {
        Ok(())
    }
}
