use std::{
    ffi::OsString,
    num::{ParseFloatError, ParseIntError},
    path::PathBuf,
    string::{FromUtf8Error, ParseError},
};

use thiserror::Error;

#[derive(Error, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Error {
    #[error("cannot serialize {0}")]
    UnsupportedType(String),
    #[error("i/o error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("{0}")]
    Serde(String),

    #[error("{0}")]
    Deserialize(#[from] DeError),

    #[error("utf8: {0}")]
    Utf8Error(FromUtf8Error),
}

#[derive(Error, Debug)]
pub enum DeError {
    #[error("empty file {0}")]
    EmptyFile(PathBuf),

    #[error("invalid unicode")]
    InvalidUnicode,

    #[error("invalid bool \"{0}\" {1}")]
    InvalidBool(String, PathBuf),

    #[error("parse: {0}")]
    ParseError(String),
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

impl From<ParseIntError> for Error {
    fn from(e: ParseIntError) -> Self {
        DeError::ParseError(e.to_string()).into()
    }
}

impl From<ParseFloatError> for Error {
    fn from(e: ParseFloatError) -> Self {
        DeError::ParseError(e.to_string()).into()
    }
}

pub type Result<T> = std::result::Result<T, Error>;
