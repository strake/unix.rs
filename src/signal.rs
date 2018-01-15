use core::mem;

use ::err::*;

pub use ::libc::id_t as Id;
pub use ::libc::c_int as Signal;

#[inline]
pub fn send(pid: Id, n: Signal) -> Result<(), OsErr> { unsafe { esyscall_!(KILL, pid, n) } }

#[derive(Clone, Copy)]
pub union Set {
    set: ::libc::sigset_t,
    raw: [u8; mem::size_of::<::libc::sigset_t>()],
}

impl Set {
    #[inline]
    pub fn mask(&self, how: How) -> Result<Self, OsErr> { unsafe {
        let mut old = mem::uninitialized();
        match ::libc::pthread_sigmask(how as _, &self.set as *const _, &mut old as *mut _) {
            0 => Ok(Set { set: old }),
            e => Err(OsErr(e as _)),
        }
    } }
}

#[repr(C)]
pub enum How {
    Block   = ::libc::SIG_BLOCK as _,
    Unblock = ::libc::SIG_UNBLOCK as _,
    SetMask = ::libc::SIG_SETMASK as _,
}

include!(concat!(env!("OUT_DIR"), "/signal.rs"));
