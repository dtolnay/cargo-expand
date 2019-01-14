use std::env;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, BufRead, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio};

use atty::Stream::{Stderr, Stdout};
use quote::quote;
use structopt::clap::AppSettings;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(bin_name = "cargo")]
enum Opts {
    /// Show the result of macro expansion.
    #[structopt(
        name = "expand",
        raw(
            setting = "AppSettings::UnifiedHelpMessage",
            setting = "AppSettings::DeriveDisplayOrder",
            setting = "AppSettings::DontCollapseArgsInUsage"
        )
    )]
    Expand(Args),
}

#[derive(StructOpt, Debug)]
#[structopt(rename_all = "kebab-case")]
struct Args {
    /// Space-separated list of features to activate
    #[structopt(long, value_name = "FEATURES")]
    features: Option<String>,

    /// Activate all available features
    #[structopt(long)]
    all_features: bool,

    /// Do not activate the `default` feature
    #[structopt(long)]
    no_default_features: bool,

    /// Build only this package's library
    #[structopt(long)]
    lib: bool,

    /// Build only the specified binary
    #[structopt(long, value_name = "NAME")]
    bin: Option<String>,

    /// Build only the specified example
    #[structopt(long, value_name = "NAME")]
    example: Option<String>,

    /// Build only the specified test target
    #[structopt(long, value_name = "NAME")]
    test: Option<String>,

    /// Build only the specified bench target
    #[structopt(long, value_name = "NAME")]
    bench: Option<String>,

    /// Target triple which compiles will be for
    #[structopt(long, value_name = "TARGET")]
    target: Option<String>,

    /// Directory for all generated artifacts
    #[structopt(long, value_name = "DIRECTORY", parse(from_os_str))]
    target_dir: Option<PathBuf>,

    /// Path to Cargo.toml
    #[structopt(long, value_name = "PATH", parse(from_os_str))]
    manifest_path: Option<PathBuf>,

    /// Number of parallel jobs, defaults to # of CPUs
    #[structopt(short, long, value_name = "N")]
    jobs: Option<u64>,

    /// Coloring: auto, always, never
    #[structopt(long, value_name = "WHEN")]
    color: Option<String>,

    /// Require Cargo.lock and cache are up to date
    #[structopt(long)]
    frozen: bool,

    /// Require Cargo.lock is up to date
    #[structopt(long)]
    locked: bool,

    /// Unstable (nightly-only) flags to Cargo
    #[structopt(short = "Z", value_name = "FLAG")]
    unstable_flags: Vec<String>,
}

fn main() {
    let result = cargo_expand_or_run_nightly();
    process::exit(match result {
        Ok(code) => code,
        Err(err) => {
            let _ = writeln!(&mut io::stderr(), "{}", err);
            1
        }
    });
}

fn cargo_expand_or_run_nightly() -> io::Result<i32> {
    const NO_RUN_NIGHTLY: &str = "CARGO_EXPAND_NO_RUN_NIGHTLY";

    let maybe_nightly = !definitely_not_nightly();
    if maybe_nightly || env::var_os(NO_RUN_NIGHTLY).is_some() {
        return cargo_expand();
    }

    let mut nightly = Command::new("cargo");
    nightly.arg("+nightly");
    nightly.arg("expand");

    let mut args = env::args_os().peekable();
    args.next().unwrap(); // cargo
    if args.peek().map_or(false, |arg| arg == "expand") {
        args.next().unwrap(); // expand
    }
    nightly.args(args);

    // Hopefully prevent infinite re-run loop.
    nightly.env(NO_RUN_NIGHTLY, "");

    let status = nightly.status()?;

    Ok(match status.code() {
        Some(code) => code,
        None => {
            if status.success() {
                0
            } else {
                1
            }
        }
    })
}

fn definitely_not_nightly() -> bool {
    let mut cmd = Command::new(cargo_binary());
    cmd.arg("--version");

    let output = match cmd.output() {
        Ok(output) => output,
        Err(_) => return false,
    };

    let version = match String::from_utf8(output.stdout) {
        Ok(version) => version,
        Err(_) => return false,
    };

    version.starts_with("cargo 1") && !version.contains("nightly")
}

fn cargo_binary() -> OsString {
    env::var_os("CARGO").unwrap_or_else(|| "cargo".to_owned().into())
}

fn cargo_expand() -> io::Result<i32> {
    let Opts::Expand(args) = Opts::from_args();

    let allow_color = args.color.as_ref().map_or(true, |color| color != "never");

    let which_rustfmt = which(&["rustfmt"]);
    let which_pygmentize = if allow_color && atty::is(Stdout) {
        which(&["pygmentize", "-l", "rust"])
    } else {
        None
    };

    let mut builder = tempfile::Builder::new();
    builder.prefix("cargo-expand");
    let outdir = builder.tempdir().expect("failed to create tmp file");
    let outfile_path = outdir.path().join("expanded");

    // Run cargo
    let mut cmd = Command::new(cargo_binary());
    apply_args(&mut cmd, &args, &outfile_path);
    let code = filter_err(&mut cmd, ignore_cargo_err)?;

    let mut outfile = File::open(&outfile_path)?;
    if outfile.metadata()?.len() == 0 {
        let _ = writeln!(
            &mut io::stderr(),
            "ERROR: rustc produced no expanded output"
        );
        return Ok(if code == 0 { 1 } else { code });
    }

    // Run rustfmt
    if let Some(fmt) = which_rustfmt {
        // Discard comments, which are misplaced by the compiler
        let mut content = Vec::new();
        outfile.read_to_end(&mut content)?;
        match String::from_utf8(content) {
            Ok(content) => {
                if let Ok(syntax_tree) = syn::parse_file(&content) {
                    let content = quote!(#syntax_tree).to_string();
                    fs::write(&outfile_path, content)?;
                }
            }
            Err(_) => {
                let _ = writeln!(&mut io::stderr(), "WARNING: non-UTF8 content");
            }
        }

        // Ignore any errors.
        let _status = Command::new(fmt)
            .arg(&outfile_path)
            .stderr(Stdio::null())
            .status();
    }

    // Run pygmentize
    if let Some(pyg) = which_pygmentize {
        let _status = Command::new(pyg)
            .args(&["-l", "rust", "-O", "encoding=utf8"])
            .arg(&outfile_path)
            .status();
    } else {
        // Cat outfile if rustfmt was used.
        let mut reader = File::open(&outfile_path)?;
        io::copy(&mut reader, &mut io::stdout())?;
    }
    Ok(0)
}

// Based on https://github.com/rsolomo/cargo-check
fn apply_args(cmd: &mut Command, args: &Args, outfile: &Path) {
    cmd.arg("rustc");
    cmd.arg("--profile=check");

    if let Some(features) = &args.features {
        cmd.arg("--features");
        cmd.arg(features);
    }

    if args.all_features {
        cmd.arg("--all-features");
    }

    if args.no_default_features {
        cmd.arg("--no-default-features");
    }

    if args.lib {
        cmd.arg("--lib");
    }

    if let Some(bin) = &args.bin {
        cmd.arg("--bin");
        cmd.arg(bin);
    }

    if let Some(example) = &args.example {
        cmd.arg("--example");
        cmd.arg(example);
    }

    if let Some(test) = &args.test {
        cmd.arg("--test");
        cmd.arg(test);
    }

    if let Some(bench) = &args.bench {
        cmd.arg("--bench");
        cmd.arg(bench);
    }

    if let Some(target) = &args.target {
        cmd.arg("--target");
        cmd.arg(target);
    }

    if let Some(target_dir) = &args.target_dir {
        cmd.arg("--target-dir");
        cmd.arg(target_dir);
    }

    if let Some(manifest_path) = &args.manifest_path {
        cmd.arg("--manifest-path");
        cmd.arg(manifest_path);
    }

    if let Some(jobs) = args.jobs {
        cmd.arg("--jobs");
        cmd.arg(jobs.to_string());
    }

    cmd.arg("--color");
    if let Some(color) = &args.color {
        cmd.arg(color);
    } else {
        cmd.arg(if atty::is(Stderr) { "always" } else { "never" });
    }

    if args.frozen {
        cmd.arg("--frozen");
    }

    if args.locked {
        cmd.arg("--locked");
    }

    for unstable_flag in &args.unstable_flags {
        cmd.arg("-Z");
        cmd.arg(unstable_flag);
    }

    cmd.arg("--");
    cmd.arg("-o");
    cmd.arg(outfile);
    cmd.arg("-Zunstable-options");
    cmd.arg("--pretty=expanded");
}

fn which(cmd: &[&str]) -> Option<OsString> {
    if env::args_os().any(|arg| arg == *"--help") {
        return None;
    }

    if let Some(which) = env::var_os(&cmd[0].to_uppercase()) {
        return if which.is_empty() { None } else { Some(which) };
    }

    let spawn = Command::new(cmd[0])
        .args(&cmd[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    let mut child = match spawn {
        Ok(child) => child,
        Err(_) => {
            return None;
        }
    };

    let exit = match child.wait() {
        Ok(exit) => exit,
        Err(_) => {
            return None;
        }
    };

    if exit.success() {
        Some(cmd[0].into())
    } else {
        None
    }
}

fn filter_err(cmd: &mut Command, ignore: fn(&str) -> bool) -> io::Result<i32> {
    let mut child = cmd.stderr(Stdio::piped()).spawn()?;
    let mut stderr = io::BufReader::new(child.stderr.take().unwrap());
    let mut line = String::new();
    while let Ok(n) = stderr.read_line(&mut line) {
        if n == 0 {
            break;
        }
        if !ignore(&line) {
            let _ = write!(&mut io::stderr(), "{}", line);
        }
        line.clear();
    }
    let code = child.wait()?.code().unwrap_or(1);
    Ok(code)
}

fn ignore_cargo_err(line: &str) -> bool {
    if line.trim().is_empty() {
        return true;
    }

    let blacklist = [
        "ignoring specified output filename because multiple outputs were \
         requested",
        "ignoring specified output filename for 'link' output because multiple \
         outputs were requested",
        "ignoring --out-dir flag due to -o flag",
        "ignoring -C extra-filename flag due to -o flag",
        "due to multiple output types requested, the explicitly specified \
         output file name will be adapted for each output type",
    ];
    for s in &blacklist {
        if line.contains(s) {
            return true;
        }
    }

    false
}
