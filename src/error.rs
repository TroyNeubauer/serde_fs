use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot serialize {0}")]
    UnsupportedType(String),
    #[error("i/o error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("{0}")]
    Serde(String),
}

impl serde::ser::Error for Error {
    fn custom<T>(t: T) -> Self
    where
        T: std::fmt::Display,
    {
        Error::Serde(t.to_string())
    }
}

impl serde::de::Error for Error {
    fn custom<T>(t: T) -> Self
    where
        T: std::fmt::Display,
    {
        Error::Serde(t.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
