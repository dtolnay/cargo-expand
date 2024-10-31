use clap::{Parser, ValueEnum};
use std::path::PathBuf;
use std::str::FromStr;
use syn_select::Selector;

// Help headings
const PACKAGE_SELECTION: &str = "Package Selection";
const TARGET_SELECTION: &str = "Target Selection";
const FEATURE_SELECTION: &str = "Feature Selection";
const COMPILATION_OPTIONS: &str = "Compilation Options";
const MANIFEST_OPTIONS: &str = "Manifest Options";

#[derive(Parser)]
#[command(
    bin_name = "cargo",
    version,
    author,
    disable_help_subcommand = true,
    styles = clap_cargo::style::CLAP_STYLING,
)]
pub enum Subcommand {
    /// Show the result of macro expansion.
    #[command(name = "expand", version, author, disable_version_flag = true)]
    Expand(Expand),
}

#[derive(Parser, Debug)]
pub struct Expand {
    /// Do not attempt to run rustfmt
    #[arg(long)]
    pub ugly: bool,

    /// Select syntax highlighting theme
    #[arg(long, value_name = "NAME")]
    pub theme: Option<String>,

    /// Print available syntax highlighting theme names
    #[arg(long)]
    pub themes: bool,

    /// Print command lines as they are executed
    #[arg(long)]
    pub verbose: bool,

    /// Syntax highlighting and colored Cargo output (auto, always, never)
    #[arg(long, value_name = "WHEN", hide_possible_values = true)]
    pub color: Option<Coloring>,

    /// Override a configuration value
    #[arg(long, value_name = "KEY=VALUE")]
    pub config: Vec<String>,

    /// Unstable (nightly-only) flags to Cargo
    #[arg(short = 'Z', value_name = "FLAG")]
    pub unstable_flags: Vec<String>,

    /// Print version
    #[arg(long)]
    pub version: bool,

    /// Package to expand
    #[arg(short, long, value_name = "SPEC", num_args = 0..=1, help_heading = PACKAGE_SELECTION)]
    pub package: Option<Option<String>>,

    /// Expand only this package's library
    #[arg(long, help_heading = TARGET_SELECTION)]
    pub lib: bool,

    /// Expand only the specified binary
    #[arg(long, value_name = "NAME", num_args = 0..=1, help_heading = TARGET_SELECTION)]
    pub bin: Option<Option<String>>,

    /// Expand only the specified example
    #[arg(long, value_name = "NAME", num_args = 0..=1, help_heading = TARGET_SELECTION)]
    pub example: Option<Option<String>>,

    /// Expand only the specified test target
    #[arg(long, value_name = "NAME", num_args = 0..=1, help_heading = TARGET_SELECTION)]
    pub test: Option<Option<String>>,

    /// Include tests when expanding the lib or bin
    #[arg(long, help_heading = TARGET_SELECTION)]
    pub tests: bool,

    /// Expand only the specified bench target
    #[arg(long, value_name = "NAME", num_args = 0..=1, help_heading = TARGET_SELECTION)]
    pub bench: Option<Option<String>>,

    /// Space or comma separated list of features to activate
    #[arg(short = 'F', long, value_name = "FEATURES", help_heading = FEATURE_SELECTION)]
    pub features: Option<String>,

    /// Activate all available features
    #[arg(long, help_heading = FEATURE_SELECTION)]
    pub all_features: bool,

    /// Do not activate the `default` feature
    #[arg(long, help_heading = FEATURE_SELECTION)]
    pub no_default_features: bool,

    /// Number of parallel jobs, defaults to # of CPUs
    #[arg(short, long, value_name = "N", help_heading = COMPILATION_OPTIONS)]
    pub jobs: Option<u64>,

    /// Build artifacts in release mode, with optimizations
    #[arg(long, help_heading = COMPILATION_OPTIONS)]
    pub release: bool,

    /// Build artifacts with the specified profile
    #[arg(long, value_name = "PROFILE-NAME", help_heading = COMPILATION_OPTIONS)]
    pub profile: Option<String>,

    /// Target triple which compiles will be for
    #[arg(long, value_name = "TARGET", help_heading = COMPILATION_OPTIONS)]
    pub target: Option<String>,

    /// Directory for all generated artifacts
    #[arg(long, value_name = "DIRECTORY", help_heading = COMPILATION_OPTIONS)]
    pub target_dir: Option<PathBuf>,

    /// Path to Cargo.toml
    #[arg(long, value_name = "PATH", help_heading = MANIFEST_OPTIONS)]
    pub manifest_path: Option<PathBuf>,

    /// Require Cargo.lock and cache are up to date
    #[arg(long, help_heading = MANIFEST_OPTIONS)]
    pub frozen: bool,

    /// Require Cargo.lock is up to date
    #[arg(long, help_heading = MANIFEST_OPTIONS)]
    pub locked: bool,

    /// Run without accessing the network
    #[arg(long, help_heading = MANIFEST_OPTIONS)]
    pub offline: bool,

    /// Local path to module or other named item to expand, e.g. os::unix::ffi
    #[arg(value_name = "ITEM", value_parser = parse_selector)]
    pub item: Option<Selector>,
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
