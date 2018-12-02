//! File and filesystem operations
//!
//! Many functions here which take a path also take an `opt_dir` argument of type `Option<&File>`.
//! If the path is relative, it is interpreted relative to `opt_dir` if it is `Some`, else the current working directory.
//! Such functions are marked with an `_at` suffix.

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

use {Error, Str};
use time::*;
use util::*;

/// File descriptor, closed on drop
#[derive(Debug)]
pub struct File {
    fd: isize
}

impl File {
    /// Change the mode of the file.
    #[inline]
    pub fn chmod(&self, mode: FileMode) -> Result<(), Error> {
        unsafe { esyscall_!(FCHMOD, self.fd, mode.bits) }
    }

    /// Return information about the file.
    #[inline]
    pub fn stat(&self) -> Result<Stat, Error> { unsafe {
        let mut st: libc::stat = mem::uninitialized();
        try!(esyscall!(FSTAT, self.fd, &mut st as *mut _));
        Ok(Stat::from(st))
    } }

    /// Flush all cached modifications of the file to the device.
    /// If `metadata`, also flush modifications of the metadata.
    #[inline]
    pub fn sync(&self, metadata: bool) -> Result<(), Error> {
        unsafe { if !metadata { esyscall_!(FDATASYNC, self.fd) }
                 else         { esyscall_!(FSYNC,     self.fd) } }
    }

    /// Truncate the file to the given length.
    #[inline]
    pub fn truncate(&self, length: u64) -> Result<(), Error> {
        unsafe { esyscall_!(FTRUNCATE, self.fd, try_to_usize(length)?) }
    }

    /// Execute the program file.
    ///
    /// The current program of the calling process is replaced with the new one, with a fresh stack, heap, and data segment.
    /// `argv` is an array of argument strings to pass to the new program. By convention, the first should be the name of the program file.
    /// `env` is an array of strings, conventionally of form "key=value", which are passed as the environment.
    #[cfg(target_os = "linux")]
    #[inline]
    pub fn exec(&self, argv: &Nul<&Str>, envp: &Nul<&Str>) -> Result<Void, Error> {
        exec_at(Some(self), str0!(""), argv, envp, AtFlags::Follow)
    }

    /// Execute the program file.
    ///
    /// The current program of the calling process is replaced with the new one, with a fresh stack, heap, and data segment.
    /// `argv` is an array of argument strings to pass to the new program. By convention, the first should be the name of the program file.
    /// `env` is an array of strings, conventionally of form "key=value", which are passed as the environment.
    #[cfg(not(target_os = "linux"))]
    #[inline]
    pub fn exec(&self, argv: &Nul<&Str>, envp: &Nul<&Str>) -> Result<Void, Error> {
        unsafe { esyscall!(FEXECVE, self.fd, argv as *const _, envp as *const _).map(|_| unreach()) }
    }

    /// Return the file descriptor of the `File`.
    #[inline]
    pub fn fd(&self) -> isize { self.fd }

    /// Make a `File` of a file descriptor, which is checked for validity.
    #[inline]
    pub fn new(fd: isize) -> Option<Self> {
        if unsafe {
            syscall!(FCNTL, fd, ::libc::F_GETFD) as isize
        } >= 0 { Some(File { fd }) } else { None }
    }

    /// Make a `File` of a file descriptor, which is not checked.
    #[inline]
    pub const fn new_unchecked(fd: isize) -> Self { File { fd } }
}

/// Return the ends `(rx, tx)` of a new pipe. Data written to `tx` can be read from `rx`.
///
/// The reverse may also be true on some systems, but this behavior is not portable.
#[inline]
pub fn new_pipe(flags: OpenFlags) -> Result<(File, File), Error> { unsafe {
    let mut fds: [::libc::c_int; 2] = mem::uninitialized();
    try!(esyscall!(PIPE2, &mut fds as *mut _, flags.bits));
    Ok((File { fd: fds[0] as _ }, File { fd: fds[1] as _ }))
} }

#[deprecated(since = "0.3.0", note = "use `new_pipe`")]
pub use self::new_pipe as mk_pipe;

/// Open the file at the given path, creating it if `f_mode` is `Some` and it isn't already there.
#[inline]
pub fn open_at(opt_dir: Option<&File>, path: &Str, o_mode: OpenMode,
                 f_mode: Option<FileMode>) -> Result<File, Error> {
    unsafe { match f_mode {
        None => esyscall!(OPENAT, from_opt_dir(opt_dir), path.as_ptr(), o_mode.0),
        Some(f_mode) => esyscall!(OPENAT, from_opt_dir(opt_dir), path.as_ptr(),
                                  o_mode.0 | ::libc::O_CREAT as usize, f_mode.bits),
    } }.map(|fd| File { fd: fd as isize })
}

/// Rename the file from `old_path` to `new_path`.
#[inline]
pub fn rename_at(opt_old_dir: Option<&File>, old_path: &Str,
                 opt_new_dir: Option<&File>, new_path: &Str) -> Result<(), Error> {
    unsafe { esyscall_!(RENAMEAT, from_opt_dir(opt_old_dir), old_path.as_ptr(),
                                  from_opt_dir(opt_new_dir), new_path.as_ptr()) }
}

/// Link the file from `old_path` to `new_path`.
#[inline]
pub fn link_at(opt_old_dir: Option<&File>, old_path: &Str,
               opt_new_dir: Option<&File>, new_path: &Str,
               at_flags: AtFlags) -> Result<(), Error> {
    unsafe { esyscall_!(LINKAT, from_opt_dir(opt_old_dir), old_path.as_ptr(),
                                from_opt_dir(opt_new_dir), new_path.as_ptr(),
                        if at_flags.contains(AtFlags::Follow) { ::libc::AT_SYMLINK_FOLLOW }
                        else { 0 } | AT_EMPTY_PATH) }
}

/// Unlink the file at `path`.
///
/// (If it was the last link, the file will likely be deleted once the last open file descriptor to it is closed.)
#[inline]
pub fn unlink_at(opt_dir: Option<&File>, path: &Str) -> Result<(), Error> {
    unsafe { esyscall_!(UNLINKAT, from_opt_dir(opt_dir), path.as_ptr(), 0) }
}

/// Change the mode of the file at `path`.
#[inline]
pub fn chmod_at(opt_dir: Option<&File>, path: &Str,
                mode: FileMode, at_flags: AtFlags) -> Result<(), Error> {
    unsafe { esyscall_!(FCHMODAT, from_opt_dir(opt_dir), path.as_ptr(), mode.bits,
                        if at_flags.contains(AtFlags::Follow) { 0 }
                        else { libc::AT_SYMLINK_NOFOLLOW }) }
}

/// Return information about the file at `path`.
#[inline]
pub fn stat_at(opt_dir: Option<&File>, path: &Str,
               at_flags: AtFlags) -> Result<Stat, Error> { unsafe {
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

/// Execute the program file at `path`.
///
/// The current program of the calling process is replaced with the new one, with a fresh stack, heap, and data segment.
/// `argv` is an array of argument strings to pass to the new program. By convention, the first should be the name of the program file.
/// `env` is an array of strings, conventionally of form "key=value", which are passed as the environment.
#[cfg(target_os = "linux")]
#[inline]
pub fn exec_at(opt_dir: Option<&File>, path: &Str,
               argv: &Nul<&Str>, env: &Nul<&Str>, at_flags: AtFlags) -> Result<Void, Error> {
    unsafe { esyscall!(EXECVEAT, from_opt_dir(opt_dir), path.as_ptr(),
                       argv.as_ptr(), env.as_ptr(),
                       if at_flags.contains(AtFlags::Follow) { 0 }
                       else { libc::AT_SYMLINK_NOFOLLOW } | AT_EMPTY_PATH).map(|_| unreach()) }
}

#[inline]
pub(crate) fn from_opt_dir(opt_dir: Option<&File>) -> isize {
    match opt_dir {
        None => libc::AT_FDCWD as isize,
        Some(dir) => dir.fd,
    }
}

/// Generate a unique temporary file name in `templ`, and create and open a file there.
///
/// The given `range` of `templ` will be replaced with a string which uniquifies the file name. The contents of `range` after the call are unspecified.
/// The file is created with mode 600 and opened with `O_EXCL`, which ensures the caller created it.
pub fn mktemp_at<R: Clone, TheRng: Rng>
  (opt_dir: Option<&File>, templ: &mut Str, range: R, rng: &mut TheRng, flags: OpenFlags) ->
  Result<File, Error> where [u8]: IndexMut<R, Output = [u8]> {
    mktemp_helper(|path| open_at(opt_dir, path, OpenMode::RdWr | flags | O_EXCL, Some((Perm::Read | Perm::Write) << USR)),
                  templ, range, rng)
}

pub(crate) fn mktemp_helper<R: Clone, TheRng: Rng, F: Fn(&Str) -> Result<A, Error>, A>
  (f: F, templ: &mut Str, range: R, rng: &mut TheRng) -> Result<A, Error>
  where [u8]: IndexMut<R, Output = [u8]> {
    let tries = 0x100;
    for _ in 0..tries {
        randname(rng, &mut templ[range.clone()]);
        match f(templ) {
            Err(Error::EEXIST) => (),
            r => return r,
        }
    }
    Err(Error::EEXIST)
}

fn randname<TheRng: Rng>(rng: &mut TheRng, bs: &mut [u8]) {
    let base = 'Z' as u64 - '@' as u64;
    let mut n: u64 = rng.gen();
    for p in bs.iter_mut() {
        *p = (n % base + 'A' as u64) as u8;
        n /= base;
    }
}

/// Specify what to do if there is already a file at `path`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Clobber {
    /** Abort the operation.                               */ NoClobber,
    /** Clobber the extant file and its permissions.       */ Clobber,
    /** Clobber the extant file, but keep its permissions. */ ClobberSavingPerms,
}

pub use self::Clobber::*;

/// Atomically write the file at the given `path`:
///
/// call the given `writer`, and only once it finishes (and not fails), atomically replace the file with a newly-written one.
pub fn atomic_write_file_at<F: FnOnce(File) -> Result<T, Error>, T>
  (opt_dir: Option<&File>, path: &Str,
   clobber: Clobber, mode: FileMode, writer: F) -> Result<T, Error> {
    let mut rng = ::rand::rngs::SmallRng::from_rng(::random::OsRandom::new())
        .map_err(|_| Error::EIO)?;

    let mut temp_path = [b' '; 13];
    temp_path[temp_path.len() - 1] = 0;
    let temp_path_ref = <&mut Str>::try_from(&mut temp_path[..]).unwrap();
    let f = try!(mktemp_at(opt_dir, temp_path_ref, 0..12, &mut rng, OpenFlags::empty()));

    struct Rm<'a, 'b> { opt_dir: Option<&'a File>, path: &'b Str };
    impl<'a, 'b> Drop for Rm<'a, 'b> {
        #[inline]
        fn drop(&mut self) { unlink_at(self.opt_dir, self.path).unwrap_or(()) }
    }
    let rm = Rm { opt_dir, path: temp_path_ref };

    try!(f.chmod(match clobber {
        NoClobber | Clobber => mode,
        ClobberSavingPerms => match stat_at(opt_dir, path, AtFlags::empty()) {
            Ok(st) => st.mode,
            Err(Error::ENOENT) => mode,
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
    type Error = Error;
    #[inline]
    fn try_clone(&self) -> Result<Self, Error> {
        unsafe { esyscall!(DUP, self.fd) }.map(|fd| File { fd: fd as _ })
    }
    #[inline]
    fn try_clone_from(&mut self, other: &Self) -> Result<(), Error> {
        unsafe { esyscall_!(DUP2, other.fd, self.fd) }
    }
}

impl Drop for File {
    #[inline]
    fn drop(&mut self) { unsafe { syscall!(CLOSE, self.fd) }; }
}

impl Read<u8> for File {
    type Err = Error;

    #[inline]
    fn readv(&mut self, bufs: &mut [&mut [u8]]) -> Result<usize, Self::Err> {
        unsafe { esyscall!(READV, self.fd, bufs.as_mut_ptr(), bufs.len()) }
    }
}

impl Write<u8> for File {
    type Err = Error;

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
    /// File mode
    pub struct FileMode: u16 {
        /// Set-user-ID flag: if a process `exec`s the file, set its effective user ID to that of the file's owner.
        const SUID = 0o4000;
        /// Set-group-ID flag: If a process `exec`s the file, set its effective group ID to that of the file's owner.
        const SGID = 0o2000;
        /// Sticky bit
        const SVTX = 0o1000;
        #[doc(hidden)]
        const ____ = 0o0777;
    }
}

#[allow(missing_docs)]
mod file_permission { bitflags! {
    pub struct FilePermission: u8 {
        const Read  = 4;
        const Write = 2;
        const Exec  = 1;
    }
} } pub use self::file_permission::*;
pub(crate) use self::FilePermission as Perm;

impl Shl<FileModeSection> for FilePermission {
    type Output = FileMode;
    #[inline]
    fn shl(self, sect: FileModeSection) -> FileMode {
        FileMode::from_bits_truncate((self.bits() as u16) << sect.pos())
    }
}

impl Shr<FileModeSection> for FileMode {
    type Output = FilePermission;
    #[inline]
    fn shr(self, sect: FileModeSection) -> FilePermission {
        FilePermission::from_bits_truncate((self.bits >> sect.pos() & 3) as _)
    }
}

/// Which agent has the permissions
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FileModeSection {
    /** Owning user  */ USR,
    /** Owning group */ GRP,
    /** All others   */ OTH,
}
pub use self::FileModeSection::*;

impl FileModeSection {
    #[inline]
    fn pos(self) -> u32 { match self { USR => 6, GRP => 3, OTH => 0 } }
}

/// Whether to open a file for reading or writing
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpenMode(usize);

impl OpenMode {
    /** Read-only  */ pub const RdOnly: Self = OpenMode(libc::O_RDONLY as _);
    /** Write-only */ pub const WrOnly: Self = OpenMode(libc::O_WRONLY as _);
    /** Read-write */ pub const RdWr  : Self = OpenMode(libc::O_RDWR   as _);
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
    /// File creation and status flags
    pub struct OpenFlags: usize {
        /// Set the close-on-exec flag on the returned file descriptor.
        const O_CLOEXEC  = libc::O_CLOEXEC  as usize;
        /// Ensure the call creates the file; if the file already is, the system call fails with
        /// [`EEXIST`](../struct.Error.html#associatedconstant.EEXIST).
        const O_EXCL     = libc::O_EXCL     as usize;
        /// Open the file in non-blocking mode: operations which would block rather return
        /// [`EAGAIN`](../struct.Error.html#associatedconstant.EAGAIN) or
        /// [`EWOULDBLOCK`](../struct.Error.html#associatedconstant.EWOULDBLOCK).
        const O_NONBLOCK = libc::O_NONBLOCK as usize;
        /// The file will not become the caller's controlling TTY, even if the file is a TTY and the process has none already.
        const O_NOCTTY   = libc::O_NOCTTY   as usize;
    }
}
#[allow(missing_docs)] pub const O_CLOEXEC : OpenFlags = OpenFlags::O_CLOEXEC;
#[allow(missing_docs)] pub const O_EXCL    : OpenFlags = OpenFlags::O_EXCL;
#[allow(missing_docs)] pub const O_NONBLOCK: OpenFlags = OpenFlags::O_NONBLOCK;
#[allow(missing_docs)] pub const O_NOCTTY  : OpenFlags = OpenFlags::O_NOCTTY;

bitflags! {
    /// Flags modifying behavior of file operations
    pub struct AtFlags: usize {
        /// Follow symbolic links
        const Follow = libc::AT_SYMLINK_FOLLOW as usize;
    }
}

/// File information
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Stat {
    /** ID of device containing file  */ pub dev:     libc::dev_t,
    /** Inode number                  */ pub ino:     libc::ino_t,
    /** File type and mode            */ pub mode:    FileMode,
    /** Number of links               */ pub nlink:   libc::nlink_t,
    /** User ID of owner              */ pub uid:     libc::uid_t,
    /** Group ID of owner             */ pub gid:     libc::gid_t,
    /** Total size, in bytes          */ pub size:    libc::off_t,
    /** Block size for filesystem I/O */ pub blksize: libc::blksize_t,
    /** Number of allocated blocks    */ pub blocks:  libc::blkcnt_t,
    /** Time of last access           */ pub atime:   EpochTime,
    /** Time of last modification     */ pub mtime:   EpochTime,
    /** Time of last status change    */ pub ctime:   EpochTime,
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
