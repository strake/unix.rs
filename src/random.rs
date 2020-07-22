//! Random number generation

use core::{mem, slice};

use rand::{CryptoRng, RngCore};

use Error;

/// Random numbers via system call
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OsRandom(());

impl OsRandom {
    #[allow(missing_docs)]
    #[inline]
    pub fn new() -> Self { OsRandom(()) }

    /// Generate a value.
    #[inline]
    pub unsafe fn next<T: Copy>(&mut self) -> T {
        self.try_next().unwrap()
    }

    /// Generate a value.
    #[inline]
    pub unsafe fn try_next<T: Copy>(&mut self) -> Result<T, ::rand::Error> {
        let mut x = mem::MaybeUninit::<T>::uninit();
        self.try_fill_bytes(slice::from_raw_parts_mut(&mut x as *mut _ as *mut u8,
                                                      mem::size_of::<T>()))?;
        Ok(x.assume_init())
    }
}

impl RngCore for OsRandom {
    #[inline] fn next_u32(&mut self) -> u32 { unsafe { self.next() } }
    #[inline] fn next_u64(&mut self) -> u64 { unsafe { self.next() } }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    #[inline]
    fn fill_bytes(&mut self, bs: &mut [u8]) { try_fill_bytes_getrandom(bs, true).unwrap() }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    #[inline]
    fn try_fill_bytes(&mut self, bs: &mut [u8]) -> Result<(), ::rand::Error> {
        try_fill_bytes_getrandom(bs, false)
    }
}

impl CryptoRng for OsRandom {}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
#[inline(always)]
fn try_fill_bytes_getrandom(bs: &mut [u8], block: bool) -> Result<(), ::rand::Error> {
    const GRND_NONBLOCK: usize = 1;
    let mut p = &mut bs[0] as *mut u8;
    let mut l = bs.len() as isize;
    while l > 0 { unsafe {
        match esyscall!(GETRANDOM, p, l, if block { 0 } else { GRND_NONBLOCK }) {
            Err(Error::EINTR)  => continue,
            Err(Error::ENOSYS) =>
                      return Err(::rand::Error::new(::rand::ErrorKind::Unavailable, "")),
            Err(Error::EAGAIN) =>
                      return Err(::rand::Error::new(::rand::ErrorKind::NotReady, "")),
            Err(e) => return Err(::rand::Error::new(::rand::ErrorKind::Unexpected, e.message())),
            Ok(n) => {
                let n = n as isize;
                p = p.offset(n);
                l -= n;
            },
        }
    } }
    Ok(())
}
