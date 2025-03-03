use crate::etcetera::HomeDirError;
use std::path::PathBuf;

pub trait BaseStrategy {
    fn cache_dir(&self) -> PathBuf;
}

macro_rules! create_strategies {
    ($base: ty) => {
        pub fn choose_base_strategy() -> Result<$base, HomeDirError> {
            <$base>::new()
        }
    };
}

cfg_if::cfg_if! {
    if #[cfg(target_os = "windows")] {
        create_strategies!(Windows);
    } else if #[cfg(any(target_os = "macos", target_os = "ios"))] {
        create_strategies!(Xdg);
    } else {
        create_strategies!(Xdg);
    }
}

mod windows;
mod xdg;

pub use windows::Windows;
pub use xdg::Xdg;
