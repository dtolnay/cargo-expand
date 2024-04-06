use std::ffi::{OsStr, OsString};

#[derive(Clone)]
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

    pub fn insert<S: AsRef<OsStr>>(&mut self, index: usize, arg: S) {
        self.args.insert(index, arg.as_ref().to_owned());
    }
}

impl IntoIterator for CommandArgs {
    type Item = OsString;
    type IntoIter = <Vec<OsString> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.args.into_iter()
    }
}
