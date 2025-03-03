use std::path::PathBuf;

use crate::etcetera::HomeDirError;

pub struct Windows {
    home_dir: PathBuf,
}

impl Windows {
    pub fn new() -> Result<Self, HomeDirError> {
        Ok(Self {
            home_dir: crate::etcetera::home_dir()?,
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
    fn cache_dir(&self) -> PathBuf {
        Self::dir_inner("LOCALAPPDATA")
            .unwrap_or_else(|| self.home_dir.join("AppData").join("Local"))
    }
}
