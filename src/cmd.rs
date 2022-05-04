use std::ffi::{OsStr, OsString};
use std::fmt::{self, Display};

#[derive(Clone)]
pub struct Line {
    bin: OsString,
    args: Vec<OsString>,
}

impl Line {
    pub fn new<S: AsRef<OsStr>>(bin: S) -> Self {
        Line {
            bin: bin.as_ref().to_owned(),
            args: Vec::new(),
        }
    }

    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) {
        self.args.push(arg.as_ref().to_owned());
    }

    pub fn args<I, S>(&mut self, args: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.args
            .extend(args.into_iter().map(|arg| arg.as_ref().to_owned()));
    }

    pub fn insert<S: AsRef<OsStr>>(&mut self, index: usize, arg: S) {
        self.args.insert(index, arg.as_ref().to_owned());
    }
}

impl Display for Line {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}", self.bin.to_string_lossy())?;
        for arg in &self.args {
            write!(formatter, " {}", arg.to_string_lossy())?;
        }
        Ok(())
    }
}

impl IntoIterator for Line {
    type Item = OsString;
    type IntoIter = <Vec<OsString> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.args.into_iter()
    }
}
