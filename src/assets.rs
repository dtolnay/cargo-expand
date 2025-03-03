use crate::error::Result;
use crate::etcetera::{self, BaseStrategy as _};
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

    let basedirs = etcetera::choose_base_strategy()?;
    Ok(basedirs.cache_dir().join("bat"))
}
