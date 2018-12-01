//! Directory operations

use core::ops::IndexMut;
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
