use std::path::Path;
use std::path::PathBuf;

use crate::etcetera::HomeDirError;

pub struct Xdg {
    home_dir: PathBuf,
}

impl Xdg {
    pub fn new() -> Result<Self, HomeDirError> {
        Ok(Self {
            home_dir: crate::etcetera::home_dir()?,
        })
    }

    fn env_var_or_none(env_var: &str) -> Option<PathBuf> {
        std::env::var(env_var).ok().and_then(|path| {
            let path = PathBuf::from(path);

            // Return None if the path obtained from the environment variable isn’t absolute.
            if path.is_absolute() {
                Some(path)
            } else {
                None
            }
        })
    }

    fn env_var_or_default(&self, env_var: &str, default: impl AsRef<Path>) -> PathBuf {
        Self::env_var_or_none(env_var).unwrap_or_else(|| self.home_dir.join(default))
    }
}

impl super::BaseStrategy for Xdg {
    fn cache_dir(&self) -> PathBuf {
        self.env_var_or_default("XDG_CACHE_HOME", ".cache/")
    }
}
