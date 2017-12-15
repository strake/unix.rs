#![no_std]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate syscall;
extern crate fallible;
extern crate io;
extern crate isaac;
extern crate libc;
extern crate null_terminated;
extern crate random as rand;

#[macro_use]
pub mod err;
pub mod file;
pub mod str { pub type OsStr = ::null_terminated::Nul<u8>; }

mod random;
