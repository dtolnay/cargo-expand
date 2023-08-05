use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let prettyplease_version = match env::var("DEP_PRETTYPLEASE02_VERSION") {
        Ok(prettyplease_version) => format!(r#"Some("{}")"#, prettyplease_version.escape_debug()),
        Err(_) => "None".to_owned(),
    };

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let prettyplease_version_file = out_dir.join("prettyplease.version");
    fs::write(prettyplease_version_file, prettyplease_version).unwrap();
}
