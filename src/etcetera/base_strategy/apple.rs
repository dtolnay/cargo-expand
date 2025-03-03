use std::path::{Path, PathBuf};

use crate::HomeDirError;

/// This is the strategy created by Apple for use on macOS and iOS devices. It is always used by GUI apps on macOS, and is sometimes used by command-line applications there too. iOS only has GUIs, so all iOS applications follow this strategy. The specification is available [here](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/FileSystemProgrammingGuide/FileSystemOverview/FileSystemOverview.html#//apple_ref/doc/uid/TP40010672-CH2-SW1).
///
/// ```
/// use etcetera::base_strategy::Apple;
/// use etcetera::base_strategy::BaseStrategy;
/// use std::path::Path;
///
/// let base_strategy = Apple::new().unwrap();
///
/// let home_dir = etcetera::home_dir().unwrap();
///
/// assert_eq!(
///     base_strategy.home_dir(),
///     &home_dir
/// );
/// assert_eq!(
///     base_strategy.config_dir().strip_prefix(&home_dir),
///     Ok(Path::new("Library/Preferences/"))
/// );
/// assert_eq!(
///     base_strategy.data_dir().strip_prefix(&home_dir),
///     Ok(Path::new("Library/Application Support/"))
/// );
/// assert_eq!(
///     base_strategy.cache_dir().strip_prefix(&home_dir),
///     Ok(Path::new("Library/Caches/"))
/// );
/// assert_eq!(
///     base_strategy.state_dir(),
///     None
/// );
/// assert_eq!(
///     base_strategy.runtime_dir(),
///     None
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Apple {
    home_dir: PathBuf,
}
impl Apple {
    /// Create a new Apple BaseStrategy
    pub fn new() -> Result<Self, HomeDirError> {
        Ok(Self {
            home_dir: crate::home_dir()?,
        })
    }
}

impl super::BaseStrategy for Apple {
    fn home_dir(&self) -> &Path {
        &self.home_dir
    }

    fn config_dir(&self) -> PathBuf {
        self.home_dir.join("Library/Preferences/")
    }

    fn data_dir(&self) -> PathBuf {
        self.home_dir.join("Library/Application Support/")
    }

    fn cache_dir(&self) -> PathBuf {
        self.home_dir.join("Library/Caches/")
    }

    fn state_dir(&self) -> Option<PathBuf> {
        None
    }

    fn runtime_dir(&self) -> Option<PathBuf> {
        None
    }
}
