#![feature(rand)]

#![no_std]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate syscall;
extern crate libc;
extern crate libreal;
extern crate rand;

pub mod args;
pub mod err;
pub mod file;
pub mod str;
