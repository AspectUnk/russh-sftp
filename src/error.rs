use std::{fmt, io};
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum Error {
    #[error("I/O: {0}")]
    IO(String),
    #[error("Unexpected EOF on stream")]
    UnexpectedEof,
    #[error("Bad message: {0}")]
    BadMessage(String),
    #[error("Unexpected behavior: {0}")]
    UnexpectedBehavior(String),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        let kind = err.kind();
        let msg = err.into_inner().map_or("".to_string(), |m| format!("{m}"));
        match kind {
            io::ErrorKind::UnexpectedEof => Self::UnexpectedEof,
            io::ErrorKind::Other if msg == "EOF" => Self::UnexpectedEof,
            e => Self::IO(e.to_string()),
        }
    }
}

impl serde::ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Self::BadMessage(msg.to_string())
    }
}

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Self::BadMessage(msg.to_string())
    }
}
