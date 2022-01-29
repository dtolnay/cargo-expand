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
    #[serde(default)]
    pub pager: bool,
}

pub fn deserialize() -> Config {
    try_deserialize().unwrap_or_default()
}

fn try_deserialize() -> Option<Config> {
    let cargo_home = env::var_os("CARGO_HOME").map(PathBuf::from)?;
    let config_names = ["config", "config.toml"];
    let config_path = config_names
        .iter()
        .map(|name| cargo_home.join(name))
        .find(|path| path.exists())?;

    let content = fs::read(&config_path).ok()?;

    let full_config: Sections = match toml::from_slice(&content) {
        Ok(config) => config,
        Err(err) => {
            let _ = writeln!(
                io::stderr(),
                "Warning: {}: {}",
                config_path.display(),
                err
            );
            return None;
        }
    };

    Some(full_config.expand)
}
