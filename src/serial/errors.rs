use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("device is not available: {0}")]
    NoDevice(String),

    #[error("a parameter was incorrect: {0}")]
    InvalidInput(String),

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    Utf8Error(#[from] std::string::FromUtf8Error),

    #[error("unknown error: {0}")]
    UnknownError(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<serialport::Error> for Error {
    fn from(e: serialport::Error) -> Self {
        match &e.kind {
            serialport::ErrorKind::InvalidInput => Error::InvalidInput(e.description),
            serialport::ErrorKind::NoDevice => Error::NoDevice(e.description),
            serialport::ErrorKind::Unknown => Error::UnknownError(e.description),
            serialport::ErrorKind::Io(ek) => Error::IoError(std::io::Error::new(ek.clone(), e.description)),
        }
    }
}
