pub mod base_strategy;

pub use base_strategy::{choose_base_strategy, BaseStrategy};

pub fn home_dir() -> Result<std::path::PathBuf, HomeDirError> {
    home::home_dir().ok_or(HomeDirError)
}

#[derive(Debug)]
pub struct HomeDirError;

impl std::fmt::Display for HomeDirError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "could not locate home directory")
    }
}

impl std::error::Error for HomeDirError {}
