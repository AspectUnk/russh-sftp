use std::io;
use thiserror::Error;

use crate::protocol::StatusCode;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O: {0}")]
    IO(String),
    #[error("{0}")]
    Protocol(#[from] StatusCode),
    #[error("Unexpected EOF on stream")]
    UnexpectedEof,
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

impl Into<StatusCode> for Error {
    fn into(self) -> StatusCode {
        match self {
            Self::Protocol(status_code) => status_code,
            _ => StatusCode::Failure,
        }
    }
}
