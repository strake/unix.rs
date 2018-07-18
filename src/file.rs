use core::ops::*;
use rand::*;

use err::*;
use str::*;

pub use unix::file::{AtFlags, Clobber, File, FileMode, FileModeSection, FilePermission, OpenFlags, OpenMode, Stat,
                     atomic_write_file_at, open_at, rename_at, link_at, unlink_at, chmod_at, stat_at, exec_at, new_pipe};

use self::OpenMode::*;

#[deprecated(since = "0.3.0", note = "use `new_pipe`")]
#[inline]
pub fn mk_pipe(flags: OpenFlags) -> Result<(File, File), OsErr> { new_pipe(flags) }

#[inline]
pub fn mktemp_at<R: Clone, Rng: RandomGen>
  (opt_dir: Option<&File>, templ: &mut OsStr, range: R, rng: &mut Rng, flags: OpenFlags) ->
  Result<File, OsErr> where [u8]: IndexMut<R, Output = [u8]> {
    let tries = 0x100;
    for _ in 0..tries {
        randname(rng, &mut templ[range.clone()]);
        match open_at(opt_dir, templ, RdWr, flags | O_CREAT | O_EXCL, Perm::Read << USR) {
            Err(EEXIST) => (),
            r_f => return r_f,
        }
    }
    Err(EEXIST)
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

pub use self::Clobber::*;

pub(crate) use self::FilePermission as Perm;

pub use self::FileModeSection::*;

pub const O_CLOEXEC : OpenFlags = OpenFlags::O_CLOEXEC;
pub const O_CREAT   : OpenFlags = OpenFlags::O_CREAT;
pub const O_EXCL    : OpenFlags = OpenFlags::O_EXCL;
pub const O_NONBLOCK: OpenFlags = OpenFlags::O_NONBLOCK;
pub const O_NOCTTY  : OpenFlags = OpenFlags::O_NOCTTY;
