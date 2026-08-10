use anyhow::anyhow;
use clap::parser::MatchesError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Unknown(anyhow!(value))
    }
}

impl From<toml::de::Error> for Error {
    fn from(value: toml::de::Error) -> Self {
        Error::Unknown(anyhow!(value))
    }
}

impl From<prometheus::Error> for Error {
    fn from(value: prometheus::Error) -> Self {
        Error::Unknown(anyhow!(value))
    }
}

impl From<MatchesError> for Error {
    fn from(value: MatchesError) -> Self {
        Error::Unknown(anyhow!(value))
    }
}

impl From<std::env::VarError> for Error {
    fn from(value: std::env::VarError) -> Self {
        Error::Unknown(anyhow!(value))
    }
}

pub type Result<T> = std::result::Result<T, Error>;
