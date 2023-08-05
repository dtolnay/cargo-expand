use clap::{Parser, ValueEnum};
use std::path::PathBuf;
use std::str::FromStr;
use syn_select::Selector;

#[derive(Parser)]
#[command(bin_name = "cargo", version, author, disable_help_subcommand = true)]
pub enum Subcommand {
    /// Show the result of macro expansion.
    #[command(name = "expand", version, author, disable_version_flag = true)]
    Expand(Expand),
}

#[derive(Parser, Debug)]
pub struct Expand {
    /// Space-separated list of features to activate
    #[arg(long, value_name = "FEATURES")]
    pub features: Option<String>,

    /// Activate all available features
    #[arg(long)]
    pub all_features: bool,

    /// Do not activate the `default` feature
    #[arg(long)]
    pub no_default_features: bool,

    /// Expand only this package's library
    #[arg(long)]
    pub lib: bool,

    /// Expand only the specified binary
    #[arg(long, value_name = "NAME", num_args = 0..=1)]
    pub bin: Option<Option<String>>,

    /// Expand only the specified example
    #[arg(long, value_name = "NAME", num_args = 0..=1)]
    pub example: Option<Option<String>>,

    /// Expand only the specified test target
    #[arg(long, value_name = "NAME", num_args = 0..=1)]
    pub test: Option<Option<String>>,

    /// Include tests when expanding the lib or bin
    #[arg(long)]
    pub tests: bool,

    /// Expand only the specified bench target
    #[arg(long, value_name = "NAME", num_args = 0..=1)]
    pub bench: Option<Option<String>>,

    /// Target triple which compiles will be for
    #[arg(long, value_name = "TARGET")]
    pub target: Option<String>,

    /// Directory for all generated artifacts
    #[arg(long, value_name = "DIRECTORY")]
    pub target_dir: Option<PathBuf>,

    /// Path to Cargo.toml
    #[arg(long, value_name = "PATH")]
    pub manifest_path: Option<PathBuf>,

    /// Package to expand
    #[arg(short, long, value_name = "SPEC", num_args = 0..=1)]
    pub package: Option<Option<String>>,

    /// Build artifacts in release mode, with optimizations
    #[arg(long)]
    pub release: bool,

    /// Build artifacts with the specified profile
    #[arg(long, value_name = "PROFILE-NAME")]
    pub profile: Option<String>,

    /// Number of parallel jobs, defaults to # of CPUs
    #[arg(short, long, value_name = "N")]
    pub jobs: Option<u64>,

    /// Print command lines as they are executed
    #[arg(long)]
    pub verbose: bool,

    /// Coloring: auto, always, never
    #[arg(long, value_name = "WHEN")]
    pub color: Option<Coloring>,

    /// Require Cargo.lock and cache are up to date
    #[arg(long)]
    pub frozen: bool,

    /// Require Cargo.lock is up to date
    #[arg(long)]
    pub locked: bool,

    /// Run without accessing the network
    #[arg(long)]
    pub offline: bool,

    /// Unstable (nightly-only) flags to Cargo
    #[arg(short = 'Z', value_name = "FLAG")]
    pub unstable_flags: Vec<String>,

    /// Do not attempt to run rustfmt
    #[arg(long)]
    pub ugly: bool,

    /// Select syntax highlighting theme
    #[arg(long, value_name = "NAME")]
    pub theme: Option<String>,

    /// Print available syntax highlighting theme names
    #[arg(long)]
    pub themes: bool,

    /// Local path to module or other named item to expand, e.g. os::unix::ffi
    #[arg(value_name = "ITEM", value_parser = parse_selector)]
    pub item: Option<Selector>,

    /// Print version
    #[arg(long)]
    pub version: bool,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum Coloring {
    Auto,
    Always,
    Never,
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
    <Subcommand as clap::CommandFactory>::command().debug_assert();
}
