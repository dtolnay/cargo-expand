mod cmd;
mod config;
mod edit;
mod error;
mod fmt;
mod opts;

use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, BufRead, Write};
use std::mem;
use std::path::{Path, PathBuf};
use std::process::{self, ChildStderr, Command, Stdio};

use atty::Stream::{Stderr, Stdout};
use prettyprint::{PagingMode, PrettyPrinter};
use quote::quote;
use structopt::StructOpt;

use crate::cmd::Line;
use crate::error::Result;
use crate::opts::Coloring::*;
use crate::opts::{Args, Opts};

fn main() {
    let result = cargo_expand_or_run_nightly();
    process::exit(match result {
        Ok(code) => code,
        Err(err) => {
            let _ = writeln!(io::stderr(), "{}", err);
            1
        }
    });
}

fn cargo_expand_or_run_nightly() -> Result<i32> {
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

fn cargo_expand() -> Result<i32> {
    let Opts::Expand(args) = Opts::from_args();
    let config = config::deserialize();

    if args.themes {
        for theme in PrettyPrinter::default()
            .build()
            .unwrap()
            .get_themes()
            .keys()
        {
            let _ = writeln!(io::stdout(), "{}", theme);
        }
        return Ok(0);
    }

    let rustfmt;
    match (&args.item, args.ugly) {
        (Some(item), true) => {
            let _ = writeln!(
                io::stderr(),
                "ERROR: cannot expand single item ({}) in ugly mode.",
                item,
            );
            return Ok(1);
        }
        (Some(item), false) => {
            rustfmt = which_rustfmt();
            if rustfmt.is_none() {
                let _ = writeln!(
                    io::stderr(),
                    "ERROR: cannot expand single item ({}) without rustfmt.",
                    item,
                );
                let _ = writeln!(
                    io::stderr(),
                    "Install rustfmt by running `rustup component add rustfmt --toolchain nightly`.",
                );
                return Ok(1);
            }
        }
        (None, true) => rustfmt = None,
        (None, false) => rustfmt = which_rustfmt(),
    }

    let mut builder = tempfile::Builder::new();
    builder.prefix("cargo-expand");
    let outdir = builder.tempdir().expect("failed to create tmp file");
    let outfile_path = outdir.path().join("expanded");

    // Run cargo
    let mut cmd = Command::new(cargo_binary());
    apply_args(&mut cmd, &args, &outfile_path);
    let code = filter_err(&mut cmd, ignore_cargo_err)?;

    if !outfile_path.exists() {
        return Ok(1);
    }

    let mut content = fs::read_to_string(&outfile_path)?;
    if content.is_empty() {
        let _ = writeln!(io::stderr(), "ERROR: rustc produced no expanded output",);
        return Ok(if code == 0 { 1 } else { code });
    }

    // Run rustfmt
    if let Some(rustfmt) = rustfmt {
        // Work around rustfmt not being able to parse paths containing $crate.
        // This placeholder should be the same width as $crate to preserve
        // alignments.
        const DOLLAR_CRATE_PLACEHOLDER: &str = "Ξcrate";
        content = content.replace("$crate", DOLLAR_CRATE_PLACEHOLDER);

        // Discard comments, which are misplaced by the compiler
        if let Ok(mut syntax_tree) = syn::parse_file(&content) {
            edit::remove_macro_rules(&mut syntax_tree);
            if let Some(filter) = args.item {
                syntax_tree.shebang = None;
                syntax_tree.attrs.clear();
                syntax_tree.items = filter.apply_to(&syntax_tree);
                if syntax_tree.items.is_empty() {
                    let _ = writeln!(io::stderr(), "WARNING: no such item: {}", filter);
                    return Ok(1);
                }
            }
            content = quote!(#syntax_tree).to_string();
        }
        fs::write(&outfile_path, content)?;

        fmt::write_rustfmt_config(&outdir)?;

        // Ignore any errors.
        let _status = Command::new(rustfmt)
            .arg("--edition=2018")
            .arg(&outfile_path)
            .stderr(Stdio::null())
            .status();

        content = fs::read_to_string(&outfile_path)?;
        content = content.replace(DOLLAR_CRATE_PLACEHOLDER, "$crate");
    }

    // Run pretty printer
    let theme = args.theme.or(config.theme);
    let none_theme = theme.as_ref().map(String::as_str) == Some("none");
    let do_color = match args.color {
        Some(Always) => true,
        Some(Never) => false,
        None | Some(Auto) => !none_theme && atty::is(Stdout),
    };
    let _ = writeln!(io::stderr());
    if do_color {
        if content.ends_with('\n') {
            // Pretty printer seems to print an extra trailing newline.
            content.truncate(content.len() - 1);
        }

        let mut builder = PrettyPrinter::default();
        builder.header(false);
        builder.grid(false);
        builder.line_numbers(false);
        builder.language("rust");
        builder.paging_mode(PagingMode::Never);

        if let Some(theme) = theme {
            builder.theme(theme);
        }

        let printer = builder.build().unwrap();

        // Ignore any errors.
        let _ = printer.string(content);
    } else {
        let _ = write!(io::stdout(), "{}", content);
    }

    Ok(0)
}

fn which_rustfmt() -> Option<PathBuf> {
    match env::var_os("RUSTFMT") {
        Some(which) => {
            if which.is_empty() {
                None
            } else {
                Some(PathBuf::from(which))
            }
        }
        None => toolchain_find::find_installed_component("rustfmt"),
    }
}

// Based on https://github.com/rsolomo/cargo-check
fn apply_args(cmd: &mut Command, args: &Args, outfile: &Path) {
    let mut line = Line::new("cargo");

    line.arg("rustc");

    if args.tests && args.test.is_none() {
        line.arg("--profile=test");
    } else {
        line.arg("--profile=check");
    }

    if let Some(features) = &args.features {
        line.arg("--features");
        line.arg(features);
    }

    if args.all_features {
        line.arg("--all-features");
    }

    if args.no_default_features {
        line.arg("--no-default-features");
    }

    if args.lib {
        line.arg("--lib");
    }

    if let Some(bin) = &args.bin {
        line.arg("--bin");
        line.arg(bin);
    }

    if let Some(example) = &args.example {
        line.arg("--example");
        line.arg(example);
    }

    if let Some(test) = &args.test {
        line.arg("--test");
        line.arg(test);
    }

    if let Some(bench) = &args.bench {
        line.arg("--bench");
        line.arg(bench);
    }

    if let Some(target) = &args.target {
        line.arg("--target");
        line.arg(target);
    }

    if let Some(target_dir) = &args.target_dir {
        line.arg("--target-dir");
        line.arg(target_dir);
    }

    if let Some(manifest_path) = &args.manifest_path {
        line.arg("--manifest-path");
        line.arg(manifest_path);
    }

    if let Some(package) = &args.package {
        line.arg("--package");
        line.arg(package);
    }

    if let Some(jobs) = args.jobs {
        line.arg("--jobs");
        line.arg(jobs.to_string());
    }

    if args.verbose {
        line.arg("--verbose");
    }

    line.arg("--color");
    if let Some(color) = &args.color {
        line.arg(color.to_string());
    } else {
        line.arg(if atty::is(Stderr) { "always" } else { "never" });
    }

    if args.frozen {
        line.arg("--frozen");
    }

    if args.locked {
        line.arg("--locked");
    }

    for unstable_flag in &args.unstable_flags {
        line.arg("-Z");
        line.arg(unstable_flag);
    }

    line.arg("--");

    line.arg("-o");
    line.arg(outfile);
    line.arg("-Zunstable-options");
    line.arg("--pretty=expanded");

    if args.verbose {
        let _ = writeln!(io::stderr(), "     Running `{}`", line);
    }

    cmd.args(line);
}

fn filter_err(cmd: &mut Command, ignore: fn(&str) -> bool) -> io::Result<i32> {
    let mut child = cmd.stderr(Stdio::piped()).spawn()?;

    if let Some(child_stderr) = &child.stderr {
        set_term_size(child_stderr);
    }

    let mut stderr = io::BufReader::new(child.stderr.take().unwrap());
    let mut line = String::new();
    while let Ok(n) = stderr.read_line(&mut line) {
        if n == 0 {
            break;
        }
        if !ignore(&line) {
            let _ = write!(io::stderr(), "{}", line);
        }
        line.clear();
    }
    let code = child.wait()?.code().unwrap_or(1);
    Ok(code)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn set_term_size(child: &ChildStderr) {
    use std::os::unix::io::AsRawFd;

    unsafe {
        let mut winsize: libc::winsize = mem::zeroed();
        if libc::ioctl(libc::STDERR_FILENO, libc::TIOCGWINSZ, &mut winsize) < 0 {
            return;
        }
        if winsize.ws_col == 0 {
            return;
        }
        let ret = libc::ioctl(child.as_raw_fd(), libc::TIOCSWINSZ, &winsize);
        if ret < 0 {
            eprintln!("ERROR: {}", io::Error::last_os_error());
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn set_term_size(_: &ChildStderr) {}

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
