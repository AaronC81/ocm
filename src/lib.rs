#![feature(never_type)]

#[doc = include_str!("../README.md")]

mod outcome;
pub use outcome::*;

mod sentinel;
pub use sentinel::*;

mod collector;
pub use collector::*;
