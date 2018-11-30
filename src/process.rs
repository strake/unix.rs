use core::mem;
pub use libc::id_t as Id;

use Error;

#[inline]
pub fn fork() -> Result<Id, Error> { unsafe { esyscall!(FORK).map(|pid| pid as _) } }

#[inline]
pub fn quit(code: isize) -> ! { unsafe {
    #[cfg(target_os = "linux")]
    syscall!(EXIT_GROUP, code);
    syscall!(EXIT,       code);
    ::core::intrinsics::abort()
} }

#[inline]
pub fn pid() -> Id { unsafe { syscall!(GETPID) as _ } }
#[inline]
pub fn ppid() -> Id { unsafe { syscall!(GETPPID) as _ } }

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WaitSpec { Pid(Id), Gid(Id), All }

impl WaitSpec {
    #[inline]
    pub fn wait(self, flags: WaitFlags) -> Result<(WaitInfo, ::libc::rusage), Error> {
        unsafe {
            let (id_type, id) = self.to_wait_args();
            let mut si: siginfo = mem::uninitialized();
            let mut ru: ::libc::rusage = mem::uninitialized();
            si.si.pid = 0;
            esyscall!(WAITID, id_type, id, &mut si as *mut _, flags.bits, &mut ru as *mut _)?;
            if 0 == si.si.pid { return Err(Error::EWOULDBLOCK) }
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

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WaitCode(::libc::c_int);

impl WaitCode {
    pub const Exit: Self = WaitCode(1);
    pub const Kill: Self = WaitCode(2);
    pub const Dump: Self = WaitCode(3);
    pub const Trap: Self = WaitCode(4);
    pub const Stop: Self = WaitCode(5);
    pub const Cont: Self = WaitCode(6);
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
