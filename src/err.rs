use libc;
use libreal::io::*;

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
