use core::{fmt, num::NonZeroUsize};
use io::*;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct OsErr(pub NonZeroUsize);

impl fmt::Debug for OsErr {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match error_names.get(self.0.get()) {
            Some(&Some(s)) => write!(f, "{}", s),
            _ => write!(f, "OsErr({})", self.0),
        }
    }
}

impl OsErr {
    #[inline] pub fn from_sysret(m: isize) -> Result<usize, Self> {
        if m < 0 { Err(Self::from(unsafe { NonZeroUsize::new_unchecked(-m as usize) })) }
        else { Ok(m as usize) }
    }

    #[inline]
    pub fn message(self) -> &'static str { error_messages.get(self.0.get()).unwrap_or(&"") }
}

impl From<NonZeroUsize> for OsErr {
    #[inline] fn from(n: NonZeroUsize) -> Self { OsErr(n) }
}

impl From<EndOfFile> for OsErr {
    #[inline] fn from(_: EndOfFile) -> Self { OsErr(unsafe { NonZeroUsize::new_unchecked(!0) }) }
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
