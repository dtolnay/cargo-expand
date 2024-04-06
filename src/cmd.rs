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

    pub fn arg<S>(&mut self, arg: S)
    where
        S: AsRef<OsStr>,
    {
        self.args.push(arg.as_ref().to_owned());
    }

    pub fn flag_value<K, V>(&mut self, k: K, v: V)
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        let k = k.as_ref();
        let v = v.as_ref();
        if let Some(k) = k.to_str() {
            if let Some(v) = v.to_str() {
                self.arg(format!("{}={}", k, v));
                return;
            }
        }
        self.arg(k);
        self.arg(v);
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
