use serde::Deserialize;

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Deserialize)]
struct Sections {
    #[serde(default)]
    expand: Config,
}

#[derive(Deserialize, Default)]
pub struct Config {
    pub theme: Option<String>,
    pub color: Option<String>,
    #[serde(default = "bool::default")]
    pub pager: bool,
}

pub fn deserialize() -> Config {
    try_deserialize().unwrap_or_default()
}

fn try_deserialize() -> Option<Config> {
    let cargo_home = env::var_os("CARGO_HOME")?;
    let config_path = PathBuf::from(cargo_home).join("config");
    if !config_path.exists() {
        return None;
    }

    let content = fs::read(&config_path).ok()?;

    let full_config: Sections = match toml::from_slice(&content) {
        Ok(config) => config,
        Err(err) => {
            let _ = writeln!(
                &mut io::stderr(),
                "Warning: {}: {}",
                config_path.display(),
                err
            );
            return None;
        }
    };

    Some(full_config.expand)
}
