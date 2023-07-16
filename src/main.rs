#![allow(
    clippy::bool_to_int_with_if,
    clippy::enum_glob_use,
    clippy::items_after_statements,
    clippy::let_underscore_untyped,
    clippy::manual_assert,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::match_same_arms, // https://github.com/rust-lang/rust-clippy/issues/10327
    clippy::needless_borrow, // https://github.com/rust-lang/rust-clippy/issues/9710
    clippy::needless_pass_by_value,
    clippy::non_ascii_literal,
    clippy::option_option,
    clippy::struct_excessive_bools,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
)]
#![cfg_attr(all(test, exhaustive), feature(non_exhaustive_omitted_patterns_lint))]

mod cmd;
mod config;
mod edit;
mod error;
mod fmt;
mod opts;
mod unparse;

use crate::cmd::Line;
use crate::config::Config;
use crate::error::Result;
use crate::opts::Coloring::*;
use crate::opts::{Coloring, Expand, Subcommand};
use crate::unparse::unparse_maximal;
use bat::{PagingMode, PrettyPrinter};
use clap::{Parser, ValueEnum};
use is_terminal::IsTerminal;
use quote::quote;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, BufRead, Write};
use std::panic::{self, PanicInfo, UnwindSafe};
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio};
use std::ptr;
use std::thread::Result as ThreadResult;
use termcolor::{Color::Green, ColorChoice, ColorSpec, StandardStream, WriteColor};

cargo_subcommand_metadata::description!("Show result of macro expansion");

fn main() {
    let result = cargo_expand();
    process::exit(match result {
        Ok(code) => code,
        Err(err) => {
            let _ = writeln!(io::stderr(), "{}", err);
            1
        }
    });
}

fn cargo_binary() -> OsString {
    env::var_os("CARGO").unwrap_or_else(|| "cargo".to_owned().into())
}

fn cargo_expand() -> Result<i32> {
    let Subcommand::Expand(args) = Subcommand::parse();
    let config = config::deserialize();

    if args.themes {
        for theme in PrettyPrinter::new().themes() {
            let _ = writeln!(io::stdout(), "{}", theme);
        }
        return Ok(0);
    }

    if let Some(item) = &args.item {
        if args.ugly {
            let _ = writeln!(
                io::stderr(),
                "ERROR: cannot expand single item ({}) in ugly mode.",
                item,
            );
            return Ok(1);
        }
    }

    let mut rustfmt = None;
    if config.rustfmt {
        rustfmt = which_rustfmt();
        if rustfmt.is_none() {
            let _ = io::stderr().write_all(
                b"ERROR: cargo-expand configuration sets rustfmt=true, but \
                rustfmt is not found. Install rustfmt by running `rustup \
                component add rustfmt --toolchain nightly`.\n",
            );
            return Ok(1);
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
    cmd.env("RUSTC_BOOTSTRAP", "1");
    let code = filter_err(&mut cmd)?;

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
            if !config.rustfmt {
                if let Ok(formatted) = ignore_panic(|| unparse_maximal(&syntax_tree)) {
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

                for edition in &["2021", "2018", "2015"] {
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
        Auto => !none_theme && io::stdout().is_terminal(),
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

fn apply_args(cmd: &mut Command, args: &Expand, color: &Coloring, outfile: &Path) {
    let mut line = Line::new("cargo");

    line.arg("rustc");

    line.arg("--profile");
    if let Some(profile) = &args.profile {
        line.arg(profile);
    } else if args.tests && args.test.is_none() {
        if args.release {
            line.arg("bench");
        } else {
            line.arg("test");
        }
    } else if args.release {
        line.arg("release");
    } else {
        line.arg("check");
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

    let mut has_explicit_build_target = false;
    if args.lib {
        line.arg("--lib");
        has_explicit_build_target = true;
    }

    if let Some(bin) = &args.bin {
        line.arg("--bin");
        line.args(bin);
        has_explicit_build_target = true;
    }

    if let Some(example) = &args.example {
        line.arg("--example");
        line.args(example);
        has_explicit_build_target = true;
    }

    if let Some(test) = &args.test {
        line.arg("--test");
        line.args(test);
        has_explicit_build_target = true;
    }

    if let Some(bench) = &args.bench {
        line.arg("--bench");
        line.args(bench);
        has_explicit_build_target = true;
    }

    if !has_explicit_build_target {
        match cargo_metadata(&args.manifest_path) {
            Ok(cargo_metadata) => {
                if let Some(root_package) = cargo_metadata.root_package() {
                    if let Some(ref default_run) = root_package.default_run {
                        line.arg("--bin");
                        line.args(Some(default_run));
                    }
                }
            }
            Err(err) => {
                let _ = writeln!(io::stderr(), "WARNING: run cargo metadata fail: {}", err);
            }
        }
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
        Coloring::Auto => line.arg(if cfg!(not(windows)) && io::stderr().is_terminal() {
            "always"
        } else {
            "never"
        }),
        color => line.arg(color.to_possible_value().unwrap().get_name()),
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

fn filter_err(cmd: &mut Command) -> io::Result<i32> {
    let mut child = cmd.stderr(Stdio::piped()).spawn()?;
    let mut stderr = io::BufReader::new(child.stderr.take().unwrap());
    let mut line = String::new();
    while let Ok(n) = stderr.read_line(&mut line) {
        if n == 0 {
            break;
        }
        if !ignore_cargo_err(&line) {
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

fn ignore_panic<F, T>(f: F) -> ThreadResult<T>
where
    F: UnwindSafe + FnOnce() -> T,
{
    type PanicHook = dyn Fn(&PanicInfo) + Sync + Send + 'static;

    let null_hook: Box<PanicHook> = Box::new(|_panic_info| { /* ignore */ });
    let sanity_check = ptr::addr_of!(*null_hook);
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

fn get_color(args: &Expand, config: &Config) -> Coloring {
    if let Some(value) = args.color {
        return value;
    }

    if env::var_os("NO_COLOR").is_some() {
        return Coloring::Never;
    }

    if let Some(value) = config.color.as_ref() {
        let ignore_case = false;
        match Coloring::from_str(value.as_str(), ignore_case) {
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

fn cargo_metadata(
    manifest_path: &Option<PathBuf>,
) -> cargo_metadata::Result<cargo_metadata::Metadata> {
    let mut cmd = cargo_metadata::MetadataCommand::new();
    if let Some(manifest_path) = manifest_path {
        cmd.manifest_path(manifest_path);
    }
    cmd.exec()
}
