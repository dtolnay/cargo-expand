use std::path::PathBuf;

use structopt::clap::AppSettings;
use structopt::StructOpt;

use crate::filter::Filter;

#[derive(StructOpt)]
#[structopt(bin_name = "cargo")]
pub enum Opts {
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
pub struct Args {
    /// Space-separated list of features to activate
    #[structopt(long, value_name = "FEATURES")]
    pub features: Option<String>,

    /// Activate all available features
    #[structopt(long)]
    pub all_features: bool,

    /// Do not activate the `default` feature
    #[structopt(long)]
    pub no_default_features: bool,

    /// Build only this package's library
    #[structopt(long)]
    pub lib: bool,

    /// Build only the specified binary
    #[structopt(long, value_name = "NAME")]
    pub bin: Option<String>,

    /// Build only the specified example
    #[structopt(long, value_name = "NAME")]
    pub example: Option<String>,

    /// Build only the specified test target
    #[structopt(long, value_name = "NAME")]
    pub test: Option<String>,

    /// Build only the specified bench target
    #[structopt(long, value_name = "NAME")]
    pub bench: Option<String>,

    /// Target triple which compiles will be for
    #[structopt(long, value_name = "TARGET")]
    pub target: Option<String>,

    /// Directory for all generated artifacts
    #[structopt(long, value_name = "DIRECTORY", parse(from_os_str))]
    pub target_dir: Option<PathBuf>,

    /// Path to Cargo.toml
    #[structopt(long, value_name = "PATH", parse(from_os_str))]
    pub manifest_path: Option<PathBuf>,

    /// Number of parallel jobs, defaults to # of CPUs
    #[structopt(short, long, value_name = "N")]
    pub jobs: Option<u64>,

    /// Coloring: auto, always, never
    #[structopt(long, value_name = "WHEN")]
    pub color: Option<String>,

    /// Require Cargo.lock and cache are up to date
    #[structopt(long)]
    pub frozen: bool,

    /// Require Cargo.lock is up to date
    #[structopt(long)]
    pub locked: bool,

    /// Unstable (nightly-only) flags to Cargo
    #[structopt(short = "Z", value_name = "FLAG")]
    pub unstable_flags: Vec<String>,

    /// Do not attempt to run rustfmt
    #[structopt(long)]
    pub ugly: bool,

    /// Local path to module or other named item to expand, e.g. os::unix::ffi
    #[structopt(value_name = "ITEM")]
    pub item: Option<Filter>,
}
