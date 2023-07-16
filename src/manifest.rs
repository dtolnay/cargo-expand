use crate::error::Result;
use serde::Deserialize;
use std::env;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

#[derive(Deserialize, Debug)]
pub struct CargoManifest {
    pub package: Option<CargoPackage>,
}

#[derive(Deserialize, Debug)]
pub struct CargoPackage {
    #[serde(rename = "default-run")]
    pub default_run: Option<String>,
}

pub fn parse(manifest_path: Option<&Path>) -> Result<CargoManifest> {
    let manifest_path = find_cargo_manifest(manifest_path)?;
    let content = fs::read_to_string(manifest_path)?;
    let cargo_manifest: CargoManifest = toml::from_str(&content)?;
    Ok(cargo_manifest)
}

fn find_cargo_manifest(manifest_path: Option<&Path>) -> io::Result<PathBuf> {
    if let Some(manifest_path) = manifest_path {
        return Ok(manifest_path.to_owned());
    }

    let dir = env::current_dir()?;
    let mut dir = dir.as_path();
    loop {
        let path = dir.join("Cargo.toml");
        if path.try_exists()? {
            return Ok(path);
        }
        dir = match dir.parent() {
            Some(parent) => parent,
            None => return Err(io::Error::new(ErrorKind::NotFound, "Cargo.toml not found")),
        };
    }
}
