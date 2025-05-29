#![allow(
    clippy::doc_markdown,
    clippy::items_after_statements,
    clippy::manual_assert,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::match_same_arms, // https://github.com/rust-lang/rust-clippy/issues/10327
    clippy::module_name_repetitions,
    clippy::needless_pass_by_value,
    clippy::needless_return,
    clippy::new_without_default,
    clippy::option_option,
    clippy::struct_excessive_bools,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
)]

mod assets;
mod cmd;
mod config;
mod edit;
mod error;
mod fmt;
mod manifest;
mod opts;
mod unparse;
mod version;

use crate::cmd::CommandExt as _;
use crate::config::Config;
use crate::error::Result;
use crate::opts::{Coloring, Expand, Subcommand};
use crate::unparse::unparse_maximal;
use crate::version::Version;
use bat::assets::HighlightingAssets;
use bat::assets_metadata::AssetsMetadata;
use bat::config::VisibleLines;
use bat::line_range::{HighlightedLineRanges, LineRanges};
use bat::style::StyleComponents;
use bat::theme::{ThemeName, ThemeOptions, ThemePreference};
use bat::{PagingMode, SyntaxMapping, WrappingMode};
use clap::{CommandFactory as _, Parser, ValueEnum};
use quote::quote;
use std::env;
use std::error::Error as StdError;
use std::ffi::{OsStr, OsString};
use std::io::{self, BufRead, IsTerminal, Write};
use std::iter;
use std::panic::{self, UnwindSafe};
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio};
use std::ptr;
use std::str;
use std::thread::Result as ThreadResult;
use termcolor::{Color::Green, ColorChoice, ColorSpec, StandardStream, WriteColor};

#[allow(deprecated)] // https://github.com/dtolnay/cargo-expand/issues/229
use std::panic::PanicInfo;

cargo_subcommand_metadata::description!("Show result of macro expansion");

fn main() {
    let result = if let Some(wrapper) = env::var_os(CARGO_EXPAND_RUSTC_WRAPPER) {
        do_rustc_wrapper(&wrapper)
    } else {
        do_cargo_expand()
    };

    process::exit(match result {
        Ok(code) => code,
        Err(err) => {
            let mut stderr = io::stderr().lock();
            let _ = writeln!(stderr, "{}", err);
            let mut err = &err as &dyn StdError;
            while let Some(source) = err.source() {
                let _ = writeln!(stderr, "\nCaused by:\n  {}", source);
                err = source;
            }
            1
        }
    });
}

const CARGO_EXPAND_RUSTC_WRAPPER: &str = "CARGO_EXPAND_RUSTC_WRAPPER";
const ARG_Z_UNPRETTY_EXPANDED: &str = "-Zunpretty=expanded";

fn cargo_binary() -> OsString {
    env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"))
}

fn do_rustc_wrapper(wrapper: &OsStr) -> Result<i32> {
    let mut rustc_command = env::args_os().skip(1);
    let mut cmd = if wrapper != "/" {
        Command::new(wrapper)
    } else if let Some(rustc) = rustc_command.next() {
        Command::new(rustc)
    } else {
        Subcommand::command().print_help()?;
        return Ok(1);
    };

    let mut is_unpretty_expanded = false;
    for arg in rustc_command {
        is_unpretty_expanded |= arg == ARG_Z_UNPRETTY_EXPANDED;
        cmd.arg(arg);
    }

    if is_unpretty_expanded {
        cmd.env("RUSTC_BOOTSTRAP", "1");
    }

    #[cfg(unix)]
    {
        use crate::error::Error;
        use std::os::unix::process::CommandExt as _;

        let err = cmd.exec();
        return Err(Error::Io(err));
    }

    #[cfg(not(unix))]
    {
        let exit_status = cmd.status()?;
        let code = exit_status.code().unwrap_or(1);
        return Ok(code);
    }
}

fn do_cargo_expand() -> Result<i32> {
    let Subcommand::Expand(args) = Subcommand::parse();

    if args.version {
        let version = Version {
            verbose: args.verbose,
        };
        let _ = writeln!(io::stdout(), "{}", version);
        return Ok(0);
    }

    let config = config::deserialize();

    if args.themes {
        print_themes()?;
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
    apply_args(&mut cmd, &args, color, &outfile_path);
    if args.verbose {
        print_command(&cmd, color)?;
    }

    if needs_rustc_bootstrap() {
        if let Ok(current_exe) = env::current_exe() {
            let original_wrapper =
                env::var_os("RUSTC_WRAPPER").filter(|wrapper| !wrapper.is_empty());
            let wrapper = original_wrapper.as_deref().unwrap_or(OsStr::new("/"));
            cmd.env(CARGO_EXPAND_RUSTC_WRAPPER, wrapper);
            cmd.env("RUSTC_WRAPPER", current_exe);
        } else {
            cmd.env("RUSTC_BOOTSTRAP", "1");
        }
    }

    let code = filter_err(&mut cmd)?;

    if !outfile_path.exists() {
        return Ok(1);
    }

    let mut content = fs_err::read_to_string(&outfile_path)?;
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
                fs_err::write(&outfile_path, unformatted)?;

                fmt::write_rustfmt_config(&outdir)?;

                for edition in &["2021", "2018", "2015"] {
                    let output = Command::new(&rustfmt)
                        .flag_value("--edition", edition)
                        .arg(&outfile_path)
                        .stderr(Stdio::null())
                        .output();
                    if let Ok(output) = output {
                        if output.status.success() {
                            stage = Stage::Formatted(fs_err::read_to_string(&outfile_path)?);
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
    let mut theme = args.theme.or(config.theme);
    let none_theme = theme.as_deref() == Some("none");
    let do_color = match color {
        Coloring::Always => true,
        Coloring::Never => false,
        Coloring::Auto => !none_theme && io::stdout().is_terminal(),
    };
    let _ = writeln!(io::stderr());
    if do_color {
        let theme_result = bat::theme::theme(ThemeOptions {
            theme: theme
                .clone()
                .or_else(|| env::var(bat::theme::env::BAT_THEME).ok())
                .map_or_else(ThemePreference::default, ThemePreference::new),
            theme_dark: env::var(bat::theme::env::BAT_THEME_DARK)
                .ok()
                .map(ThemeName::new),
            theme_light: env::var(bat::theme::env::BAT_THEME_LIGHT)
                .ok()
                .map(ThemeName::new),
        });
        match theme_result.theme {
            ThemeName::Named(named) => theme = Some(named),
            ThemeName::Default => {
                if let Some(color_scheme) = theme_result.color_scheme {
                    let default_theme = bat::theme::default_theme(color_scheme);
                    theme = Some(default_theme.to_owned());
                }
            }
        }
        let mut assets = HighlightingAssets::from_binary();
        if let Some(requested_theme) = &theme {
            if !assets
                .themes()
                .any(|supported_theme| supported_theme == requested_theme)
            {
                let cache_dir = assets::cache_dir()?;
                if let Some(metadata) = AssetsMetadata::load_from_folder(&cache_dir)? {
                    if metadata.is_compatible_with(assets::BAT_VERSION) {
                        assets = HighlightingAssets::from_cache(&cache_dir)?;
                    }
                }
            }
        }
        let config = bat::config::Config {
            language: Some("rust"),
            show_nonprintable: false,
            term_width: console::Term::stdout().size().1 as usize,
            tab_width: 4,
            colored_output: true,
            true_color: false,
            style_components: StyleComponents::new(&[]),
            wrapping_mode: WrappingMode::default(),
            paging_mode: if config.pager {
                PagingMode::QuitIfOneScreen
            } else {
                PagingMode::Never
            },
            visible_lines: VisibleLines::Ranges(LineRanges::all()),
            theme: theme.unwrap_or_else(String::new),
            syntax_mapping: SyntaxMapping::new(),
            pager: None,
            use_italic_text: false,
            highlighted_lines: HighlightedLineRanges(LineRanges::none()),
            ..Default::default()
        };
        let controller = bat::controller::Controller::new(&config, &assets);
        let inputs = vec![bat::input::Input::from_reader(Box::new(content.as_bytes()))];
        // Ignore any errors.
        let _ = controller.run(inputs, None);
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

fn apply_args(cmd: &mut Command, args: &Expand, color: Coloring, outfile: &Path) {
    cmd.arg("rustc");

    if args.verbose {
        cmd.arg("--verbose");
    }

    match color {
        Coloring::Auto => {
            if cfg!(not(windows)) && io::stderr().is_terminal() {
                cmd.flag_value("--color", "always");
            } else {
                cmd.flag_value("--color", "never");
            }
        }
        color => {
            cmd.flag_value("--color", color.to_possible_value().unwrap().get_name());
        }
    }

    for kv in &args.config {
        cmd.flag_value("--config", kv);
    }

    for unstable_flag in &args.unstable_flags {
        cmd.arg(format!("-Z{}", unstable_flag));
    }

    if let Some(opt_package) = &args.package {
        if let Some(package) = opt_package {
            cmd.flag_value("--package", package);
        } else {
            cmd.arg("--package");
        }
    }

    let mut has_explicit_build_target = false;
    if args.lib {
        cmd.arg("--lib");
        has_explicit_build_target = true;
    }

    if let Some(opt_bin) = &args.bin {
        if let Some(bin) = opt_bin {
            cmd.flag_value("--bin", bin);
        } else {
            cmd.arg("--bin");
        }
        has_explicit_build_target = true;
    }

    if let Some(opt_example) = &args.example {
        if let Some(example) = opt_example {
            cmd.flag_value("--example", example);
        } else {
            cmd.arg("--example");
        }
        has_explicit_build_target = true;
    }

    if let Some(opt_test) = &args.test {
        if let Some(test) = opt_test {
            cmd.flag_value("--test", test);
        } else {
            cmd.arg("--test");
        }
        has_explicit_build_target = true;
    }

    if let Some(opt_bench) = &args.bench {
        if let Some(bench) = opt_bench {
            cmd.flag_value("--bench", bench);
        } else {
            cmd.arg("--bench");
        }
        has_explicit_build_target = true;
    }

    if !has_explicit_build_target {
        if let Ok(cargo_manifest) = manifest::parse(args.manifest_path.as_deref()) {
            if let Some(root_package) = cargo_manifest.package {
                if let Some(default_run) = &root_package.default_run {
                    cmd.flag_value("--bin", default_run);
                }
            }
        }
    }

    if let Some(features) = &args.features {
        cmd.flag_value("--features", features);
    }

    if args.all_features {
        cmd.arg("--all-features");
    }

    if args.no_default_features {
        cmd.arg("--no-default-features");
    }

    if let Some(jobs) = args.jobs {
        cmd.flag_value("--jobs", jobs.to_string());
    }

    if let Some(profile) = &args.profile {
        cmd.flag_value("--profile", profile);
    } else if args.tests && args.test.is_none() {
        if args.release {
            cmd.flag_value("--profile", "bench");
        } else {
            cmd.flag_value("--profile", "test");
        }
    } else if args.release {
        cmd.flag_value("--profile", "release");
    } else {
        cmd.flag_value("--profile", "check");
    }

    if let Some(target) = &args.target {
        cmd.flag_value("--target", target);
    }

    if let Some(target_dir) = &args.target_dir {
        cmd.flag_value("--target-dir", target_dir);
    }

    if let Some(manifest_path) = &args.manifest_path {
        cmd.flag_value("--manifest-path", manifest_path);
    }

    if args.frozen {
        cmd.arg("--frozen");
    }

    if args.locked {
        cmd.arg("--locked");
    }

    if args.offline {
        cmd.arg("--offline");
    }

    cmd.arg("--");

    cmd.arg("-o");
    cmd.arg(outfile);
    cmd.arg(ARG_Z_UNPRETTY_EXPANDED);
}

fn needs_rustc_bootstrap() -> bool {
    if env::var_os("RUSTC_BOOTSTRAP").is_some_and(|var| !var.is_empty()) {
        return false;
    }

    let rustc = if let Some(rustc) = env::var_os("RUSTC") {
        PathBuf::from(rustc)
    } else {
        let mut cmd = Command::new(cargo_binary());
        cmd.arg("rustc");
        cmd.arg("-Zunstable-options");
        cmd.flag_value("--print", "sysroot");
        cmd.env("RUSTC_BOOTSTRAP", "1");
        cmd.stdin(Stdio::null());
        cmd.stderr(Stdio::null());
        let Ok(output) = cmd.output() else {
            return true;
        };
        let Ok(stdout) = str::from_utf8(&output.stdout) else {
            return true;
        };
        let sysroot = Path::new(stdout.trim_end());
        sysroot.join("bin").join("rustc")
    };

    let rustc_wrapper = env::var_os("RUSTC_WRAPPER").filter(|wrapper| !wrapper.is_empty());
    let rustc_workspace_wrapper =
        env::var_os("RUSTC_WORKSPACE_WRAPPER").filter(|wrapper| !wrapper.is_empty());
    let mut wrapped_rustc = rustc_wrapper
        .into_iter()
        .chain(rustc_workspace_wrapper)
        .chain(iter::once(rustc.into_os_string()));

    let mut cmd = Command::new(wrapped_rustc.next().unwrap());
    cmd.args(wrapped_rustc);
    cmd.arg("-Zunpretty=expanded");
    cmd.arg("-");
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());
    let Ok(status) = cmd.status() else {
        return true;
    };
    !status.success()
}

fn print_command(cmd: &Command, color: Coloring) -> Result<()> {
    let mut shell_words = String::new();
    let quoter = shlex::Quoter::new().allow_nul(true);
    for arg in cmd.get_args() {
        let arg_lossy = arg.to_string_lossy();
        shell_words.push(' ');
        match arg_lossy.split_once('=') {
            Some((flag, value)) if flag.starts_with('-') && flag == quoter.quote(flag)? => {
                shell_words.push_str(flag);
                shell_words.push('=');
                if !value.is_empty() {
                    shell_words.push_str(&quoter.quote(value)?);
                }
            }
            _ => shell_words.push_str(&quoter.quote(&arg_lossy)?),
        }
    }

    let color_choice = match color {
        Coloring::Auto => ColorChoice::Auto,
        Coloring::Always => ColorChoice::Always,
        Coloring::Never => ColorChoice::Never,
    };

    let mut stream = StandardStream::stderr(color_choice);
    let _ = stream.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Green)));
    let _ = write!(stream, "{:>12}", "Running");
    let _ = stream.reset();
    let _ = writeln!(stream, " `cargo +nightly{}`", shell_words);
    Ok(())
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
    #[allow(deprecated)] // https://github.com/dtolnay/cargo-expand/issues/229
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

fn print_themes() -> Result<()> {
    let mut cache_dir = assets::cache_dir()?;
    let metadata = AssetsMetadata::load_from_folder(&cache_dir)?;
    let compatible = metadata
        .as_ref()
        .is_some_and(|m| m.is_compatible_with(assets::BAT_VERSION));
    let assets = if compatible {
        HighlightingAssets::from_cache(&cache_dir)?
    } else {
        HighlightingAssets::from_binary()
    };

    for theme in assets.themes() {
        let _ = writeln!(io::stdout(), "{}", theme);
    }

    if metadata.is_some() && !compatible {
        if let Some(home_dir) = home::home_dir() {
            if let Ok(relative) = cache_dir.strip_prefix(home_dir) {
                cache_dir = Path::new("~").join(relative);
            }
        }
        let bat_version = semver::Version::parse(assets::BAT_VERSION).unwrap();
        let _ = writeln!(
            io::stderr(),
            "\nThere may be other themes in {cache_dir} but they are not \
             compatible with the version of bat built into cargo-expand. Run \
             `bat cache --build` with bat v{major}.{minor} to update the cache.",
            cache_dir = cache_dir.display(),
            major = bat_version.major,
            minor = bat_version.minor,
        );
    }

    Ok(())
}
