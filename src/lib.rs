#![no_std]

#![deny(missing_debug_implementations)]

#![feature(const_fn)]
#![feature(core_intrinsics)]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate syscall;
extern crate fallible;
extern crate idem;
extern crate io;
extern crate libc;
#[macro_use]
extern crate null_terminated;
extern crate rand;
extern crate time as tempus;
extern crate void;

#[macro_use]
pub mod err;
pub mod env;
pub mod file;
pub mod poll;
pub mod process;
pub mod mem;
pub mod str { pub type OsStr = ::null_terminated::Nul<u8>; }
pub mod time;

mod random;
mod util;
