use crate::serial::Error as SerialError;
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error(transparent)]
    SerialError(#[from] SerialError),
    #[error("module returned error {0}")]
    CommandError(String),
    #[error("failed to scan pan: {0}")]
    ScanError(String),
    #[error("timeout")]
    TimeoutError(),
}

pub type Result<T> = std::result::Result<T, Error>;
