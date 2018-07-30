use core::fmt;
use core::hint::unreachable_unchecked as unreach;
use core::mem;
use core::ops::*;
use fallible::*;
use libc;
use null_terminated::Nul;
use io::*;
use rand::*;
use void::Void;

use err::*;
use str::*;
use time::*;
use util::*;

#[derive(Debug)]
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
    pub fn truncate(&self, length: u64) -> Result<(), OsErr> {
        unsafe { esyscall_!(FTRUNCATE, self.fd, try_to_usize(length)?) }
    }

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

#[inline]
pub fn new_pipe(flags: OpenFlags) -> Result<(File, File), OsErr> { unsafe {
    let mut fds: [::libc::c_int; 2] = mem::uninitialized();
    try!(esyscall!(PIPE2, &mut fds as *mut _, flags.bits));
    Ok((File { fd: fds[0] as _ }, File { fd: fds[1] as _ }))
} }

#[deprecated(since = "0.3.0", note = "use `new_pipe`")]
#[inline]
pub fn mk_pipe(flags: OpenFlags) -> Result<(File, File), OsErr> { new_pipe(flags) }

#[inline]
pub fn open_at(opt_dir: Option<&File>, path: &OsStr, o_mode: OpenMode,
               f_mode: FileMode) -> Result<File, OsErr> {
    unsafe { esyscall!(OPENAT, from_opt_dir(opt_dir), path.as_ptr(), o_mode.0, f_mode.bits) }
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
    unsafe { esyscall_!(UNLINKAT, from_opt_dir(opt_dir), path.as_ptr(), 0) }
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

pub fn mktemp_at<R: Clone, TheRng: Rng>
  (opt_dir: Option<&File>, templ: &mut OsStr, range: R, rng: &mut TheRng, flags: OpenFlags) ->
  Result<File, OsErr> where [u8]: IndexMut<R, Output = [u8]> {
    let tries = 0x100;
    for _ in 0..tries {
        randname(rng, &mut templ[range.clone()]);
        match open_at(opt_dir, templ, OpenMode::RdWr | flags | O_CREAT | O_EXCL,
                      Perm::Read << USR) {
            Err(EEXIST) => (),
            r_f => return r_f,
        }
    }
    Err(EEXIST)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Clobber {
    NoClobber,
    Clobber,
    ClobberSavingPerms,
}

pub use self::Clobber::*;

pub fn atomic_write_file_at<F: FnOnce(File) -> Result<T, OsErr>, T>
  (opt_dir: Option<&File>, path: &OsStr,
   clobber: Clobber, mode: FileMode, writer: F) -> Result<T, OsErr> {
    let mut rng = XorShiftRng::from_rng(::random::OsRandom::new())
        .map_err(|_| ::err::EIO)?;

    let mut temp_path = [b' '; 13];
    { let l = temp_path.len(); temp_path[l - 1] = 0; }
    let temp_path_ref = <&mut OsStr>::try_from(&mut temp_path[..]).unwrap();
    let f = try!(mktemp_at(opt_dir, temp_path_ref, 0..12, &mut rng, OpenFlags::empty()));

    struct Rm<'a, 'b> { opt_dir: Option<&'a File>, path: &'b OsStr };
    impl<'a, 'b> Drop for Rm<'a, 'b> {
        #[inline]
        fn drop(&mut self) { unlink_at(self.opt_dir, self.path).unwrap_or(()) }
    }
    let rm = Rm { opt_dir, path: temp_path_ref };

    try!(f.chmod(match clobber {
        NoClobber | Clobber => mode,
        ClobberSavingPerms => match stat_at(opt_dir, path, AtFlags::empty()) {
            Ok(st) => st.mode,
            Err(::err::ENOENT) => mode,
            Err(e) => return Err(e),
        },
    }));

    let m = try!(writer(f));

    match clobber {
        NoClobber => try!(link_at(opt_dir, temp_path_ref, opt_dir, path, AtFlags::empty())),
        Clobber | ClobberSavingPerms => {
            try!(rename_at(opt_dir, temp_path_ref, opt_dir, path));
            mem::forget(rm);
        },
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
pub(crate) use self::FilePermission as Perm;

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

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpenMode(usize);

impl OpenMode {
    pub const RdOnly: Self = OpenMode(libc::O_RDONLY as _);
    pub const WrOnly: Self = OpenMode(libc::O_WRONLY as _);
    pub const RdWr  : Self = OpenMode(libc::O_RDWR   as _);
}

impl BitOr<OpenFlags> for OpenMode {
    type Output = Self;
    #[inline]
    fn bitor(self, flags: OpenFlags) -> Self { OpenMode(self.0 | flags.bits) }
}

impl BitAnd<OpenFlags> for OpenMode {
    type Output = Self;
    #[inline]
    fn bitand(self, flags: OpenFlags) -> Self { OpenMode(self.0 & flags.bits) }
}

impl BitXor<OpenFlags> for OpenMode {
    type Output = Self;
    #[inline]
    fn bitxor(self, flags: OpenFlags) -> Self { OpenMode(self.0 ^ flags.bits) }
}

impl BitOrAssign<OpenFlags> for OpenMode {
    #[inline]
    fn bitor_assign(&mut self, flags: OpenFlags) { self.0 |= flags.bits }
}

impl BitAndAssign<OpenFlags> for OpenMode {
    #[inline]
    fn bitand_assign(&mut self, flags: OpenFlags) { self.0 &= flags.bits }
}

impl BitXorAssign<OpenFlags> for OpenMode {
    #[inline]
    fn bitxor_assign(&mut self, flags: OpenFlags) { self.0 ^= flags.bits }
}

bitflags! {
    pub struct OpenFlags: usize {
        const O_CLOEXEC  = libc::O_CLOEXEC  as usize;
        const O_CREAT    = libc::O_CREAT    as usize;
        const O_EXCL     = libc::O_EXCL     as usize;
        const O_NONBLOCK = libc::O_NONBLOCK as usize;
        const O_NOCTTY   = libc::O_NOCTTY   as usize;
    }
}
pub const O_CLOEXEC : OpenFlags = OpenFlags::O_CLOEXEC;
pub const O_CREAT   : OpenFlags = OpenFlags::O_CREAT;
pub const O_EXCL    : OpenFlags = OpenFlags::O_EXCL;
pub const O_NONBLOCK: OpenFlags = OpenFlags::O_NONBLOCK;
pub const O_NOCTTY  : OpenFlags = OpenFlags::O_NOCTTY;

bitflags! {
    pub struct AtFlags: usize {
        const Follow = libc::AT_SYMLINK_FOLLOW as usize;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
