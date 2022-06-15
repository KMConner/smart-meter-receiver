use crate::serial::Error as SerialError;
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error(transparent)]
    SerialError(#[from] SerialError),
}

pub type Result<T> = std::result::Result<T, Error>;