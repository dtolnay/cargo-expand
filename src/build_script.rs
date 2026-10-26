use crate::cmd::CommandExt as _;
use crate::error::Result;
use crate::opts::Expand;
use serde::Deserialize;
use std::env;
use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const DISCOVER: &str = "CARGO_EXPAND_DISCOVER_WRAPPER";
pub const SCRIPT: &str = "CARGO_EXPAND_BUILD_SCRIPT";
const MANIFEST_DIR: &str = "CARGO_EXPAND_BUILD_MANIFEST_DIR";
const OUTPUT: &str = "CARGO_EXPAND_BUILD_OUTPUT";
const STATUS: &str = "CARGO_EXPAND_BUILD_STATUS";
const WORKSPACE_WRAPPER: &str = "CARGO_EXPAND_WORKSPACE_WRAPPER";

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_default_members: Vec<String>,
    workspace_members: Vec<String>,
}

#[derive(Deserialize)]
struct Package {
    id: String,
    name: String,
    manifest_path: PathBuf,
    targets: Vec<Target>,
}

#[derive(Deserialize)]
struct Target {
    kind: Vec<String>,
    src_path: PathBuf,
}

fn cargo_command(subcommand: &str, args: &Expand) -> Command {
    let mut cmd = Command::new(crate::cargo_binary());
    cmd.arg(subcommand);
    for config in &args.config {
        cmd.flag_value("--config", config);
    }
    for flag in &args.unstable_flags {
        cmd.arg(format!("-Z{}", flag));
    }
    if let Some(path) = &args.manifest_path {
        cmd.flag_value("--manifest-path", path);
    }
    if args.offline {
        cmd.arg("--offline");
    }
    if args.locked {
        cmd.arg("--locked");
    }
    if args.frozen {
        cmd.arg("--frozen");
    }
    cmd
}

pub fn configure(cmd: &mut Command, args: &Expand, output: &Path, status: &Path) -> Result<()> {
    let mut metadata = cargo_command("metadata", args);
    metadata.arg("--format-version=1");
    for features in &args.features {
        metadata.flag_value("--features", features);
    }
    if args.all_features {
        metadata.arg("--all-features");
    }
    if args.no_default_features {
        metadata.arg("--no-default-features");
    }
    let metadata_output = metadata.output()?;
    if !metadata_output.status.success() {
        io::stderr().write_all(&metadata_output.stderr)?;
        return Err(io::Error::other("failed to read Cargo metadata").into());
    }
    let metadata: Metadata = serde_json::from_slice(&metadata_output.stdout)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

    let package_id = if let Some(package) = &args.package {
        let Some(spec) = package else {
            return Err(io::Error::other("--build requires a package name after --package").into());
        };
        // Let Cargo resolve package ID specifications, including name@version.
        let mut pkgid = cargo_command("pkgid", args);
        pkgid.flag_value("--package", spec);
        let output = pkgid.output()?;
        if !output.status.success() {
            io::stderr().write_all(&output.stderr)?;
            return Err(io::Error::other("failed to select build-script package").into());
        }
        String::from_utf8(output.stdout)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?
            .trim()
            .to_owned()
    } else if let [id] = metadata.workspace_default_members.as_slice() {
        id.clone()
    } else {
        return Err(io::Error::other(
            "--build requires --package when multiple workspace packages are selected",
        )
        .into());
    };
    if !metadata.workspace_members.contains(&package_id) {
        return Err(io::Error::other("--build requires a workspace package").into());
    }
    let package = metadata
        .packages
        .iter()
        .find(|package| package.id == package_id)
        .ok_or_else(|| io::Error::other("--build requires a workspace package"))?;
    let script = package
        .targets
        .iter()
        .find(|target| target.kind.iter().any(|kind| kind == "custom-build"))
        .ok_or_else(|| {
            io::Error::other(format!("package `{}` has no build script", package.name))
        })?;

    // Discover the configured wrapper before replacing it for this expansion.
    let wrapper_path = status.with_extension("wrapper");
    let mut probe = cargo_command("check", args);
    probe.env("CARGO_CACHE_RUSTC_INFO", "0");
    probe.env("RUSTC_WRAPPER", env::current_exe()?);
    probe.env(DISCOVER, &wrapper_path);
    let result = probe.output()?;
    if !wrapper_path.exists() {
        io::stderr().write_all(&result.stderr)?;
        return Err(io::Error::other("failed to discover Cargo's compiler wrapper").into());
    }
    let wrapper: Option<PathBuf> = serde_json::from_slice(&fs_err::read(&wrapper_path)?)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    if let Some(wrapper) = wrapper {
        cmd.env(WORKSPACE_WRAPPER, wrapper);
    } else {
        cmd.env_remove(WORKSPACE_WRAPPER);
    }
    // Cargo hashes the workspace wrapper path. Use a unique path so a script
    // compiled as another expansion's dependency cannot bypass interception.
    let executable = env::current_exe()?;
    let wrapper = status
        .parent()
        .unwrap()
        .join(format!("rustc-wrapper{}", env::consts::EXE_SUFFIX));
    if fs_err::hard_link(&executable, &wrapper).is_err() {
        fs_err::copy(&executable, &wrapper)?;
    }
    cmd.env("RUSTC_WORKSPACE_WRAPPER", wrapper);
    cmd.env(SCRIPT, script.src_path.canonicalize()?);
    cmd.env(
        MANIFEST_DIR,
        package.manifest_path.parent().unwrap().canonicalize()?,
    );
    cmd.env(OUTPUT, output);
    cmd.env(STATUS, status);
    Ok(())
}

pub fn rustc_wrapper() -> Result<i32> {
    let mut rustc_args = env::args_os().skip(1);
    let rustc = rustc_args
        .next()
        .ok_or_else(|| io::Error::other("missing rustc executable"))?;
    let args: Vec<OsString> = rustc_args.collect();
    let script = PathBuf::from(env::var_os(SCRIPT).unwrap());
    let manifest_dir = PathBuf::from(env::var_os(MANIFEST_DIR).unwrap());
    let selected_package = env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .and_then(|path| path.canonicalize().ok())
        .is_some_and(|path| path == manifest_dir);
    let selected = selected_package
        && args.iter().any(|arg| {
            Path::new(arg)
                .canonicalize()
                .is_ok_and(|path| path == script)
        });
    let mut cmd = if let Some(wrapper) = env::var_os(WORKSPACE_WRAPPER) {
        let mut cmd = Command::new(wrapper);
        cmd.arg(rustc);
        cmd
    } else {
        Command::new(rustc)
    };
    cmd.args(args);
    if selected {
        cmd.arg(crate::ARG_Z_UNPRETTY_EXPANDED);
        cmd.arg("-o").arg(env::var_os(OUTPUT).unwrap());
        if env::var_os("RUSTC_BOOTSTRAP").is_none_or(|value| value.is_empty()) {
            cmd.env("RUSTC_BOOTSTRAP", "1");
        }
    }
    cmd.stdin(Stdio::null());
    let code = cmd.status()?.code().unwrap_or(1);
    if selected {
        fs_err::write(
            PathBuf::from(env::var_os(STATUS).unwrap()),
            code.to_string(),
        )?;
        // Cargo must stop here. It otherwise tries to execute the build script.
        return Ok(1);
    }
    Ok(code)
}

// Cargo resolves workspace wrappers from config files, --config, and environment
// variables. Capture the resolved command during a query that compiles no code.
pub fn discover_wrapper(path: &Path) -> Result<i32> {
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    let Some(program) = args.first() else {
        return Err(io::Error::other("missing rustc executable").into());
    };
    let wrapper = if args
        .get(1)
        .is_some_and(|arg| !arg.to_string_lossy().starts_with('-'))
    {
        Some(Path::new(program))
    } else {
        None
    };
    let data = serde_json::to_vec(&wrapper)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    fs_err::write(path, data)?;
    // Stop the discovery pass before Cargo compiles or executes any code.
    Ok(1)
}
