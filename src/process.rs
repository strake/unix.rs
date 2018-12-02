//! Process operations

use core::mem;
pub use libc::id_t as Id;

use Error;

/// Create a new process which is a copy of the calling process.
#[inline]
pub fn fork() -> Result<Id, Error> { unsafe { esyscall!(FORK).map(|pid| pid as _) } }

/// Terminate the calling process.
#[inline]
pub fn quit(code: isize) -> ! { unsafe {
    #[cfg(target_os = "linux")]
    syscall!(EXIT_GROUP, code);
    syscall!(EXIT,       code);
    ::core::intrinsics::abort()
} }

/// Return the ID of the calling process.
#[inline]
pub fn pid() -> Id { unsafe { syscall!(GETPID) as _ } }

/// Return the ID of the parent of the calling process.
#[inline]
pub fn ppid() -> Id { unsafe { syscall!(GETPPID) as _ } }

/// Specify which child to wait for
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WaitSpec {
    /** Wait for the given process              */ Pid(Id),
    /** Wait for any process in the given group */ Gid(Id),
    /** Wait for any process                    */ All,
}

impl WaitSpec {
    /// Wait for the state of a child process to change, and return information about it.
    #[inline]
    pub fn wait(self, flags: WaitFlags) -> Result<(WaitInfo, ::libc::rusage), Error> {
        unsafe {
            let (id_type, id) = self.to_wait_args();
            let mut si = siginfo_ { u: () };
            let mut ru: ::libc::rusage = mem::uninitialized();
            si.si.si_pid = 0;
            #[cfg(target_os = "linux")]
            esyscall!(WAITID, id_type, id, &mut si as *mut _, flags.bits(), &mut ru as *mut _)?;
            #[cfg(target_os = "freebsd")]
            esyscall!(WAIT6, id_type, id, &mut 0usize as *mut _, flags.bits(), &mut ru as *mut _, &mut si as *mut _)?;
            if 0 == si.si.si_pid { return Err(Error::EWOULDBLOCK) }
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

#[allow(missing_docs)]
mod wait_flags { bitflags! {
    pub struct WaitFlags: usize {
        const Exit = ::libc::WEXITED as usize;
        const Stop = ::libc::WSTOPPED as usize;
        const Cont = ::libc::WCONTINUED as usize;

        /// Return immediately if no child already changed state.
        const NoHang = ::libc::WNOHANG as usize;
        /// Return if a child is stopped, even if the child is not traced.
        const NoWait = ::libc::WNOWAIT as usize;
    }
} } pub use self::wait_flags::WaitFlags;

/// Notice of process termination
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WaitInfo {
    /** ID of terminated process */     pid: Id,
    /** Cause of process termination */ code: WaitCode,
    status: isize,
}

impl WaitInfo {
    #[inline]
    unsafe fn from_c(si: siginfo) -> Self {
        WaitInfo {
            pid: si.si_pid as _,
            code: WaitCode(si.si_code),
            status: si.si_status as _,
        }
    }
}

/// Cause of process termination
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WaitCode(::libc::c_int);

#[allow(missing_docs)]
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
struct siginfo {
    si_signo:  ::libc::c_int,
    si_errno:  ::libc::c_int,
    si_code:   ::libc::c_int,
    si_pid:    ::libc::pid_t,
    si_uid:    ::libc::uid_t,
    si_value:  ::libc::sigval,
    si_status: ::libc::c_int,
}

#[cfg(target_os = "freebsd")]
type siginfo = ::libc::siginfo_t;

#[repr(C)]
union siginfo_ {
    si: siginfo,
    pad: [u8; 0x80],
    u: (),
}
