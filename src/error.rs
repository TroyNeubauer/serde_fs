use std::{
    num::{ParseFloatError, ParseIntError},
    path::PathBuf,
    string::FromUtf8Error,
};

use thiserror::Error;

#[derive(Error, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum SerError {
    #[error("cannot serialize {0}")]
    UnsupportedType(String),

    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("{0}")]
    Serde(String),

    #[error("utf8: {0}")]
    Utf8Error(FromUtf8Error),
}

#[derive(Error, Debug)]
pub enum DeError {
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("empty file {0}")]
    EmptyFile(PathBuf),

    #[error("empty dir {0}")]
    EmptyDirectory(PathBuf),

    #[error("symlinks are not allowed {0}")]
    EncounteredSymlink(PathBuf),

    #[error("invalid unicode")]
    InvalidUnicode,

    #[error("invalid bool \"{0}\" {1}")]
    InvalidBool(String, PathBuf),

    #[error("parse: {0}")]
    ParseError(String),

    #[error("{0}")]
    Serde(String),
}

impl serde::ser::Error for SerError {
    fn custom<T>(t: T) -> Self
    where
        T: std::fmt::Display,
    {
        SerError::Serde(t.to_string())
    }
}

impl serde::de::Error for DeError {
    fn custom<T>(t: T) -> Self
    where
        T: std::fmt::Display,
    {
        DeError::Serde(t.to_string())
    }
}

impl From<ParseIntError> for DeError {
    fn from(e: ParseIntError) -> Self {
        DeError::ParseError(e.to_string()).into()
    }
}

impl From<ParseFloatError> for DeError {
    fn from(e: ParseFloatError) -> Self {
        DeError::ParseError(e.to_string()).into()
    }
}
