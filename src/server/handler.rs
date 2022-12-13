use std::fmt::Display;

use crate::{
    packet::{Init, Version},
    server::Packet,
};

#[async_trait]
pub trait Handler: Sized {
    type Error: From<crate::ErrorProtocol> + Display;

    #[allow(unused_variables)]
    async fn init(&mut self, init: Init) -> Result<Packet, Self::Error> {
        Ok(Version::new().into())
    }
}
