use core::{mem, slice};

use rand::{CryptoRng, RngCore};

use Error;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OsRandom(());

impl OsRandom {
    #[inline]
    pub fn new() -> Self { OsRandom(()) }

    #[inline]
    pub unsafe fn next<T: Copy>(&mut self) -> T {
        let mut x: T = mem::uninitialized();
        self.fill_bytes(slice::from_raw_parts_mut(&mut x as *mut _ as *mut u8,
                                                  mem::size_of::<T>()));
        x
    }
}

impl RngCore for OsRandom {
    #[inline] fn next_u32(&mut self) -> u32 { unsafe { self.next() } }
    #[inline] fn next_u64(&mut self) -> u64 { unsafe { self.next() } }

    #[cfg(target_os = "linux")]
    #[inline]
    fn fill_bytes(&mut self, bs: &mut [u8]) { try_fill_bytes_linux(bs, true).unwrap() }

    #[cfg(target_os = "linux")]
    #[inline]
    fn try_fill_bytes(&mut self, bs: &mut [u8]) -> Result<(), ::rand::Error> {
        try_fill_bytes_linux(bs, false)
    }

    #[cfg(target_os = "openbsd")]
    #[inline]
    fn fill_bytes(&mut self, bs: &mut [u8]) { self.try_fill_bytes(bs).unwrap() }

    #[cfg(target_os = "openbsd")]
    #[inline]
    fn try_fill_bytes(&mut self, bs: &mut [u8]) -> Result<(), ::rand::Error> {
        for bs in bs.chunks_mut(0x100) {
            unsafe { esyscall!(GETENTROPY, bs.as_mut_ptr(), bs.len()) }
                .map_err(|e| ::rand::Error::new(::rand::ErrorKind::Unavailable, e.message()))?
        }
        Ok(())
    }
}

impl CryptoRng for OsRandom {}

#[cfg(target_os = "linux")]
#[inline(always)]
fn try_fill_bytes_linux(bs: &mut [u8], block: bool) -> Result<(), ::rand::Error> {
    let mut p = &mut bs[0] as *mut u8;
    let mut l = bs.len() as isize;
    while l > 0 { unsafe {
        match esyscall!(GETRANDOM, p, l, if block { 0 } else { ::libc::GRND_NONBLOCK }) {
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
