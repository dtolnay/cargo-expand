use std::fmt::{self, Display};

const CARGO_EXPAND_VERSION: &str = env!("CARGO_PKG_VERSION");
const PRETTYPLEASE_VERSION: Option<&str> =
    include!(concat!(env!("OUT_DIR"), "/prettyplease.version"));

pub(crate) struct Version {
    pub verbose: bool,
}

impl Display for Version {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("cargo-expand ")?;
        formatter.write_str(CARGO_EXPAND_VERSION)?;
        if self.verbose {
            if let Some(prettyplease_version) = PRETTYPLEASE_VERSION {
                formatter.write_str(" + prettyplease ")?;
                formatter.write_str(prettyplease_version)?;
            }
        }
        Ok(())
    }
}
