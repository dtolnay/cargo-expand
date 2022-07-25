#![allow(
    clippy::enum_glob_use,
    clippy::items_after_statements,
    clippy::let_underscore_drop,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::needless_pass_by_value,
    clippy::non_ascii_literal,
    clippy::option_option,
    clippy::struct_excessive_bools,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref
)]

mod cmd;
mod config;
mod edit;
mod error;
mod fmt;
mod opts;

use crate::cmd::Line;
use crate::config::Config;
use crate::error::Result;
use crate::opts::Coloring::*;
use crate::opts::{Args, Coloring, Opts};
use atty::Stream::{Stderr, Stdout};
use bat::{PagingMode, PrettyPrinter};
use clap::Parser;
use quote::quote;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, BufRead, Write};
#[cfg(feature = "prettyplease")]
use std::panic::{self, PanicInfo, UnwindSafe};
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio};
use std::str::FromStr;
#[cfg(feature = "prettyplease")]
use std::thread::Result as ThreadResult;
use termcolor::{Color::Green, ColorChoice, ColorSpec, StandardStream, WriteColor};

cargo_subcommand_metadata::description!("Show result of macro expansion");

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

    if env::var_os(NO_RUN_NIGHTLY).is_some() || maybe_nightly() || !can_rustup_run_nightly() {
        return cargo_expand();
    }

    let mut nightly = Command::new("rustup");
    nightly.arg("run");
    nightly.arg("nightly");
    nightly.arg("cargo");
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

fn maybe_nightly() -> bool {
    !definitely_not_nightly()
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

fn can_rustup_run_nightly() -> bool {
    Command::new("rustup")
        .arg("run")
        .arg("nightly")
        .arg("cargo")
        .arg("--version")
        .output()
        .map_or(false, |output| output.status.success())
}

fn cargo_binary() -> OsString {
    env::var_os("CARGO").unwrap_or_else(|| "cargo".to_owned().into())
}

fn cargo_expand() -> Result<i32> {
    let Opts::Expand(args) = Opts::parse();
    let config = config::deserialize();

    if args.themes {
        for theme in PrettyPrinter::new().themes() {
            let _ = writeln!(io::stdout(), "{}", theme);
        }
        return Ok(0);
    }

    let mut rustfmt = None;
    if let Some(item) = &args.item {
        if args.ugly {
            let _ = writeln!(
                io::stderr(),
                "ERROR: cannot expand single item ({}) in ugly mode.",
                item,
            );
            return Ok(1);
        }
        if !cfg!(feature = "prettyplease") {
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
    }

    let mut builder = tempfile::Builder::new();
    builder.prefix("cargo-expand");
    let outdir = builder.tempdir().expect("failed to create tmp file");
    let outfile_path = outdir.path().join("expanded");
    let color = get_color(&args, &config);

    // Run cargo
    let mut cmd = Command::new(cargo_binary());
    apply_args(&mut cmd, &args, &color, &outfile_path);
    let code = filter_err(&mut cmd, ignore_cargo_err)?;

    if !outfile_path.exists() {
        return Ok(1);
    }

    let mut content = fs::read_to_string(&outfile_path)?;
    if content.is_empty() {
        let _ = writeln!(io::stderr(), "ERROR: rustc produced no expanded output");
        return Ok(if code == 0 { 1 } else { code });
    }

    // Format the expanded code
    if !args.ugly {
        let questionably_formatted = content;

        // Work around rustfmt not being able to parse paths containing $crate.
        // This placeholder should be the same width as $crate to preserve
        // alignments.
        const DOLLAR_CRATE_PLACEHOLDER: &str = "Ξcrate";
        let wip = questionably_formatted.replace("$crate", DOLLAR_CRATE_PLACEHOLDER);

        // Support cargo-expand built with panic=abort, as otherwise proc-macro2
        // ends up using a catch_unwind.
        proc_macro2::fallback::force();

        enum Stage {
            Formatted(String),
            Unformatted(String),
            QuestionablyFormatted,
        }

        let mut stage = Stage::QuestionablyFormatted;

        // Discard comments, which are misplaced by the compiler
        if let Ok(mut syntax_tree) = syn::parse_file(&wip) {
            edit::sanitize(&mut syntax_tree);
            if let Some(filter) = args.item {
                syntax_tree.shebang = None;
                syntax_tree.attrs.clear();
                syntax_tree.items = filter.apply_to(&syntax_tree);
                if syntax_tree.items.is_empty() {
                    let _ = writeln!(io::stderr(), "WARNING: no such item: {}", filter);
                    return Ok(1);
                }
            }
            #[cfg(feature = "prettyplease")]
            // This is behind a feature because it's probably not mature enough
            // to use in panic=abort mode yet. I'll remove the feature and do
            // this by default when prettyplease is further along, or when
            // cfg(panic = "unwind") is stabilized, whichever comes first.
            // Tracking issue: https://github.com/rust-lang/rust/issues/77443
            {
                if let Ok(formatted) = ignore_panic(|| prettyplease::unparse(&syntax_tree)) {
                    stage = Stage::Formatted(formatted);
                }
            }
            if let Stage::QuestionablyFormatted = stage {
                let unformatted = quote!(#syntax_tree).to_string();
                stage = Stage::Unformatted(unformatted);
            }
        }

        let to_rustfmt = match &stage {
            Stage::Formatted(_) => None,
            Stage::Unformatted(unformatted) => Some(unformatted),
            Stage::QuestionablyFormatted => Some(&wip),
        };

        if let Some(unformatted) = to_rustfmt {
            if let Some(rustfmt) = rustfmt.or_else(which_rustfmt) {
                fs::write(&outfile_path, unformatted)?;

                fmt::write_rustfmt_config(&outdir)?;

                for edition in &["2018", "2015"] {
                    let output = Command::new(&rustfmt)
                        .arg("--edition")
                        .arg(edition)
                        .arg(&outfile_path)
                        .stderr(Stdio::null())
                        .output();
                    if let Ok(output) = output {
                        if output.status.success() {
                            stage = Stage::Formatted(fs::read_to_string(&outfile_path)?);
                            break;
                        }
                    }
                }
            }
        }

        content = match stage {
            Stage::Formatted(formatted) => formatted.replace(DOLLAR_CRATE_PLACEHOLDER, "$crate"),
            Stage::Unformatted(_) | Stage::QuestionablyFormatted => questionably_formatted,
        };
    }

    // Run pretty printer
    let theme = args.theme.or(config.theme);
    let none_theme = theme.as_deref() == Some("none");
    let do_color = match color {
        Always => true,
        Never => false,
        Auto => !none_theme && atty::is(Stdout),
    };
    let _ = writeln!(io::stderr());
    if do_color {
        let mut pretty_printer = PrettyPrinter::new();
        pretty_printer
            .input_from_bytes(content.as_bytes())
            .language("rust")
            .tab_width(Some(4))
            .true_color(false)
            .header(false)
            .line_numbers(false)
            .grid(false);
        if let Some(theme) = theme {
            pretty_printer.theme(theme);
        }
        if config.pager {
            pretty_printer.paging_mode(PagingMode::QuitIfOneScreen);
        }

        // Ignore any errors.
        let _ = pretty_printer.print();
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
fn apply_args(cmd: &mut Command, args: &Args, color: &Coloring, outfile: &Path) {
    let mut line = Line::new("cargo");

    line.arg("rustc");

    line.arg("--profile");
    if let Some(profile) = &args.profile {
        line.arg(profile);
    } else if args.tests && args.test.is_none() {
        line.arg("test");
    } else {
        line.arg("check");
    }

    if args.release {
        line.arg("--release");
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
        line.args(bin);
    }

    if let Some(example) = &args.example {
        line.arg("--example");
        line.args(example);
    }

    if let Some(test) = &args.test {
        line.arg("--test");
        line.args(test);
    }

    if let Some(bench) = &args.bench {
        line.arg("--bench");
        line.args(bench);
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
        line.args(package);
    }

    if let Some(jobs) = args.jobs {
        line.arg("--jobs");
        line.arg(jobs.to_string());
    }

    if args.verbose {
        line.arg("--verbose");
    }

    line.arg("--color");
    match color {
        Coloring::Auto => line.arg(if cfg!(not(windows)) && atty::is(Stderr) {
            "always"
        } else {
            "never"
        }),
        color => line.arg(color.to_string()),
    }

    if args.frozen {
        line.arg("--frozen");
    }

    if args.locked {
        line.arg("--locked");
    }

    if args.offline {
        line.arg("--offline");
    }

    for unstable_flag in &args.unstable_flags {
        line.arg("-Z");
        line.arg(unstable_flag);
    }

    line.arg("--");

    line.arg("-o");
    line.arg(outfile);
    line.arg("-Zunpretty=expanded");

    if args.verbose {
        let mut display = line.clone();
        display.insert(0, "+nightly");
        print_command(display, color);
    }

    cmd.args(line);
}

fn print_command(line: Line, color: &Coloring) {
    let color_choice = match color {
        Coloring::Auto => ColorChoice::Auto,
        Coloring::Always => ColorChoice::Always,
        Coloring::Never => ColorChoice::Never,
    };

    let mut stream = StandardStream::stderr(color_choice);
    let _ = stream.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Green)));
    let _ = write!(stream, "{:>12}", "Running");
    let _ = stream.reset();
    let _ = writeln!(stream, " `{}`", line);
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
            let _ = write!(io::stderr(), "{}", line);
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

    let discarded_lines = [
        "ignoring specified output filename because multiple outputs were \
         requested",
        "ignoring specified output filename for 'link' output because multiple \
         outputs were requested",
        "ignoring --out-dir flag due to -o flag",
        "ignoring -C extra-filename flag due to -o flag",
        "due to multiple output types requested, the explicitly specified \
         output file name will be adapted for each output type",
        "warning emitted",
        "warnings emitted",
        ") generated ",
    ];
    for s in &discarded_lines {
        if line.contains(s) {
            return true;
        }
    }

    false
}

#[cfg(feature = "prettyplease")]
fn ignore_panic<F, T>(f: F) -> ThreadResult<T>
where
    F: UnwindSafe + FnOnce() -> T,
{
    type PanicHook = dyn Fn(&PanicInfo) + Sync + Send + 'static;

    let null_hook: Box<PanicHook> = Box::new(|_panic_info| { /* ignore */ });
    let sanity_check = &*null_hook as *const PanicHook;
    let original_hook = panic::take_hook();
    panic::set_hook(null_hook);

    let result = panic::catch_unwind(f);

    let hopefully_null_hook = panic::take_hook();
    panic::set_hook(original_hook);
    if sanity_check != &*hopefully_null_hook {
        panic!("race condition on std::panic hook");
    }

    result
}

fn get_color(args: &Args, config: &Config) -> Coloring {
    if let Some(value) = args.color {
        return value;
    }

    if env::var_os("NO_COLOR").is_some() {
        return Coloring::Never;
    }

    if let Some(value) = config.color.as_ref() {
        match Coloring::from_str(value.as_str()) {
            Ok(color) => return color,
            Err(err) => {
                let _ = writeln!(
                    io::stderr(),
                    "WARNING: invalid color in cargo config: {}",
                    err
                );
            }
        }
    }

    Coloring::Auto // default
}
