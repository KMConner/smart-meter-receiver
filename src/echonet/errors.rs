use std::array::TryFromSliceError;
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("failed to parse binary data {0}")]
    ParseError(String),

    #[error("unknown value: {0}")]
    InvalidValueError(String),

    #[error("invalid echonet object id: {0}")]
    InvalidEchonetObjectIdError(String),

    #[error("invalid echonet service id: {0}")]
    InvalidEchonetServiceError(u8),

    #[error("invalid echonet property id: {0}")]
    InvalidEchonetProperty(u8),
}

impl From<TryFromSliceError> for Error {
    fn from(e: TryFromSliceError) -> Error {
        Error::ParseError(format!("failed to convert into slice: {}", e))
    }
}

pub type Result<T> = std::result::Result<T, Error>;
