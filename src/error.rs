use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Protocol(ErrorProtocol),
    #[error("Unexpected EOF on buffer reading")]
    UnexpectedEof,
    #[error("{0}")]
    Internal(String),
}

impl Error {
    pub(crate) fn from(error: std::io::Error) -> Self {
        let kind = error.kind();
        let msg = error
            .into_inner()
            .map_or("".to_string(), |m| format!("{m}"));

        match kind {
            std::io::ErrorKind::Other if msg == "EOF" => Self::UnexpectedEof,
            err => Self::Internal(err.to_string()),
        }
    }
}

#[derive(Debug, Error)]
pub enum ErrorProtocol {
    #[error("Ok")]
    Ok,
    #[error("Eof")]
    Eof,
    #[error("No such file")]
    NoSuchFile,
    #[error("Permission denined")]
    PermissionDenined,
    #[error("Failure")]
    Failure,
    #[error("Bad message")]
    BadMessage,
    #[error("No connection")]
    NoConnection,
    #[error("Connection lost")]
    ConnectionLost,
    #[error("Operation unsupported")]
    OpUnsupported,
}
