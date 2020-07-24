//! Directory operations

use core::{convert::TryFrom, fmt, mem, ops::IndexMut, slice};
use rand::Rng;

use {Error, File, Str};
use file::{FileMode, Perm};

/// Create a directory at the given `path`.
pub fn mkdir_at(opt_dir: Option<&File>, path: &Str, mode: FileMode) -> Result<(), Error> {
    unsafe { esyscall_!(MKDIRAT, ::file::from_opt_dir(opt_dir), path.as_ptr(), mode.bits()) }
}

/// Generate a unique temporary file name in `templ`, and create a directory there.
///
/// The given `range` of `templ` will be replaced with a string which uniquifies the file name. The contents of `range` after the call are unspecified.
/// The directory is created with mode 700.
pub fn mktemp_at<R: Clone, TheRng: Rng>
  (opt_dir: Option<&File>, templ: &mut Str, range: R, rng: &mut TheRng) -> Result<(), Error>
  where [u8]: IndexMut<R, Output = [u8]> {
    ::file::mktemp_helper(|path| mkdir_at(opt_dir, path, (Perm::Read | Perm::Write | Perm::Exec) << ::file::FileModeSection::USR),
                          templ, range, rng)
}

/// Directory entry iterator
pub struct Entries {
    file: File,
    buf: [u8; 0xFF8],
    end: usize,
    k: usize,
}

impl fmt::Debug for Entries {
    fn fmt(&self, _: &mut fmt::Formatter) -> fmt::Result { Ok(()) }
}

impl TryFrom<File> for Entries {
    type Error = Error;

    #[inline]
    fn try_from(file: File) -> Result<Self, Error> {
        Ok(Self {
            file,
            buf: unsafe { mem::MaybeUninit::uninit().assume_init() },
            end: 0,
            k: 0,
        })
    }
}

impl Entries {
    /// Next entry
    pub fn next(&mut self) -> Result<Option<Entry>, Error> {
        if self.k >= self.end {
            let n = unsafe { esyscall!(GETDENTS, self.file.fd(), self.buf.as_mut_ptr(), self.buf.len())? };
            if 0 == n { return Ok(None) }
            self.end = n;
            self.k = 0;
        }

        #[repr(C)]
        #[derive(Clone, Copy)]
        struct Hdr {
            ino: usize,
            off: usize,
            len: u16,
        }

        Ok(Some(unsafe {
            let k = self.k;
            let hdr = *(self.buf.as_ptr().wrapping_add(k) as *const Hdr);
            self.k += hdr.off;
            let typ = self.buf[k + hdr.len as usize - 1];
            Entry { ino: hdr.ino, typ,
                    name: &slice::from_raw_parts(self.buf.as_ptr().wrapping_add(k),
                                                 hdr.len as usize - 2)[mem::size_of::<Hdr>()..] }
        }))
    }
}

/// Directory entry
#[repr(C)]
#[derive(Debug)]
pub struct Entry<'a> {
    /// Inode number
    pub ino: usize,
    /// File type
    pub typ: u8,
    /// File name
    pub name: &'a [u8],
}
