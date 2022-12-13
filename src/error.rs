use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Protocol(ErrorProtocol),
    #[error("Unexpected EOF on buffer reading")]
    UnexpectedEof,
    #[error("{0}")]
    Internal(String),
    #[error("{0}")]
    Custom(String),
}

impl Error {
    pub fn from(err: std::io::Error) -> Self {
        let kind = err.kind();
        let msg = err.into_inner().map_or("".to_string(), |m| format!("{m}"));

        match kind {
            std::io::ErrorKind::Other if msg == "EOF" => Self::UnexpectedEof,
            err => Self::Internal(err.to_string()),
        }
    }
}

impl From<ErrorProtocol> for Error {
    fn from(err: ErrorProtocol) -> Self {
        Self::Protocol(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(err.to_string())
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ErrorProtocol {
    #[error("Ok")]
    Ok = 0,
    #[error("Eof")]
    Eof = 1,
    #[error("No such file")]
    NoSuchFile = 2,
    #[error("Permission denined")]
    PermissionDenined = 3,
    #[error("Failure")]
    Failure = 4,
    #[error("Bad message")]
    BadMessage = 5,
    #[error("No connection")]
    NoConnection = 6,
    #[error("Connection lost")]
    ConnectionLost = 7,
    #[error("Operation unsupported")]
    OpUnsupported = 8,
}
