use std::fmt::{self, Display};
use std::io;

use prettyprint::PrettyPrintError;

pub enum Error {
    Io(io::Error),
    Print(PrettyPrintError),
    Toml(toml::ser::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io(error)
    }
}

impl From<PrettyPrintError> for Error {
    fn from(error: PrettyPrintError) -> Self {
        Error::Print(error)
    }
}

impl From<toml::ser::Error> for Error {
    fn from(error: toml::ser::Error) -> Self {
        Error::Toml(error)
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        use self::Error::*;

        match self {
            Io(e) => e.fmt(formatter),
            Print(e) => e.fmt(formatter),
            Toml(e) => e.fmt(formatter),
        }
    }
}
