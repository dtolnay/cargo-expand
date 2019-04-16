use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::error::Result;

#[derive(Serialize)]
struct Rustfmt {
    normalize_doc_attributes: bool,
}

pub fn write_rustfmt_config(outdir: impl AsRef<Path>) -> Result<()> {
    let config = Rustfmt {
        normalize_doc_attributes: true,
    };

    let toml_string = toml::to_string(&config)?;

    let rustfmt_config_path = outdir.as_ref().join("rustfmt.toml");
    fs::write(rustfmt_config_path, toml_string)?;

    Ok(())
}
