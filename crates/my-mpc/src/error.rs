use std::fmt;

#[derive(Debug)]
pub enum Error {
    ChannelClosed,
    // Add other errors as needed
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ChannelClosed => write!(f, "Channel closed"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
