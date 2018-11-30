use Error;
use file::*;
use tempus::Span;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Poll {
    pub fd: ::libc::c_int,
    pub ev: Event,
    r: Event,
}

impl Poll {
    #[inline]
    pub fn new(f: &File, ev: Event) -> Self { Poll {
        fd: f.fd() as _, ev, r: Event::empty(),
    } }

    #[inline]
    pub fn ready(self) -> Event { self.r }
}

bitflags! {
    pub struct Event: ::libc::c_short {
        const In  = ::libc::POLLIN;
        const Out = ::libc::POLLOUT;
        const Pri = ::libc::POLLPRI;
        const Hup = ::libc::POLLHUP;
        const Err = ::libc::POLLERR;
    }
}

pub trait PollExt {
    fn poll(&mut self, t: Option<Span>) -> Result<usize, Error>;
}

impl PollExt for [Poll] {
    #[inline]
    fn poll(&mut self, t: Option<Span>) -> Result<usize, Error> {
        let t = match t {
            None => None,
            Some(t) => Some(t.to_c_timespec().ok_or(Error::ERANGE)?),
        };
        unsafe { esyscall!(POLL, self.as_ptr(), self.len(), t.as_ref().map_or(::core::ptr::null(), |p| p as *const _), 0) }
    }
}
