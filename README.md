# cargo-expand

[![Build Status](https://travis-ci.org/dtolnay/cargo-expand.svg?branch=master)](https://travis-ci.org/dtolnay/cargo-expand)
[![Latest Version](https://img.shields.io/crates/v/cargo-expand.svg)](https://crates.io/crates/cargo-expand)

This is a wrapper around `cargo rustc -- --pretty=expanded`. Once installed, the
command `cargo expand` prints out the result of macro expansion applied to the
current crate.

## Installation

Install with `cargo install cargo-expand`.

This command optionally uses
[`rustfmt`](https://github.com/rust-lang-nursery/rustfmt)
to format the expanded output. If `rustfmt` is not available, the expanded code
is not formatted. Install `rustfmt` with `cargo install rustfmt`.

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in cargo-expand by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
