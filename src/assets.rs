use crate::error::{Error, Result};
use std::env;
use std::path::PathBuf;
use std::str;

pub const BAT_VERSION: &str = {
    let mut bytes = include_str!("bat.version").as_bytes();
    while let [rest @ .., b'\n' | b'\r'] = bytes {
        bytes = rest;
    }
    if let Ok(version) = str::from_utf8(bytes) {
        version
    } else {
        panic!()
    }
};

pub fn cache_dir() -> Result<PathBuf> {
    if let Some(cache_dir) = env::var_os("BAT_CACHE_PATH") {
        return Ok(PathBuf::from(cache_dir));
    }

    let home_dir = home::home_dir().ok_or(Error::HomeDir)?;

    #[cfg(windows)]
    let cache_dir = windows::cache_dir(&home_dir);
    #[cfg(not(windows))]
    let cache_dir = xdg::cache_dir(&home_dir);

    Ok(cache_dir.join("bat"))
}

// Based on etcetera v0.9.0
#[cfg(windows)]
mod windows {
    use std::env;
    use std::path::{Path, PathBuf};

    pub fn cache_dir(home_dir: &Path) -> PathBuf {
        env::var_os("LOCALAPPDATA")
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .or_else(dir_crt)
            .unwrap_or_else(|| home_dir.join("AppData").join("Local"))
    }

    // Ref: https://github.com/rust-lang/cargo/blob/home-0.5.11/crates/home/src/windows.rs
    // We should keep this code in sync with the above.
    #[cfg(not(target_vendor = "uwp"))]
    fn dir_crt() -> Option<PathBuf> {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        use std::ptr;
        use std::slice;
        use windows_sys::Win32::Foundation::S_OK;
        use windows_sys::Win32::System::Com::CoTaskMemFree;
        use windows_sys::Win32::UI::Shell::{
            FOLDERID_LocalAppData, SHGetKnownFolderPath, KF_FLAG_DONT_VERIFY,
        };

        extern "C" {
            fn wcslen(buf: *const u16) -> usize;
        }

        let mut path = ptr::null_mut();
        let S_OK = (unsafe {
            SHGetKnownFolderPath(
                &FOLDERID_LocalAppData,
                KF_FLAG_DONT_VERIFY as u32,
                ptr::null_mut(),
                &mut path,
            )
        }) else {
            // Free any allocated memory even on failure. A null ptr is a no-op for `CoTaskMemFree`.
            unsafe { CoTaskMemFree(path.cast()) };
            return None;
        };

        let path_slice = unsafe { slice::from_raw_parts(path, wcslen(path)) };
        let s = OsString::from_wide(path_slice);
        unsafe { CoTaskMemFree(path.cast()) };
        Some(PathBuf::from(s))
    }

    #[cfg(target_vendor = "uwp")]
    fn dir_crt() -> Option<PathBuf> {
        None
    }
}

// Based on etcetera v0.9.0
#[cfg(not(windows))]
mod xdg {
    use std::env;
    use std::path::{Path, PathBuf};

    pub fn cache_dir(home_dir: &Path) -> PathBuf {
        env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .unwrap_or_else(|| home_dir.join(".cache/"))
    }
}
