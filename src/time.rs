use core::ops::*;
use tempus::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EpochTime(i128);

impl EpochTime {
    #[inline]
    pub fn from_ns_since_epoch(n: i128) -> Self { EpochTime(n) }

    #[inline]
    pub fn to_ns_since_epoch(self) -> i128 { self.0 }

    #[inline]
    pub(crate) fn from_c_timespec(ts: ::libc::timespec) -> Self {
        EpochTime(0) + Span::from(ts)
    }

    #[inline]
    pub(crate) fn from_s_ns(s: ::libc::time_t, ns: ::libc::c_long) -> Self {
        Self::from_c_timespec(::libc::timespec { tv_sec: s, tv_nsec: ns as _ })
    }
}

impl Add<Span> for EpochTime {
    type Output = Self;
    #[inline]
    fn add(self, other: Span) -> Self { EpochTime(self.0 + other.to_ns()) }
}

impl Sub<Span> for EpochTime {
    type Output = Self;
    #[inline]
    fn sub(self, other: Span) -> Self { EpochTime(self.0 - other.to_ns()) }
}

impl Sub for EpochTime {
    type Output = Span;
    #[inline]
    fn sub(self, other: Self) -> Span { Span::from_ns(self.0 - other.0) }
}

impl AddAssign<Span> for EpochTime {
    #[inline]
    fn add_assign(&mut self, other: Span) { self.0 += other.to_ns() }
}

impl SubAssign<Span> for EpochTime {
    #[inline]
    fn sub_assign(&mut self, other: Span) { self.0 -= other.to_ns() }
}
