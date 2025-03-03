//! These strategies simply provide the user’s configuration, data, and cache directories, without knowing about the application specifically.

use crate::HomeDirError;
use std::path::{Path, PathBuf};

/// Provides configuration, data, and cache directories of the current user.
pub trait BaseStrategy {
    /// Gets the home directory of the current user.
    fn home_dir(&self) -> &Path;

    /// Gets the user’s configuration directory.
    fn config_dir(&self) -> PathBuf;

    /// Gets the user’s data directory.
    fn data_dir(&self) -> PathBuf;

    /// Gets the user’s cache directory.
    fn cache_dir(&self) -> PathBuf;

    /// Gets the user’s state directory.
    /// Currently, only the [`Xdg`](struct.Xdg.html) strategy supports this.
    fn state_dir(&self) -> Option<PathBuf>;

    /// Gets the user’s runtime directory.
    /// Currently, only the [`Xdg`](struct.Xdg.html) strategy supports this.
    ///
    /// Note: The [XDG Base Directory Specification](spec) places additional requirements on this
    /// directory related to ownership, permissions, and persistence. This library does not check
    /// these requirements.
    ///
    /// [spec]: https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html
    fn runtime_dir(&self) -> Option<PathBuf>;
}

macro_rules! create_strategies {
    ($native: ty, $base: ty) => {
        /// Returns the current OS’s native [`BaseStrategy`](trait.BaseStrategy.html).
        /// This uses the [`Windows`](struct.Windows.html) strategy on Windows, [`Apple`](struct.Apple.html) on macOS & iOS, and [`Xdg`](struct.Xdg.html) everywhere else.
        /// This is the convention used by most GUI applications.
        pub fn choose_native_strategy() -> Result<$native, HomeDirError> {
            <$native>::new()
        }

        /// Returns the current OS’s default [`BaseStrategy`](trait.BaseStrategy.html).
        /// This uses the [`Windows`](struct.Windows.html) strategy on Windows, and [`Xdg`](struct.Xdg.html) everywhere else.
        /// This is the convention used by most CLI applications.
        pub fn choose_base_strategy() -> Result<$base, HomeDirError> {
            <$base>::new()
        }
    };
}

cfg_if::cfg_if! {
    if #[cfg(target_os = "windows")] {
        create_strategies!(Windows, Windows);
    } else if #[cfg(any(target_os = "macos", target_os = "ios"))] {
        create_strategies!(Apple, Xdg);
    } else {
        create_strategies!(Xdg, Xdg);
    }
}

mod apple;
mod windows;
mod xdg;

pub use apple::Apple;
pub use windows::Windows;
pub use xdg::Xdg;
