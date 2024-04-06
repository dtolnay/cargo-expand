use std::ffi::{OsStr, OsString};
use std::slice;
use std::vec;

pub struct CommandArgs {
    args: Vec<OsString>,
}

impl CommandArgs {
    pub fn new() -> Self {
        CommandArgs { args: Vec::new() }
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
}

impl IntoIterator for CommandArgs {
    type Item = OsString;
    type IntoIter = vec::IntoIter<OsString>;

    fn into_iter(self) -> Self::IntoIter {
        self.args.into_iter()
    }
}

impl<'a> IntoIterator for &'a CommandArgs {
    type Item = &'a OsString;
    type IntoIter = slice::Iter<'a, OsString>;

    fn into_iter(self) -> Self::IntoIter {
        self.args.iter()
    }
}
