//! I/O multiplexing

use Error;
use file::*;
use tempus::Span;

/// Specify what to poll
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Poll {
    /** Which file descriptor to poll */ pub fd: ::libc::c_int,
    /** Which events to poll          */ pub ev: Event,
    r: Event,
}

impl Poll {
    #[allow(missing_docs)]
    #[inline]
    pub fn new(f: &File, ev: Event) -> Self { Poll {
        fd: f.fd() as _, ev, r: Event::empty(),
    } }

    /// Return which events are ready on the file descriptor.
    #[inline]
    pub fn ready(self) -> Event { self.r }
}

bitflags! {
    /// Which events to poll
    pub struct Event: ::libc::c_short {
        /** Readable       */ const In  = ::libc::POLLIN;
        /** Writable       */ const Out = ::libc::POLLOUT;
        /** Exceptional    */ const Pri = ::libc::POLLPRI;
        /** Closed/hung-up */ const Hup = ::libc::POLLHUP;
        /** Erroneous      */ const Err = ::libc::POLLERR;
    }
}

#[allow(missing_docs)]
pub trait PollExt {
    fn poll(&mut self, t: Option<Span>) -> Result<usize, Error>;
}

impl PollExt for [Poll] {
    /// Poll the given file descriptors for the given events.
    #[inline]
    fn poll(&mut self, t: Option<Span>) -> Result<usize, Error> {
        let t = match t {
            None => None,
            Some(t) => Some(t.to_c_timespec().ok_or(Error::ERANGE)?),
        };
        unsafe { esyscall!(POLL, self.as_ptr(), self.len(), t.as_ref().map_or(::core::ptr::null(), |p| p as *const _), 0) }
    }
}
