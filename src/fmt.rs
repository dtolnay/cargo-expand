use crate::error::Result;
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Serialize)]
struct Rustfmt {
    normalize_doc_attributes: bool,
    reorder_imports: bool,
    reorder_modules: bool,
}

pub fn write_rustfmt_config(outdir: impl AsRef<Path>) -> Result<()> {
    let config = Rustfmt {
        normalize_doc_attributes: true,
        reorder_imports: false,
        reorder_modules: false,
    };

    let toml_string = toml::to_string(&config)?;

    let rustfmt_config_path = outdir.as_ref().join("rustfmt.toml");
    fs::write(rustfmt_config_path, toml_string)?;

    Ok(())
}
