# cargo-expand

[<img alt="github" src="https://img.shields.io/badge/github-dtolnay/cargo--expand-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/dtolnay/cargo-expand)
[<img alt="crates.io" src="https://img.shields.io/crates/v/cargo-expand.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/cargo-expand)
[<img alt="build status" src="https://img.shields.io/github/actions/workflow/status/dtolnay/cargo-expand/ci.yml?branch=master&style=for-the-badge" height="20">](https://github.com/dtolnay/cargo-expand/actions?query=branch%3Amaster)

Once installed, the following command prints out the result of macro expansion
and `#[derive]` expansion applied to the current crate.

```console
$ cargo expand
```

This is a wrapper around the more verbose compiler command:

```console
$ cargo rustc --profile=check -- -Zunpretty=expanded
```

## Installation

Install with **`cargo install cargo-expand`**.

This command optionally uses [rustfmt] to format the expanded output. The
resulting code is typically much more readable than what you get from the
compiler. If rustfmt is not available, the expanded code is not formatted.
Install rustfmt with **`rustup component add rustfmt`**.

Cargo expand relies on unstable compiler flags so it requires a nightly
toolchain to be installed, though does not require nightly to be the default
toolchain or the one with which cargo expand itself is executed. If the default
toolchain is one other than nightly, running `cargo expand` will find and use
nightly anyway.

[rustfmt]: https://github.com/rust-lang/rustfmt

## Example

#### `$ cat src/main.rs`

```rust
#[derive(Debug)]
struct S;

fn main() {
    println!("{:?}", S);
}
```

#### `$ cargo expand`

```rust
#![feature(prelude_import)]
#[prelude_import]
use std::prelude::v1::*;
#[macro_use]
extern crate std;
struct S;
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for S {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            S => {
                let mut debug_trait_builder = f.debug_tuple("S");
                debug_trait_builder.finish()
            }
        }
    }
}
fn main() {
    {
        ::std::io::_print(::core::fmt::Arguments::new_v1(
            &["", "\n"],
            &match (&S,) {
                (arg0,) => [::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Debug::fmt)],
            },
        ));
    };
}
```

## Options

*See `cargo expand --help` for a complete list of options, most of which are
consistent with other Cargo subcommands. Here are a few that are common in the
context of cargo expand.*

To expand a particular test target:

`$ cargo expand --test test_something`

To expand without rustfmt:

`$ cargo expand --ugly`

To expand a specific module or type or function only:

`$ cargo expand path::to::module`

[![cargo expand punctuated::printing][punctuated.png]][syn]
[![cargo expand token::FatArrow][fatarrow.png]][syn]

[punctuated.png]: https://raw.githubusercontent.com/dtolnay/cargo-expand/screenshots/punctuated.png
[fatarrow.png]: https://raw.githubusercontent.com/dtolnay/cargo-expand/screenshots/fatarrow.png
[syn]: https://github.com/dtolnay/syn

## Configuration

The cargo expand command reads the `[expand]` section of $CARGO_HOME/config.toml
if there is one (usually ~/.cargo/config.toml).

Set the default syntax highlighting theme with the `theme` setting:

```toml
[expand]
theme = "TwoDark"
```

Run `cargo expand --themes` or `bat --list-themes` to print a list of available
themes. Use `theme = "none"` to disable coloring.

Change the default coloring disposition (normally `auto`) with the `color`
setting:

```toml
[expand]
color = "always"
```

Enable paging of the output with the `pager` setting:

```toml
[expand]
pager = true
```

## Disclaimer

Be aware that macro expansion to text is a lossy process. This is a debugging
aid only. There should be no expectation that the expanded code can be compiled
successfully, nor that if it compiles then it behaves the same as the original
code.

For instance the following function returns `3` when compiled ordinarily by Rust
but the expanded code compiles and returns `4`.

```rust
fn f() -> i32 {
    let x = 1;

    macro_rules! first_x {
        () => { x }
    }

    let x = 2;

    x + first_x!()
}
```

Refer to [The Book] for more on the considerations around macro hygiene.

[The Book]: https://doc.rust-lang.org/1.30.0/book/first-edition/macros.html#hygiene

<br>

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
