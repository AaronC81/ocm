#![feature(never_type)]

mod fallible;
pub use fallible::*;

mod sentinel;
pub use sentinel::*;

mod collector;
pub use collector::*;
