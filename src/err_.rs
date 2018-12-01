use core::{fmt, num::NonZeroUsize};
use io::*;

/// System error
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Error(pub NonZeroUsize);

impl fmt::Debug for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match error_names.get(self.0.get()) {
            Some(&Some(s)) => write!(f, "{}", s),
            _ => write!(f, "Error({})", self.0),
        }
    }
}

impl Error {
    #[doc(hidden)]
    #[inline]
    pub fn from_sysret(m: isize) -> Result<usize, Self> {
        if m < 0 { Err(Self::from(unsafe { NonZeroUsize::new_unchecked(-m as usize) })) }
        else { Ok(m as usize) }
    }

    /// Return the message for the error.
    #[inline]
    pub fn message(self) -> &'static str { error_messages.get(self.0.get()).unwrap_or(&"") }
}

impl From<NonZeroUsize> for Error {
    #[inline] fn from(n: NonZeroUsize) -> Self { Error(n) }
}

impl From<EndOfFile> for Error {
    #[inline] fn from(_: EndOfFile) -> Self { Error(unsafe { NonZeroUsize::new_unchecked(!0) }) }
}

impl From<NoMemory> for Error {
    #[inline] fn from(_: NoMemory) -> Self { Self::ENOMEM }
}

include!(concat!(env!("OUT_DIR"), "/e.rs"));

#[macro_export]
macro_rules! esyscall {
    ($n:ident $(, $a:expr)*) =>
        ($crate::Error::from_sysret(syscall!($n $(, $a)*) as isize))
}

#[macro_export]
macro_rules! esyscall_ { ($n:ident $(, $a:expr)*) => (esyscall!($n $(, $a)*).map(|_| ())) }
