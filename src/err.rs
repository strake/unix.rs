pub use unix::err::*;

#[macro_export]
macro_rules! esyscall {
    ($n:ident $(, $a:expr)*) =>
        ($crate::err::OsErr::from_sysret(syscall!($n $(, $a)*) as isize))
}

#[macro_export]
macro_rules! esyscall_ { ($n:ident $(, $a:expr)*) => (esyscall!($n $(, $a)*).map(|_| ())) }
