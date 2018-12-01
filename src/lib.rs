//! Unix system types and operations

#![no_std]

#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

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
mod err_;
mod env_;

pub mod dir;
pub mod file;
pub mod poll;
pub mod process;
pub mod mem;
pub mod time;

mod random;
mod util;

pub use err_::Error;
pub use env_::{Environ, environ};
pub use file::File;

/// Null-terminated byte string, used in many Unix system interfaces
pub type Str = ::null_terminated::Nul<u8>;

#[deprecated(note = "Use associated constants of `Error`")]
#[allow(missing_docs)]
pub mod err {
    pub use Error as OsErr;
    include!(concat!(env!("OUT_DIR"), "/error_consts.rs"));
}

#[deprecated]
#[allow(missing_docs)]
pub mod env { pub use env_::*; }

#[deprecated]
#[allow(missing_docs)]
pub mod str { pub use Str as OsStr; }
