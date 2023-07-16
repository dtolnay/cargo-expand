use std::fmt::{self, Display};
use std::io;

pub enum Error {
    Io(io::Error),
    TomlSer(toml::ser::Error),
    TomlDe(toml::de::Error),
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

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        use self::Error::*;

        match self {
            Io(e) => e.fmt(formatter),
            TomlSer(e) => e.fmt(formatter),
            TomlDe(e) => e.fmt(formatter),
        }
    }
}
