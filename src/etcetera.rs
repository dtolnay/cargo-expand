use crate::error::{Error, Result};

pub mod base_strategy {
    use crate::error::Result;
    use std::path::PathBuf;

    pub trait BaseStrategy {
        fn cache_dir(&self) -> PathBuf;
    }

    macro_rules! create_strategies {
        ($base: ty) => {
            pub fn choose_base_strategy() -> Result<$base> {
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

    mod windows {
        use crate::error::Result;
        use std::path::PathBuf;

        pub struct Windows {
            home_dir: PathBuf,
        }

        impl Windows {
            pub fn new() -> Result<Self> {
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
    }

    mod xdg {
        use crate::error::Result;
        use std::path::Path;
        use std::path::PathBuf;

        pub struct Xdg {
            home_dir: PathBuf,
        }

        impl Xdg {
            pub fn new() -> Result<Self> {
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
    }

    pub use windows::Windows;
    pub use xdg::Xdg;
}

pub use base_strategy::{choose_base_strategy, BaseStrategy};

pub fn home_dir() -> Result<std::path::PathBuf> {
    home::home_dir().ok_or(Error::HomeDir)
}
