#![no_std]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate syscall;
extern crate io;
extern crate isaac;
extern crate libc;
extern crate random as rand;

pub mod args;
pub mod err;
pub mod file;
pub mod str;

mod random;
