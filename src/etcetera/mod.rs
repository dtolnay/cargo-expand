use crate::error::{Error, Result};

pub mod base_strategy;

pub use base_strategy::{choose_base_strategy, BaseStrategy};

pub fn home_dir() -> Result<std::path::PathBuf> {
    home::home_dir().ok_or(Error::HomeDir)
}
