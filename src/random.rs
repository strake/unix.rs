use core::{mem, slice};

use rand::RandomGen;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OsRandom(());

impl OsRandom {
    #[inline]
    pub fn new() -> Self { OsRandom(()) }

    pub unsafe fn next<T: Copy>(&mut self) -> T {
        let mut x: T = mem::uninitialized();
        self.fill_bytes(slice::from_raw_parts_mut(&mut x as *mut _ as *mut u8,
                                                  mem::size_of::<T>()));
        x
    }
}

impl RandomGen for OsRandom {
    fn gen_u32(&mut self) -> u32 { unsafe { self.next() } }
    fn gen_u64(&mut self) -> u64 { unsafe { self.next() } }

    #[cfg(target_os = "linux")]
    fn fill_bytes(&mut self, bs: &mut [u8]) {
        let mut p = &mut bs[0] as *mut u8;
        let mut l = bs.len() as isize;
        while l > 0 { unsafe {
            let n = syscall!(GETRANDOM, p, l, 0) as isize;
            if n == -::libc::EINTR as _ { continue; }
            assert!(n >= 0, "failed to get entropy");
            p = p.offset(n);
            l -= n;
        } }
    }

    #[cfg(target_os = "openbsd")]
    fn fill_bytes(&mut self, bs: &mut [u8]) {
        assert_eq!(0, unsafe { syscall!(GETENTROPY, bs.as_mut_ptr(), bs.len()) },
                   "failed to get entropy");
    }
}
