use core::mem;
pub use libc::id_t as Id;

use err::*;

#[inline]
pub fn fork() -> Result<Id, OsErr> { unsafe { esyscall!(FORK).map(|pid| pid as _) } }

#[inline]
pub fn quit(code: isize) -> ! { unsafe {
    #[cfg(target_os = "linux")]
    syscall!(EXIT_GROUP, code);
    syscall!(EXIT,       code);
    ::core::intrinsics::abort()
} }

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WaitSpec { Pid(Id), Gid(Id), All }

impl WaitSpec {
    #[inline]
    pub fn wait(self, flags: WaitFlags) -> Result<(WaitInfo, ::libc::rusage), OsErr> {
        unsafe {
            let (id_type, id) = self.to_wait_args();
            let mut si: siginfo = mem::uninitialized();
            let mut ru: ::libc::rusage = mem::uninitialized();
            si.si.pid = 0;
            esyscall!(WAITID, id_type, id, &mut si as *mut _, flags.bits, &mut ru as *mut _)?;
            if 0 == si.si.pid { return Err(EWOULDBLOCK) }
            Ok((WaitInfo::from_c(si.si), ru))
        }
    }

    #[inline]
    unsafe fn to_wait_args(self) -> (::libc::idtype_t, Id) {
        use self::WaitSpec::*;
        match self {
            Pid(pid) => (::libc::P_PID, pid),
            Gid(gid) => (::libc::P_PGID, gid),
            All      => (::libc::P_ALL, mem::uninitialized()),
        }
    }
}

bitflags! {
    pub struct WaitFlags: usize {
        const Exit = ::libc::WEXITED as usize;
        const Stop = ::libc::WSTOPPED as usize;
        const Cont = ::libc::WCONTINUED as usize;
        const NoHang = ::libc::WNOHANG as usize;
        const NoWait = ::libc::WNOWAIT as usize;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WaitInfo {
    pid: Id,
    code: WaitCode,
    status: isize,
}

impl WaitInfo {
    #[inline]
    unsafe fn from_c(si: siginfo_) -> Self {
        WaitInfo {
            pid: si.pid as _,
            code: WaitCode::from_c(si.code),
            status: si.status as _,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WaitCode { Exit = 1, Kill = 2, Dump = 3, Trap = 4, Stop = 5, Cont = 6 }

impl WaitCode {
    #[inline]
    unsafe fn from_c(c: ::libc::c_int) -> Self {
        use self::WaitCode::*;
        match c {
            1 => Exit,
            2 => Kill,
            3 => Dump,
            4 => Trap,
            5 => Stop,
            6 => Cont,
            _ => mem::uninitialized()
        }
    }
}

#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Clone, Copy)]
struct siginfo_ {
    signo:  ::libc::c_int,
    errno:  ::libc::c_int,
    code:   ::libc::c_int,
    pid:    ::libc::pid_t,
    uid:    ::libc::uid_t,
    value:  ::libc::sigval,
    status: ::libc::c_int,
}

#[repr(C)]
union siginfo {
    si: siginfo_,
    pad: [u8; 0x80],
}
