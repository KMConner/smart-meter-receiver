use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("failed to parse binary data {0}")]
    ParseError(String),
}

pub type Result<T> = std::result::Result<T, Error>;
