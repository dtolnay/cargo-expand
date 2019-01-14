use std::fmt::{self, Display};
use std::io;

use prettyprint::PrettyPrintError;

pub enum Error {
    Io(io::Error),
    Print(PrettyPrintError),
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

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Io(error) => Display::fmt(error, formatter),
            Error::Print(error) => Display::fmt(error, formatter),
        }
    }
}
