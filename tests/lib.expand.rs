#![feature(prelude_import)]
#![doc = " Test"]
#[prelude_import]
use std::prelude::v1::*;
#[macro_use]
extern crate std;
#[doc = " Test"]
pub fn test() {}
