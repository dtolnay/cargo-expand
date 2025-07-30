#![feature(prelude_import)]
//! Test
#[macro_use]
extern crate std;
#[prelude_import]
use std::prelude::rust_2021::*;
/// Test
pub fn test() {}
