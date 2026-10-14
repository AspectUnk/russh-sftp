use serde::{Deserialize, Deserializer};
use thiserror::Error;

use super::{impl_packet_for, impl_request_id, Packet, RequestId};

/// Error Codes for SSH_FXP_STATUS
///
/// A code the server sends that is not listed here – later drafts define more, and some servers send their own –
/// is read as [`StatusCode::Failure`], so that one unusual reply does not end the whole session.
#[derive(Debug, Error, Clone, Copy, PartialEq, Serialize)]
pub enum StatusCode {
    /// Indicates successful completion of the operation.
    #[error("Ok")]
    Ok = 0,
    /// Indicates end-of-file condition; for SSH_FX_READ it means that no more data is available in the file,
    /// and for SSH_FX_READDIR it indicates that no more files are contained in the directory.
    #[error("Eof")]
    Eof = 1,
    /// A reference is made to a file which should exist but doesn't.
    #[error("No such file")]
    NoSuchFile = 2,
    /// Authenticated user does not have sufficient permissions to perform the operation.
    #[error("Permission denied")]
    PermissionDenied = 3,
    /// A generic catch-all error message;
    /// it should be returned if an error occurs for which there is no more specific error code defined.
    #[error("Failure")]
    Failure = 4,
    /// May be returned if a badly formatted packet or protocol incompatibility is detected.
    #[error("Bad message")]
    BadMessage = 5,
    /// A pseudo-error which indicates that the client has no connection to the server
    /// (it can only be generated locally by the client, and MUST NOT be returned by servers).
    #[error("No connection")]
    NoConnection = 6,
    /// A pseudo-error which indicates that the connection to the server has been lost
    /// (it can only be generated locally by the client, and MUST NOT be returned by servers).
    #[error("Connection lost")]
    ConnectionLost = 7,
    /// Indicates that an attempt was made to perform an operation which is not supported for the server
    /// (it may be generated locally by the client if e.g. the version number exchange indicates that a required feature is not supported by the server,
    /// or it may be returned by the server if the server does not implement an operation).
    #[error("Operation unsupported")]
    OpUnsupported = 8,
}

impl StatusCode {
    /// The status code for a number received from the peer; unknown numbers become [`StatusCode::Failure`].
    fn from_code(code: u32) -> Self {
        match code {
            0 => Self::Ok,
            1 => Self::Eof,
            2 => Self::NoSuchFile,
            3 => Self::PermissionDenied,
            4 => Self::Failure,
            5 => Self::BadMessage,
            6 => Self::NoConnection,
            7 => Self::ConnectionLost,
            8 => Self::OpUnsupported,
            _ => Self::Failure,
        }
    }
}

impl<'de> Deserialize<'de> for StatusCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        u32::deserialize(deserializer).map(Self::from_code)
    }
}

/// Implementation for SSH_FXP_STATUS as defined in the specification draft
/// <https://datatracker.ietf.org/doc/html/draft-ietf-secsh-filexfer-02#section-7>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub id: u32,
    pub status_code: StatusCode,
    pub error_message: String,
    pub language_tag: String,
}

impl_request_id!(Status);
impl_packet_for!(Status);

#[cfg(test)]
mod tests {
    use bytes::{Buf, Bytes};

    use super::*;

    fn status_packet(code: u32) -> Bytes {
        let mut bytes = Bytes::try_from(Packet::Status(Status {
            id: 7,
            status_code: StatusCode::Failure,
            error_message: "denied".to_owned(),
            language_tag: "en-US".to_owned(),
        }))
        .unwrap()
        .to_vec();
        // Length (4), type (1), id (4), then the code.
        bytes[9..13].copy_from_slice(&code.to_be_bytes());
        Bytes::from(bytes)
    }

    fn decode(code: u32) -> Status {
        let mut bytes = status_packet(code);
        bytes.advance(4);
        match Packet::try_from(&mut bytes).unwrap() {
            Packet::Status(status) => status,
            other => panic!("not a status: {other:?}"),
        }
    }

    #[test]
    fn known_codes_are_read_as_themselves() {
        assert_eq!(decode(0).status_code, StatusCode::Ok);
        assert_eq!(decode(2).status_code, StatusCode::NoSuchFile);
        assert_eq!(decode(8).status_code, StatusCode::OpUnsupported);
    }

    #[test]
    fn unknown_codes_are_read_as_failure_and_keep_the_message() {
        // A UGREEN NAS (UGOS, OpenSSH 9.2) sends 0xFFFFFFFE; later drafts define codes from 9 on.
        for code in [9, 31, 0x100, 0xFFFF_FFFE] {
            let status = decode(code);
            assert_eq!(status.status_code, StatusCode::Failure, "code {code}");
            assert_eq!((status.id, status.error_message.as_str()), (7, "denied"));
        }
    }

    #[test]
    fn codes_are_written_as_before() {
        assert_eq!(&status_packet(3)[9..13], &3u32.to_be_bytes());
        let written = Bytes::try_from(Packet::Status(Status {
            id: 1,
            status_code: StatusCode::OpUnsupported,
            error_message: String::new(),
            language_tag: String::new(),
        }))
        .unwrap();
        assert_eq!(&written[9..13], &8u32.to_be_bytes());
    }
}
