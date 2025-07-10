use crate::protocol::StatusCode;

#[derive(Debug, Clone, PartialEq)]
pub struct HandlerError {
    pub status_code: StatusCode,
    pub error_message: String,
    pub language_tag: String,
}
