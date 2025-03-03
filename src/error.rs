use crate::etcetera;
use std::error::Error as StdError;
use std::fmt::{self, Display};
use std::io;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    TomlSer(toml::ser::Error),
    TomlDe(toml::de::Error),
    Quote(shlex::QuoteError),
    HomeDir(etcetera::HomeDirError),
    Bat(bat::error::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io(error)
    }
}

impl From<toml::ser::Error> for Error {
    fn from(error: toml::ser::Error) -> Self {
        Error::TomlSer(error)
    }
}

impl From<toml::de::Error> for Error {
    fn from(error: toml::de::Error) -> Self {
        Error::TomlDe(error)
    }
}

impl From<shlex::QuoteError> for Error {
    fn from(error: shlex::QuoteError) -> Self {
        Error::Quote(error)
    }
}

impl From<etcetera::HomeDirError> for Error {
    fn from(error: etcetera::HomeDirError) -> Self {
        Error::HomeDir(error)
    }
}

impl From<bat::error::Error> for Error {
    fn from(error: bat::error::Error) -> Self {
        Error::Bat(error)
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Io(e) => e.fmt(formatter),
            Error::TomlSer(e) => e.fmt(formatter),
            Error::TomlDe(e) => e.fmt(formatter),
            Error::Quote(e) => e.fmt(formatter),
            Error::HomeDir(e) => e.fmt(formatter),
            Error::Bat(e) => e.fmt(formatter),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::Io(e) => e.source(),
            Error::TomlSer(e) => e.source(),
            Error::TomlDe(e) => e.source(),
            Error::Quote(e) => e.source(),
            Error::HomeDir(e) => e.source(),
            Error::Bat(e) => e.source(),
        }
    }
}
