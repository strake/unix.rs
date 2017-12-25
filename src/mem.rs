use core::ops::{Deref, DerefMut};
use core::{ptr, slice};
use err::*;
use file::*;
use libc;
use util::*;

pub struct Map {
    ptr: *mut u8,
    length: usize,
}

impl Deref for Map {
    type Target = [u8];
    #[inline]
    fn deref(&self) -> &[u8] { unsafe { slice::from_raw_parts(self.ptr, self.length) } }
}

impl DerefMut for Map {
    #[inline]
    fn deref_mut(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.ptr, self.length) }
    }
}

impl Drop for Map {
    #[inline]
    fn drop(&mut self) {
        unsafe { syscall!(MUNMAP, self.ptr, self.length) };
    }
}

bitflags! {
    struct Prot: usize {
        const PROT_EXEC  = libc::PROT_EXEC  as usize;
        const PROT_READ  = libc::PROT_READ  as usize;
        const PROT_WRITE = libc::PROT_WRITE as usize;
    }
}

impl From<Perm> for Prot {
    #[inline]
    fn from(perm: Perm) -> Prot {
        let mut prot = Prot::empty();
        for &(prt, prm) in &[(Prot::PROT_READ,  Perm::Read),
                             (Prot::PROT_WRITE, Perm::Write),
                             (Prot::PROT_EXEC,  Perm::Exec)] {
            if perm.contains(prm) { prot |= prt; }
        }
        prot
    }
}

pub trait MapExt {
    type MapSpec;

    fn map(&self, perm: Perm, seg: Self::MapSpec) -> Result<Map, OsErr>;
    unsafe fn map_at(&self, loc: *mut u8, perm: Perm, seg: Self::MapSpec) -> Result<Map, OsErr>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Segment { pub offset: u64, pub length: usize }

impl MapExt for File {
    type MapSpec = Option<Segment>;

    #[inline]
    fn map(&self, perm: Perm, seg: Option<Segment>) -> Result<Map, OsErr> {
        unsafe { do_map_file(self, ptr::null_mut(), perm, seg) }
    }

    #[inline]
    unsafe fn map_at(&self, loc: *mut u8, perm: Perm, seg: Option<Segment>) ->
      Result<Map, OsErr> { do_map_file(self, loc, perm, seg) }
}

impl MapExt for () {
    type MapSpec = usize;

    #[inline]
    fn map(&self, perm: Perm, length: usize) -> Result<Map, OsErr> {
        unsafe { do_map(-1, ptr::null_mut(), perm, 0, length) }
    }

    #[inline]
    unsafe fn map_at(&self, loc: *mut u8, perm: Perm, length: usize) -> Result<Map, OsErr> {
        do_map(-1, loc, perm, 0, length)
    }
}

#[inline]
unsafe fn do_map_file(f: &File, loc: *mut u8, perm: Perm, seg: Option<Segment>) ->
  Result<Map, OsErr> {
    let Segment { offset, length } = seg.unwrap_or(Segment {
        offset: 0, length: try_to_usize(f.stat()?.size as _)?
    });
    do_map(f.fd(), loc, perm, offset, length)
}

#[inline]
unsafe fn do_map(fd: isize, loc: *mut u8, perm: Perm, offset: u64, length: usize) ->
  Result<Map, OsErr> {
    let ptr = syscall!(MMAP, loc, length, Prot::from(perm).bits,
                       if loc.is_null() { 0 } else { libc::MAP_FIXED }, fd, offset) as *mut u8;
    if (ptr as usize) > 0x1000usize.wrapping_neg() {
        Err(OsErr::from((ptr as usize).wrapping_neg()))
    } else { Ok(Map { ptr: ptr as *mut u8, length: length }) }
}