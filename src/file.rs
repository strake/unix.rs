use core::fmt;
use core::intrinsics::unreachable as unreach;
use core::mem;
use core::ops::*;
use core::ptr;
use core::slice;
use fallible::*;
use libc;
use null_terminated::Nul;
use io::*;
use isaac::Rng;
use rand::*;
use void::Void;

use err::*;
use random::*;
use str::*;
use time::*;

use self::OpenMode::*;

pub struct File {
    fd: isize
}

impl File {
    #[inline]
    pub fn chmod(&self, mode: FileMode) -> Result<(), OsErr> {
        unsafe { esyscall_!(FCHMOD, self.fd, mode.bits) }
    }

    #[inline]
    pub fn stat(&self) -> Result<Stat, OsErr> { unsafe {
        let mut st: libc::stat = mem::uninitialized();
        try!(esyscall!(FSTAT, self.fd, &mut st as *mut _));
        Ok(Stat::from(st))
    } }

    #[inline]
    pub fn sync(&self, metadata: bool) -> Result<(), OsErr> {
        unsafe { if !metadata { esyscall_!(FDATASYNC, self.fd) }
                 else         { esyscall_!(FSYNC,     self.fd) } }
    }

    #[inline]
    unsafe fn do_map(&self, loc: Option<*mut u8>, prot: Prot, seg: Option<Segment>) ->
      Result<Map, OsErr> {
        let Segment { offset, length } = seg.unwrap_or(Segment { offset: 0, length: {
            let l = try!(self.stat()).size;
            if ((l as usize) as libc::off_t) != l {
                return Err(OsErr::from(libc::EOVERFLOW as usize));
            }
            l as _
        } });
        let ptr = syscall!(MMAP, loc.unwrap_or(ptr::null_mut()), length, prot.bits,
                           if loc.is_some() { libc::MAP_FIXED } else { 0 }, self.fd,
                           offset) as *mut u8;
        if (ptr as usize) > 0x1000usize.wrapping_neg() {
            Err(OsErr::from((ptr as usize).wrapping_neg()))
        } else { Ok(Map { ptr: ptr as *mut u8, length: length }) }
    }

    #[inline]
    pub fn map(&self, prot: Prot, seg: Option<Segment>) -> Result<Map, OsErr> {
        unsafe { self.do_map(None, prot, seg) }
    }

    #[inline]
    pub unsafe fn map_at(&self, loc: *mut u8, prot: Prot, seg: Option<Segment>) ->
      Result<Map, OsErr> { self.do_map(Some(loc), prot, seg) }

    #[cfg(target_os = "linux")]
    #[inline]
    pub fn exec(&self, argv: &Nul<&OsStr>, envp: &Nul<&OsStr>) -> Result<Void, OsErr> {
        exec_at(Some(self), str0!(""), argv, envp, AtFlags::Follow)
    }

    #[cfg(not(target_os = "linux"))]
    #[inline]
    pub fn exec(&self, argv: &Nul<&OsStr>, envp: &Nul<&OsStr>) -> Result<Void, OsErr> {
        unsafe { esyscall!(FEXECVE, self.fd, argv, envp).map(|_| unreach()) }
    }

    #[inline]
    pub fn fd(&self) -> isize { self.fd }

    #[inline]
    pub fn new(fd: isize) -> Option<Self> {
        if unsafe {
            syscall!(FCNTL, fd, ::libc::F_GETFD) as isize
        } >= 0 { Some(File { fd: fd }) } else { None }
    }

    #[inline]
    pub const fn new_unchecked(fd: isize) -> Self { File { fd: fd } }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Segment { pub offset: u64, pub length: usize }

#[inline]
pub fn mk_pipe(flags: OpenFlags) -> Result<(File, File), OsErr> { unsafe {
    let mut fds: [::libc::c_int; 2] = mem::uninitialized();
    try!(esyscall!(PIPE2, &mut fds as *mut _, flags.bits));
    Ok((File { fd: fds[0] as _ }, File { fd: fds[1] as _ }))
} }

#[inline]
pub fn open_at(opt_dir: Option<&File>, path: &OsStr, o_mode: OpenMode, flags: OpenFlags,
               f_mode: FileMode) -> Result<File, OsErr> {
    unsafe { esyscall!(OPENAT, from_opt_dir(opt_dir), path.as_ptr(),
                       flags.bits | o_mode.to_usize(), f_mode.bits) }
        .map(|fd| File { fd: fd as isize })
}

#[inline]
pub fn rename_at(opt_old_dir: Option<&File>, old_path: &OsStr,
                 opt_new_dir: Option<&File>, new_path: &OsStr) -> Result<(), OsErr> {
    unsafe { esyscall_!(RENAMEAT, from_opt_dir(opt_old_dir), old_path.as_ptr(),
                                  from_opt_dir(opt_new_dir), new_path.as_ptr()) }
}

#[inline]
pub fn link_at(opt_old_dir: Option<&File>, old_path: &OsStr,
               opt_new_dir: Option<&File>, new_path: &OsStr,
               at_flags: AtFlags) -> Result<(), OsErr> {
    unsafe { esyscall_!(LINKAT, from_opt_dir(opt_old_dir), old_path.as_ptr(),
                                from_opt_dir(opt_new_dir), new_path.as_ptr(),
                        if at_flags.contains(AtFlags::Follow) { ::libc::AT_SYMLINK_FOLLOW }
                        else { 0 } | AT_EMPTY_PATH) }
}

#[inline]
pub fn unlink_at(opt_dir: Option<&File>, path: &OsStr) -> Result<(), OsErr> {
    unsafe { esyscall_!(UNLINKAT, from_opt_dir(opt_dir), path.as_ptr()) }
}

#[inline]
pub fn chmod_at(opt_dir: Option<&File>, path: &OsStr,
                mode: FileMode, at_flags: AtFlags) -> Result<(), OsErr> {
    unsafe { esyscall_!(FCHMODAT, from_opt_dir(opt_dir), path.as_ptr(), mode.bits,
                        if at_flags.contains(AtFlags::Follow) { 0 }
                        else { libc::AT_SYMLINK_NOFOLLOW }) }
}

#[inline]
pub fn stat_at(opt_dir: Option<&File>, path: &OsStr,
               at_flags: AtFlags) -> Result<Stat, OsErr> { unsafe {
    let fl = if at_flags.contains(AtFlags::Follow) { 0 } else { libc::AT_SYMLINK_NOFOLLOW }
           | AT_EMPTY_PATH;
    let fd = from_opt_dir(opt_dir);
    let mut st: libc::stat = mem::uninitialized();
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    try!(esyscall_!(NEWFSTATAT, fd, path.as_ptr(), &mut st as *mut _, fl));
    #[cfg(all(target_os = "linux", not(target_arch = "x86_64")))]
    try!(esyscall_!(FSTATAT64,  fd, path.as_ptr(), &mut st as *mut _, fl));
    #[cfg(not(target_os = "linux"))]
    try!(esyscall_!(FSTATAT,    fd, path.as_ptr(), &mut st as *mut _, fl));
    Ok(Stat::from(st))
} }

#[cfg(target_os = "linux")]
#[inline]
pub fn exec_at(opt_dir: Option<&File>, path: &OsStr,
               argv: &Nul<&OsStr>, env: &Nul<&OsStr>, at_flags: AtFlags) -> Result<Void, OsErr> {
    unsafe { esyscall!(EXECVEAT, from_opt_dir(opt_dir), path.as_ptr(),
                       argv.as_ptr(), env.as_ptr(),
                       if at_flags.contains(AtFlags::Follow) { 0 }
                       else { libc::AT_SYMLINK_NOFOLLOW } | AT_EMPTY_PATH).map(|_| unreach()) }
}

#[inline]
fn from_opt_dir(opt_dir: Option<&File>) -> isize {
    match opt_dir {
        None => libc::AT_FDCWD as isize,
        Some(dir) => dir.fd,
    }
}

#[inline]
pub fn mktemp_at<R: Clone, Rng: RandomGen>
  (opt_dir: Option<&File>, templ: &mut OsStr, range: R, rng: &mut Rng, flags: OpenFlags) ->
  Result<File, OsErr> where [u8]: IndexMut<R, Output = [u8]> {
    const EEXIST: usize = libc::EEXIST as usize;

    let tries = 0x100;
    for _ in 0..tries {
        randname(rng, &mut templ[range.clone()]);
        match open_at(opt_dir, templ, RdWr, flags | O_CREAT | O_EXCL, Perm::Read << USR) {
            Err(OsErr::Unknown(EEXIST)) => (),
            r_f => return r_f,
        }
    }
    Err(OsErr::from(EEXIST))
}

#[inline]
fn randname<Rng: RandomGen>(rng: &mut Rng, bs: &mut [u8]) {
    let base = 'Z' as u64 - '@' as u64;
    let mut n: u64 = rng.gen();
    for p in bs.iter_mut() {
        *p = (n % base + 'A' as u64) as u8;
        n /= base;
    }
}

pub enum Clobber {
    NoClobber,
    Clobber,
    ClobberSavingPerms,
}

pub use self::Clobber::*;

#[inline]
pub fn atomic_write_file_at<F: FnOnce(File) -> Result<T, OsErr>, T>
  (opt_dir: Option<&File>, path: &OsStr,
   clobber: Clobber, mode: FileMode, writer: F) -> Result<T, OsErr> {
    let mut rng: Rng = OsRandom::new().gen();

    let mut temp_path = [b' '; 13];
    { let l = temp_path.len(); temp_path[l - 1] = 0; }
    let temp_path_ref = <&mut OsStr>::try_from(&mut temp_path[..]).unwrap();
    let f = try!(mktemp_at(opt_dir, temp_path_ref, 0..12, &mut rng, OpenFlags::empty()));
    try!(f.chmod(match clobber {
        NoClobber | Clobber => mode,
        ClobberSavingPerms => match stat_at(opt_dir, path, AtFlags::empty()) {
            Ok(st) => st.mode,
            Err(OsErr::Unknown(c)) if libc::ENOENT == c as _ => mode,
            Err(e) => return Err(e),
        },
    }));

    let m = try!(writer(f));

    match clobber {
        NoClobber => {
            try!(link_at(opt_dir, temp_path_ref, opt_dir, path, AtFlags::empty()));
            let _ = unlink_at(opt_dir, temp_path_ref);
        },
        Clobber | ClobberSavingPerms => try!(rename_at(opt_dir, temp_path_ref, opt_dir, path)),
    }
    Ok(m)
}

impl TryClone for File {
    type Error = OsErr;
    #[inline]
    fn try_clone(&self) -> Result<Self, OsErr> {
        unsafe { esyscall!(DUP, self.fd) }.map(|fd| File { fd: fd as _ })
    }
    #[inline]
    fn try_clone_from(&mut self, other: &Self) -> Result<(), OsErr> {
        unsafe { esyscall_!(DUP2, other.fd, self.fd) }
    }
}

impl Drop for File {
    #[inline]
    fn drop(&mut self) { unsafe { syscall!(CLOSE, self.fd) }; }
}

impl Read<u8> for File {
    type Err = OsErr;

    #[inline]
    fn readv(&mut self, bufs: &mut [&mut [u8]]) -> Result<usize, Self::Err> {
        unsafe { esyscall!(READV, self.fd, bufs.as_mut_ptr(), bufs.len()) }
    }
}

impl Write<u8> for File {
    type Err = OsErr;

    #[inline]
    fn writev(&mut self, bufs: &[&[u8]]) -> Result<usize, Self::Err> {
        unsafe { esyscall!(WRITEV, self.fd, bufs.as_ptr(), bufs.len()) }
    }

    #[inline]
    fn flush(&mut self) -> Result<(), Self::Err> { self.sync(false) }
}

impl fmt::Write for File {
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_all(s.as_bytes()).map_err(|_| fmt::Error)
    }
}

bitflags! {
    pub struct FileMode: u16 {
        const SUID = 0o4000;
        const SGID = 0o2000;
        const SVTX = 0o1000;
        #[doc(hidden)]
        const ____ = 0o0777;
    }
}

bitflags! {
    pub struct FilePermission: u8 {
        const Read  = 4;
        const Write = 2;
        const Exec  = 1;
    }
}
use self::FilePermission as Perm;

impl Shl<FileModeSection> for FilePermission {
    type Output = FileMode;
    #[inline]
    fn shl(self, sect: FileModeSection) -> FileMode {
        FileMode::from_bits_truncate((self.bits as u16) << sect.pos())
    }
}

impl Shr<FileModeSection> for FileMode {
    type Output = FilePermission;
    #[inline]
    fn shr(self, sect: FileModeSection) -> FilePermission {
        FilePermission::from_bits_truncate((self.bits >> sect.pos() & 3) as _)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FileModeSection { USR, GRP, OTH }
pub use self::FileModeSection::*;

impl FileModeSection {
    #[inline]
    fn pos(self) -> u32 { match self { USR => 6, GRP => 3, OTH => 0 } }
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
    pub struct OpenFlags: usize {
        const O_CLOEXEC  = libc::O_CLOEXEC  as usize;
        const O_CREAT    = libc::O_CREAT    as usize;
        const O_EXCL     = libc::O_EXCL     as usize;
        const O_NONBLOCK = libc::O_NONBLOCK as usize;
    }
}
pub const O_CLOEXEC : OpenFlags = OpenFlags::O_CLOEXEC;
pub const O_CREAT   : OpenFlags = OpenFlags::O_CREAT;
pub const O_EXCL    : OpenFlags = OpenFlags::O_EXCL;
pub const O_NONBLOCK: OpenFlags = OpenFlags::O_NONBLOCK;

bitflags! {
    pub struct AtFlags: usize {
        const Follow = libc::AT_SYMLINK_FOLLOW as usize;
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
    pub struct Prot: usize {
        const PROT_EXEC  = libc::PROT_EXEC  as usize;
        const PROT_READ  = libc::PROT_READ  as usize;
        const PROT_WRITE = libc::PROT_WRITE as usize;
    }
}

pub struct Stat {
    pub dev:     libc::dev_t,
    pub ino:     libc::ino_t,
    pub mode:    FileMode,
    pub nlink:   libc::nlink_t,
    pub uid:     libc::uid_t,
    pub gid:     libc::gid_t,
    pub size:    libc::off_t,
    pub blksize: libc::blksize_t,
    pub blocks:  libc::blkcnt_t,
    pub atime:   EpochTime,
    pub mtime:   EpochTime,
    pub ctime:   EpochTime,
}

impl From<libc::stat> for Stat {
    #[inline(always)]
    fn from(st: libc::stat) -> Self {
        Stat {
            dev: st.st_dev,
            ino: st.st_ino,
            mode: FileMode::from_bits_truncate(st.st_mode as _),
            nlink: st.st_nlink,
            uid: st.st_uid,
            gid: st.st_gid,
            size: st.st_size,
            blksize: st.st_blksize,
            blocks: st.st_blocks,
            atime: EpochTime::from_s_ns(st.st_atime, st.st_atime_nsec),
            mtime: EpochTime::from_s_ns(st.st_mtime, st.st_mtime_nsec),
            ctime: EpochTime::from_s_ns(st.st_ctime, st.st_ctime_nsec),
        }
    }
}

#[cfg(target_os = "linux")]
const AT_EMPTY_PATH: libc::c_int = libc::AT_EMPTY_PATH;
#[cfg(not(target_os = "linux"))]
const AT_EMPTY_PATH: libc::c_int = 0;
