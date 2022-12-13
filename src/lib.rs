#[macro_use]
extern crate log;
#[macro_use]
extern crate async_trait;

mod buf;
mod error;
pub mod packet;
pub mod server;

pub use error::ErrorProtocol;

pub const PROTOCOL_VERSION: u32 = 3;
