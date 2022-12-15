use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O: {0}")]
    IO(String),
    #[error("Unexpected EOF on stream")]
    UnexpectedEof,
    #[error("Bad message")]
    BadMessage,
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        let kind = err.kind();
        let msg = err.into_inner().map_or("".to_string(), |m| format!("{m}"));
        match kind {
            io::ErrorKind::Other if msg == "EOF" => Self::UnexpectedEof,
            e => Self::IO(e.to_string()),
        }
    }
}
