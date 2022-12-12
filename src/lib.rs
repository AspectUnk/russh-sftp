#[macro_use]
extern crate log;

mod buf;
mod error;
pub mod packets;
pub mod server;

pub use error::{Error, ErrorProtocol};

pub const PROTOCOL_VERSION: u32 = 3;
