use core::fmt;
use io::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct OsErr(pub usize);

impl fmt::Debug for OsErr {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match error_names.get(self.0) {
            Some(&Some(s)) => write!(f, "{}", s),
            _ => write!(f, "OsErr({})", self.0),
        }
    }
}

impl OsErr {
    #[inline] pub fn from_sysret(m: isize) -> Result<usize, Self> {
        if m < 0 { Err(Self::from(-m as usize)) } else { Ok(m as usize) }
    }
}

impl From<usize> for OsErr {
    #[inline] fn from(n: usize) -> Self { OsErr(n) }
}

impl From<EndOfFile> for OsErr {
    #[inline] fn from(_: EndOfFile) -> Self { OsErr(0) }
}

impl From<NoMemory> for OsErr {
    #[inline] fn from(_: NoMemory) -> Self { ENOMEM }
}

include!(concat!(env!("OUT_DIR"), "/e.rs"));

#[macro_export]
macro_rules! esyscall {
    ($n:ident $(, $a:expr)*) =>
        ($crate::err::OsErr::from_sysret(syscall!($n $(, $a)*) as isize))
}

#[macro_export]
macro_rules! esyscall_ { ($n:ident $(, $a:expr)*) => (esyscall!($n $(, $a)*).map(|_| ())) }
