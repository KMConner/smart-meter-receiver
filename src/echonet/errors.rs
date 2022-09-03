use std::array::TryFromSliceError;
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("failed to parse binary data {0}")]
    ParseError(String),
}

impl From<TryFromSliceError> for Error {
    fn from(e: TryFromSliceError) -> Error {
        Error::ParseError(format!("failed to convert into slice: {}", e))
    }
}

pub type Result<T> = std::result::Result<T, Error>;
