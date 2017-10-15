use core::mem;
use core::ops::*;
use core::ptr;
use core::slice;
use libc;
use io::*;
use rand::*;

pub use libc::stat as Stat;

use err::*;
use str::*;

use self::OpenMode::*;

pub struct File {
    fd: isize
}

impl File {
    #[inline]
    pub fn stat(&self) -> Result<Stat, OsErr> {
        let mut st = unsafe { mem::uninitialized() };
        OsErr::from_sysret(unsafe { syscall!(FSTAT, self.fd, &mut st as *mut libc::stat) }
                           as isize).map(|_| st)
    }

    #[inline]
    pub fn map(&self, loc: Option<*mut u8>, prot: Prot, offset: u64, length: usize) ->
      Result<Map, OsErr> {
        let ptr = unsafe { syscall!(MMAP, loc.unwrap_or(ptr::null_mut()), length, prot.bits,
                                    if loc.is_some() { libc::MAP_FIXED } else { 0 }, self.fd,
                                    offset) } as *mut u8;
        if (ptr as usize) > 0x1000usize.wrapping_neg() {
            Err(OsErr::from((ptr as usize).wrapping_neg()))
        } else { Ok(Map { ptr: ptr as *mut u8, length: length }) }
    }

    #[inline]
    pub fn map_full(&self, loc: Option<*mut u8>, prot: Prot) -> Result<Map, OsErr> {
        let l = try!(self.stat()).st_size;
        if ((l as usize) as libc::off_t) != l { Err(OsErr::from(libc::EOVERFLOW as usize)) }
        else { self.map(loc, prot, 0, l as usize) }
    }
}

#[inline]
pub fn open_at(opt_dir: Option<&File>, path: &OsStr, o_mode: OpenMode, flags: OpenFlags,
               f_mode: FileMode) -> Result<File, OsErr> {
    OsErr::from_sysret(unsafe { syscall!(OPENAT, from_opt_dir(opt_dir), path.as_ptr(),
                                         flags.bits | o_mode.to_usize(), f_mode.to_usize()) }
                       as isize).map(|fd| File { fd: fd as isize })
}

#[inline]
pub fn rename_at(opt_old_dir: Option<&File>, old_path: &OsStr,
                 opt_new_dir: Option<&File>, new_path: &OsStr) -> Result<(), OsErr> {
    OsErr::from_sysret(unsafe { syscall!(RENAMEAT,
                                         from_opt_dir(opt_old_dir), old_path.as_ptr(),
                                         from_opt_dir(opt_new_dir), new_path.as_ptr()) }
                       as isize).map(|_| ())
}

#[inline]
pub fn link_at(opt_old_dir: Option<&File>, old_path: &OsStr,
               opt_new_dir: Option<&File>, new_path: &OsStr) -> Result<(), OsErr> {
    OsErr::from_sysret(unsafe { syscall!(LINKAT,
                                         from_opt_dir(opt_old_dir), old_path.as_ptr(),
                                         from_opt_dir(opt_new_dir), new_path.as_ptr()) }
                       as isize).map(|_| ())
}

#[inline]
pub fn unlink_at(opt_dir: Option<&File>, path: &OsStr) -> Result<(), OsErr> {
    OsErr::from_sysret(unsafe { syscall!(UNLINKAT, from_opt_dir(opt_dir), path.as_ptr()) }
                       as isize).map(|_| ())
}

#[inline]
fn from_opt_dir(opt_dir: Option<&File>) -> isize {
    match opt_dir {
        None => libc::AT_FDCWD as isize,
        Some(dir) => dir.fd,
    }
}

#[inline]
pub fn mktemp_at<R: Clone, TheRng: Rng>
  (opt_dir: Option<&File>, templ: &mut OsStr, range: R, rng: &mut TheRng, flags: OpenFlags) ->
  Result<File, OsErr> where [u8]: IndexMut<R, Output = [u8]> {
    const EEXIST: usize = libc::EEXIST as usize;

    let tries = 0x100;
    for _ in 0..tries {
        randname(rng, &mut templ[range.clone()]);
        match open_at(opt_dir, templ, RdWr, flags | O_CREAT | O_EXCL,
                      { let mut f_m = FileMode::empty(); f_m.usr = S_IR; f_m }) {
            Err(OsErr::Unknown(EEXIST)) => (),
            r_f => return r_f,
        }
    }
    Err(OsErr::from(EEXIST))
}

#[inline]
fn randname<TheRng: Rng>(rng: &mut TheRng, bs: &mut [u8]) {
    let base = 'Z' as u64 - '@' as u64;
    let mut n: u64 = rng.gen();
    for p in bs.iter_mut() {
        *p = (n % base + 'A' as u64) as u8;
        n /= base;
    }
}

#[inline]
pub fn atomic_write_file_at<F: FnOnce(File) -> Result<T, OsErr>, T>
  (opt_dir: Option<&File>, path: &OsStr, overwrite: bool, writer: F) -> Result<T, OsErr> {
    let mut rng = {
        let mut seed = 0u64;
        try!(get_entropy(unsafe { slice::from_raw_parts_mut(&mut seed as *mut _ as *mut u8,
                                                            mem::size_of_val(&seed)) }));
        Isaac64Rng::from_seed(&[seed])
    };

    let mut temp_path = [0; 13];
    let temp_path_ref = OsStr::from_mut_bytes(&mut temp_path);
    let f = try!(mktemp_at(opt_dir, temp_path_ref, 0..12, &mut rng, OpenFlags::empty()));
    let m = try!(writer(f));
    if overwrite {
        try!(rename_at(opt_dir, temp_path_ref, opt_dir, path));
    } else {
        try!(link_at(opt_dir, temp_path_ref, opt_dir, path));
        let _ = unlink_at(opt_dir, temp_path_ref);
    }
    Ok(m)
}

#[inline]
fn get_entropy(bs: &mut [u8]) -> Result<(), OsErr> {
    try!(open_at(None, OsStr::from_bytes(b"/dev/urandom\0"), RdOnly, OpenFlags::empty(),
                 FileMode::empty())).read_full(bs).map_err(|(e, _)| e)
}

impl Drop for File {
    #[inline]
    fn drop(&mut self) { unsafe { syscall!(CLOSE, self.fd) }; }
}

impl Read<u8> for File {
    type Err = OsErr;

    #[inline]
    fn readv(&mut self, bufs: &mut [&mut [u8]]) -> Result<usize, Self::Err> {
        OsErr::from_sysret(unsafe { syscall!(READV, self.fd, bufs.as_mut_ptr(), bufs.len()) }
                           as isize)
    }
}

impl Write<u8> for File {
    type Err = OsErr;

    #[inline]
    fn writev(&mut self, bufs: &[&[u8]]) -> Result<usize, Self::Err> {
        OsErr::from_sysret(unsafe { syscall!(WRITEV, self.fd, bufs.as_ptr(), bufs.len()) }
                           as isize)
    }

    #[inline]
    fn flush(&mut self) -> Result<(), Self::Err> {
        OsErr::from_sysret(unsafe { syscall!(FSYNC, self.fd) } as isize).map(|_| ())
    }
}

pub struct FileMode {
    pub usr: FilePermission,
    pub grp: FilePermission,
    pub oth: FilePermission,
    pub suid: bool,
    pub sgid: bool,
    pub svtx: bool,
}

impl FileMode {
    #[inline] pub fn empty() -> Self {
        FileMode {
            usr: FilePermission::empty(),
            grp: FilePermission::empty(),
            oth: FilePermission::empty(),
            suid: false,
            sgid: false,
            svtx: false,
        }
    }

    #[inline] fn to_usize(self) -> usize {
        (self.suid as usize) << 11 |
        (self.sgid as usize) << 10 |
        (self.svtx as usize) << 09 |
        (self.usr.bits as usize) << 6 |
        (self.grp.bits as usize) << 3 |
        (self.oth.bits as usize) << 0 |
        0
    }
}

impl BitOr for FileMode {
    type Output = Self;
    fn bitor(self, other: Self) -> Self {
        FileMode {
            usr: self.usr | other.usr,
            grp: self.grp | other.grp,
            oth: self.oth | other.oth,
            suid: self.suid | other.suid,
            sgid: self.sgid | other.sgid,
            svtx: self.svtx | other.svtx,
        }
    }
}

bitflags! {
    pub flags FilePermission: u8 {
        const S_IR = 4,
        const S_IW = 2,
        const S_IX = 1,
    }
}

pub enum OpenMode {
    RdOnly,
    WrOnly,
    RdWr,
}

impl OpenMode {
    #[inline] fn to_usize(self) -> usize {
        match self {
            RdOnly => libc::O_RDONLY as usize,
            WrOnly => libc::O_WRONLY as usize,
            RdWr   => libc::O_RDWR   as usize,
        }
    }
}

bitflags! {
    pub flags OpenFlags: usize {
        const O_CLOEXEC  = libc::O_CLOEXEC  as usize,
        const O_CREAT    = libc::O_CREAT    as usize,
        const O_EXCL     = libc::O_EXCL     as usize,
        const O_NONBLOCK = libc::O_NONBLOCK as usize,
    }
}

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
    pub flags Prot: usize {
        const PROT_EXEC  = libc::PROT_EXEC  as usize,
        const PROT_READ  = libc::PROT_READ  as usize,
        const PROT_WRITE = libc::PROT_WRITE as usize,
    }
}
