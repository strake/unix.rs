use libc;
use io::*;

use self::OsErr::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OsErr {
    Unknown(usize),
}

impl OsErr {
    #[inline] pub fn from_sysret(m: isize) -> Result<usize, Self> {
        if m < 0 { Err(Self::from(-m as usize)) } else { Ok(m as usize) }
    }
}

impl From<usize> for OsErr {
    #[inline] fn from(n: usize) -> Self { Unknown(n) }
}

impl From<EndOfFile> for OsErr {
    #[inline] fn from(_: EndOfFile) -> Self { Unknown(0) }
}

impl From<NoMemory> for OsErr {
    #[inline] fn from(_: NoMemory) -> Self { Unknown(libc::ENOMEM as usize) }
}

macro_rules! esyscall {
    ($n:ident $(, $a:expr)*) => (::err::OsErr::from_sysret(syscall!($n $(, $a)*) as isize))
}

macro_rules! esyscall_ { ($n:ident $(, $a:expr)*) => (esyscall!($n $(, $a)*).map(|_| ())) }
