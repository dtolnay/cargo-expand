use clap::{AppSettings, Parser};
use std::fmt::{self, Display};
use std::path::PathBuf;
use std::str::FromStr;
use syn_select::Selector;

#[derive(Parser)]
#[clap(bin_name = "cargo", version)]
pub enum Opts {
    /// Show the result of macro expansion.
    #[clap(
        name = "expand",
        version,
        setting = AppSettings::DeriveDisplayOrder,
        setting = AppSettings::DontCollapseArgsInUsage
    )]
    Expand(Args),
}

#[derive(Parser, Debug)]
pub struct Args {
    /// Space-separated list of features to activate
    #[clap(long, value_name = "FEATURES")]
    pub features: Option<String>,

    /// Activate all available features
    #[clap(long)]
    pub all_features: bool,

    /// Do not activate the `default` feature
    #[clap(long)]
    pub no_default_features: bool,

    /// Expand only this package's library
    #[clap(long)]
    pub lib: bool,

    /// Expand only the specified binary
    #[clap(long, value_name = "NAME")]
    pub bin: Option<String>,

    /// Expand only the specified example
    #[clap(long, value_name = "NAME")]
    pub example: Option<String>,

    /// Expand only the specified test target
    #[clap(long, value_name = "NAME")]
    pub test: Option<String>,

    /// Include tests when expanding the lib or bin
    #[clap(long)]
    pub tests: bool,

    /// Expand only the specified bench target
    #[clap(long, value_name = "NAME")]
    pub bench: Option<String>,

    /// Target triple which compiles will be for
    #[clap(long, value_name = "TARGET")]
    pub target: Option<String>,

    /// Directory for all generated artifacts
    #[clap(long, value_name = "DIRECTORY", parse(from_os_str))]
    pub target_dir: Option<PathBuf>,

    /// Path to Cargo.toml
    #[clap(long, value_name = "PATH", parse(from_os_str))]
    pub manifest_path: Option<PathBuf>,

    /// Package to expand
    #[clap(short, long, value_name = "SPEC")]
    pub package: Option<String>,

    /// Build artifacts in release mode, with optimizations
    #[clap(long)]
    pub release: bool,

    /// Number of parallel jobs, defaults to # of CPUs
    #[clap(short, long, value_name = "N")]
    pub jobs: Option<u64>,

    /// Print command lines as they are executed
    #[clap(long)]
    pub verbose: bool,

    /// Coloring: auto, always, never
    #[clap(long, value_name = "WHEN")]
    pub color: Option<Coloring>,

    /// Require Cargo.lock and cache are up to date
    #[clap(long)]
    pub frozen: bool,

    /// Require Cargo.lock is up to date
    #[clap(long)]
    pub locked: bool,

    /// Run without accessing the network
    #[clap(long)]
    pub offline: bool,

    /// Unstable (nightly-only) flags to Cargo
    #[clap(short = 'Z', value_name = "FLAG")]
    pub unstable_flags: Vec<String>,

    /// Do not attempt to run rustfmt
    #[clap(long)]
    pub ugly: bool,

    /// Select syntax highlighting theme
    #[clap(long, value_name = "NAME")]
    pub theme: Option<String>,

    /// Print available syntax highlighting theme names
    #[clap(long)]
    pub themes: bool,

    /// Local path to module or other named item to expand, e.g. os::unix::ffi
    #[clap(value_name = "ITEM", parse(try_from_str = parse_selector))]
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
