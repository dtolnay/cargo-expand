use std::ffi::OsStr;
use std::process::Command;

pub trait CommandExt {
    fn flag_value<K, V>(&mut self, k: K, v: V)
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>;
}

impl CommandExt for Command {
    fn flag_value<K, V>(&mut self, k: K, v: V)
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
}
