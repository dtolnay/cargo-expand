use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let mut version = env!("CARGO_PKG_VERSION").to_owned();
    if let Ok(prettyplease_version) = env::var("DEP_PRETTYPLEASE02_VERSION") {
        // TODO: Make this appear only if `--version --verbose` is used.
        version.push_str(" + prettyplease ");
        version.push_str(&prettyplease_version);
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let version_file = out_dir.join("version");
    fs::write(version_file, version).unwrap();
}
