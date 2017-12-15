use core::mem;
use core::ops::*;
use core::slice;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct OsStr([u8]);

impl OsStr {
    #[inline]
    pub fn from_bytes(bs: &[u8]) -> &Self {
        Self::try_from_bytes(bs).expect("argument not null-terminated")
    }

    #[inline]
    pub fn from_mut_bytes(bs: &mut [u8]) -> &mut Self {
        Self::try_from_mut_bytes(bs).expect("argument not null-terminated")
    }

    #[inline]
    pub fn try_from_bytes(bs: &[u8]) -> Option<&Self> {
        if Some(&0) != bs.last() { None } else { Some(unsafe { mem::transmute(bs) }) }
    }

    #[inline]
    pub fn try_from_mut_bytes(bs: &mut [u8]) -> Option<&mut Self> {
        if Some(&0) != bs.last() { None } else { Some(unsafe { mem::transmute(bs) }) }
    }

    #[inline]
    pub unsafe fn from_ptr(ptr: *const u8) -> &'static Self {
        Self::from_mut_ptr(ptr as *mut u8)
    }

    #[inline]
    pub unsafe fn from_mut_ptr(ptr: *mut u8) -> &'static mut Self {
        let mut i = 0;
        while 0 != *ptr.offset(i) { i += 1 };
        mem::transmute(slice::from_raw_parts_mut(ptr, (i as usize)+1))
    }

    #[inline]
    pub fn as_ptr(&self) -> *const u8 { &self[0] }
}

impl Deref for OsStr {
    type Target = [u8];
    #[inline] fn deref(&self) -> &[u8] { &self.0[0..self.0.len()-1] }
}

impl DerefMut for OsStr {
    #[inline] fn deref_mut(&mut self) -> &mut [u8] { let l = self.0.len(); &mut self.0[0..l-1] }
}
