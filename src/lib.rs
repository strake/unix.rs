#![no_std]

#![feature(const_fn)]
#![feature(core_intrinsics)]
#![feature(i128_type)]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate syscall;
extern crate fallible;
extern crate io;
extern crate isaac;
extern crate libc;
#[macro_use]
extern crate null_terminated;
extern crate random as rand;
extern crate time as tempus;
extern crate void;

#[macro_use]
pub mod err;
pub mod file;
pub mod poll;
pub mod process;
pub mod mem;
pub mod str { pub type OsStr = ::null_terminated::Nul<u8>; }
pub mod time;

mod random;
mod util;
