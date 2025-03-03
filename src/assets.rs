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

    let cache_dir = if cfg!(windows) {
        windows::cache_dir(&home_dir)
    } else {
        xdg::cache_dir(&home_dir)
    };

    Ok(cache_dir.join("bat"))
}

mod windows {
    use std::path::{Path, PathBuf};

    fn dir_inner(env: &'static str) -> Option<PathBuf> {
        std::env::var_os(env)
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .or_else(|| dir_crt(env))
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

    pub fn cache_dir(home_dir: &Path) -> PathBuf {
        dir_inner("LOCALAPPDATA").unwrap_or_else(|| home_dir.join("AppData").join("Local"))
    }
}

mod xdg {
    use std::path::{Path, PathBuf};

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

    fn env_var_or_default(home_dir: &Path, env_var: &str, default: impl AsRef<Path>) -> PathBuf {
        env_var_or_none(env_var).unwrap_or_else(|| home_dir.join(default))
    }

    pub fn cache_dir(home_dir: &Path) -> PathBuf {
        env_var_or_default(home_dir, "XDG_CACHE_HOME", ".cache/")
    }
}
