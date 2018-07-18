#![no_std]

#[macro_use]
extern crate syscall;
extern crate random as rand;
extern crate unix;

pub use unix::{env, mem, poll, process, str, time};

#[macro_use]
pub mod err;
pub mod file;
