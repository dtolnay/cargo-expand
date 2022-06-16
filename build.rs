use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let mut version = env!("CARGO_PKG_VERSION").to_owned();
    if cfg!(feature = "prettyplease") {
        if let Ok(prettyplease_version) = env::var("DEP_PRETTYPLEASE01_VERSION") {
            // TODO: Make this appear only if `--version --verbose` is used.
            version.push_str(" + prettyplease ");
            version.push_str(&prettyplease_version);
        }
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let version_file = out_dir.join("version");
    fs::write(version_file, version).unwrap();
}
