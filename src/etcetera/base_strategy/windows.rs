use std::path::{Path, PathBuf};

use crate::HomeDirError;

/// This strategy follows Windows’ conventions. It seems that all Windows GUI apps, and some command-line ones follow this pattern. The specification is available [here](https://docs.microsoft.com/en-us/windows/win32/shell/knownfolderid).
///
/// This initial example removes all the relevant environment variables to show the strategy’s use of the:
/// - (on Windows) SHGetKnownFolderPath API.
/// - (on non-Windows) Windows default directories.
///
/// ```
/// use etcetera::base_strategy::BaseStrategy;
/// use etcetera::base_strategy::Windows;
/// use std::path::Path;
///
/// // Remove the environment variables that the strategy reads from.
/// std::env::remove_var("USERPROFILE");
/// std::env::remove_var("APPDATA");
/// std::env::remove_var("LOCALAPPDATA");
///
/// let base_strategy = Windows::new().unwrap();
///
/// let home_dir = etcetera::home_dir().unwrap();
///
/// assert_eq!(
///     base_strategy.home_dir(),
///     &home_dir
/// );
/// assert_eq!(
///     base_strategy.config_dir().strip_prefix(&home_dir),
///     Ok(Path::new("AppData/Roaming/"))
/// );
/// assert_eq!(
///     base_strategy.data_dir().strip_prefix(&home_dir),
///     Ok(Path::new("AppData/Roaming/"))
/// );
/// assert_eq!(
///     base_strategy.cache_dir().strip_prefix(&home_dir),
///     Ok(Path::new("AppData/Local/"))
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
///
/// This next example gives the environment variables values:
///
/// ```
/// use etcetera::base_strategy::BaseStrategy;
/// use etcetera::base_strategy::Windows;
/// use std::path::Path;
///
/// let home_path = if cfg!(windows) {
///     "C:\\foo\\".to_string()
/// } else {
///     etcetera::home_dir().unwrap().to_string_lossy().to_string()
/// };
/// let data_path = if cfg!(windows) {
///     "C:\\bar\\"
/// } else {
///     "/bar/"
/// };
/// let cache_path = if cfg!(windows) {
///     "C:\\baz\\"
/// } else {
///     "/baz/"
/// };
///
/// std::env::set_var("USERPROFILE", &home_path);
/// std::env::set_var("APPDATA", data_path);
/// std::env::set_var("LOCALAPPDATA", cache_path);
///
/// let base_strategy = Windows::new().unwrap();
///
/// assert_eq!(
///     base_strategy.home_dir(),
///     Path::new(&home_path)
/// );
/// assert_eq!(
///     base_strategy.config_dir(),
///     Path::new(data_path)
/// );
/// assert_eq!(
///     base_strategy.data_dir(),
///     Path::new(data_path)
/// );
/// assert_eq!(
///     base_strategy.cache_dir(),
///     Path::new(cache_path)
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
pub struct Windows {
    home_dir: PathBuf,
}

impl Windows {
    /// Create a new Windows BaseStrategy
    pub fn new() -> Result<Self, HomeDirError> {
        Ok(Self {
            home_dir: crate::home_dir()?,
        })
    }

    fn dir_inner(env: &'static str) -> Option<PathBuf> {
        std::env::var_os(env)
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .or_else(|| Self::dir_crt(env))
    }

    // Ref: https://github.com/rust-lang/cargo/blob/home-0.5.11/crates/home/src/windows.rs
    // We should keep this code in sync with the above.
    #[cfg(all(windows, not(target_vendor = "uwp")))]
    fn dir_crt(env: &'static str) -> Option<PathBuf> {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        use std::ptr;
        use std::slice;

        use windows_sys::Win32::Foundation::S_OK;
        use windows_sys::Win32::System::Com::CoTaskMemFree;
        use windows_sys::Win32::UI::Shell::{
            FOLDERID_LocalAppData, FOLDERID_RoamingAppData, SHGetKnownFolderPath,
            KF_FLAG_DONT_VERIFY,
        };

        extern "C" {
            fn wcslen(buf: *const u16) -> usize;
        }

        let folder_id = match env {
            "APPDATA" => FOLDERID_RoamingAppData,
            "LOCALAPPDATA" => FOLDERID_LocalAppData,
            _ => return None,
        };

        unsafe {
            let mut path = ptr::null_mut();
            match SHGetKnownFolderPath(
                &folder_id,
                KF_FLAG_DONT_VERIFY as u32,
                std::ptr::null_mut(),
                &mut path,
            ) {
                S_OK => {
                    let path_slice = slice::from_raw_parts(path, wcslen(path));
                    let s = OsString::from_wide(path_slice);
                    CoTaskMemFree(path.cast());
                    Some(PathBuf::from(s))
                }
                _ => {
                    // Free any allocated memory even on failure. A null ptr is a no-op for `CoTaskMemFree`.
                    CoTaskMemFree(path.cast());
                    None
                }
            }
        }
    }

    #[cfg(not(all(windows, not(target_vendor = "uwp"))))]
    fn dir_crt(_env: &'static str) -> Option<PathBuf> {
        None
    }
}

impl super::BaseStrategy for Windows {
    fn home_dir(&self) -> &Path {
        &self.home_dir
    }

    fn config_dir(&self) -> PathBuf {
        self.data_dir()
    }

    fn data_dir(&self) -> PathBuf {
        Self::dir_inner("APPDATA").unwrap_or_else(|| self.home_dir.join("AppData").join("Roaming"))
    }

    fn cache_dir(&self) -> PathBuf {
        Self::dir_inner("LOCALAPPDATA")
            .unwrap_or_else(|| self.home_dir.join("AppData").join("Local"))
    }

    fn state_dir(&self) -> Option<PathBuf> {
        None
    }

    fn runtime_dir(&self) -> Option<PathBuf> {
        None
    }
}
