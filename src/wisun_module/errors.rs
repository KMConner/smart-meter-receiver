use crate::serial::Error as SerialError;
use thiserror::Errpr as ThisError;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error(transparent)]
    SerialError(SerialError),
}

pub type Result<T> = std::result::Result<T, Error>;
