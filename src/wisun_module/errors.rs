use crate::serial::Error as SerialError;
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error(transparent)]
    SerialError(#[from] SerialError),
    #[error("module returned error {0}")]
    CommandError(String),
}

pub type Result<T> = std::result::Result<T, Error>;