use clap::{AppSettings, Parser};
use std::fmt::{self, Display};
use std::path::PathBuf;
use std::str::FromStr;
use syn_select::Selector;

#[derive(Parser)]
#[clap(bin_name = "cargo", version, author)]
pub enum Opts {
    /// Show the result of macro expansion.
    #[clap(
        name = "expand",
        version,
        author,
        setting = AppSettings::DeriveDisplayOrder,
        dont_collapse_args_in_usage = true
    )]
    Expand(Args),
}

#[derive(Parser, Debug)]
pub struct Args {
    /// Space-separated list of features to activate
    #[clap(long, value_name = "FEATURES", action)]
    pub features: Option<String>,

    /// Activate all available features
    #[clap(long, action)]
    pub all_features: bool,

    /// Do not activate the `default` feature
    #[clap(long, action)]
    pub no_default_features: bool,

    /// Expand only this package's library
    #[clap(long, action)]
    pub lib: bool,

    /// Expand only the specified binary
    #[clap(
        long,
        value_name = "NAME",
        min_values = 0,
        multiple_values = false,
        action
    )]
    pub bin: Option<Option<String>>,

    /// Expand only the specified example
    #[clap(
        long,
        value_name = "NAME",
        min_values = 0,
        multiple_values = false,
        action
    )]
    pub example: Option<Option<String>>,

    /// Expand only the specified test target
    #[clap(
        long,
        value_name = "NAME",
        min_values = 0,
        multiple_values = false,
        action
    )]
    pub test: Option<Option<String>>,

    /// Include tests when expanding the lib or bin
    #[clap(long, action)]
    pub tests: bool,

    /// Expand only the specified bench target
    #[clap(
        long,
        value_name = "NAME",
        min_values = 0,
        multiple_values = false,
        action
    )]
    pub bench: Option<Option<String>>,

    /// Target triple which compiles will be for
    #[clap(long, value_name = "TARGET", action)]
    pub target: Option<String>,

    /// Directory for all generated artifacts
    #[clap(long, value_name = "DIRECTORY", action)]
    pub target_dir: Option<PathBuf>,

    /// Path to Cargo.toml
    #[clap(long, value_name = "PATH", action)]
    pub manifest_path: Option<PathBuf>,

    /// Package to expand
    #[clap(
        short,
        long,
        value_name = "SPEC",
        min_values = 0,
        multiple_values = false,
        action
    )]
    pub package: Option<Option<String>>,

    /// Build artifacts in release mode, with optimizations
    #[clap(long, action)]
    pub release: bool,

    /// Number of parallel jobs, defaults to # of CPUs
    #[clap(short, long, value_name = "N", action)]
    pub jobs: Option<u64>,

    /// Print command lines as they are executed
    #[clap(long, action)]
    pub verbose: bool,

    /// Coloring: auto, always, never
    #[clap(long, value_name = "WHEN", action)]
    pub color: Option<Coloring>,

    /// Require Cargo.lock and cache are up to date
    #[clap(long, action)]
    pub frozen: bool,

    /// Require Cargo.lock is up to date
    #[clap(long, action)]
    pub locked: bool,

    /// Run without accessing the network
    #[clap(long, action)]
    pub offline: bool,

    /// Unstable (nightly-only) flags to Cargo
    #[clap(short = 'Z', value_name = "FLAG", action)]
    pub unstable_flags: Vec<String>,

    /// Do not attempt to run rustfmt
    #[clap(long, action)]
    pub ugly: bool,

    /// Select syntax highlighting theme
    #[clap(long, value_name = "NAME", action)]
    pub theme: Option<String>,

    /// Print available syntax highlighting theme names
    #[clap(long, action)]
    pub themes: bool,

    /// Local path to module or other named item to expand, e.g. os::unix::ffi
    #[clap(value_name = "ITEM", value_parser = parse_selector)]
    pub item: Option<Selector>,
}

#[derive(Debug, Clone, Copy)]
pub enum Coloring {
    Auto,
    Always,
    Never,
}

impl FromStr for Coloring {
    type Err = String;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "auto" => Ok(Coloring::Auto),
            "always" => Ok(Coloring::Always),
            "never" => Ok(Coloring::Never),
            other => Err(format!(
                "must be auto, always, or never, but found `{}`",
                other,
            )),
        }
    }
}

impl Display for Coloring {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            Coloring::Auto => "auto",
            Coloring::Always => "always",
            Coloring::Never => "never",
        };
        formatter.write_str(name)
    }
}

fn parse_selector(s: &str) -> Result<Selector, <Selector as FromStr>::Err> {
    if s.starts_with("::") {
        s[2..].parse()
    } else {
        s.parse()
    }
}

#[test]
fn test_cli() {
    <Opts as clap::CommandFactory>::command().debug_assert();
}
