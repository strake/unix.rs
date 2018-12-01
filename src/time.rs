//! Temporal types and operations

use core::{mem, ops::*};
use idem::Zero;
use tempus::Span;

/// Time measured since the Unix epoch
///
/// The Unix epoch = Julian date 2440587.5 = Gregorian date January 1st 1970 00:00 UTC
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EpochTime(i128);

impl EpochTime {
    /// Return the present time.
    #[inline]
    pub fn now() -> Self { unsafe {
        let mut t = mem::uninitialized();
        syscall!(CLOCK_GETTIME, ::libc::CLOCK_REALTIME, &mut t as *mut _);
        Self::from_c_timespec(t)
    } }

    /// Convert from nanoseconds since the Unix epoch to an `EpochTime`.
    #[inline]
    pub fn from_ns_since_epoch(n: i128) -> Self { EpochTime(n) }

    /// Convert from an `EpochTime` to nanoseconds since the Unix epoch.
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

/// Sleep for the given time span.
///
/// # Failures
///
/// Returns `Err(remaining_time)` if interrupted.
#[inline]
pub fn sleep_for(t: Span) -> Result<(), Span> { unsafe {
    assert!(Span::zero < t);
    let mut rem = mem::uninitialized();
    esyscall_!(CLOCK_NANOSLEEP, ::libc::CLOCK_MONOTONIC, 0,
               &t.to_c_timespec().expect("timespan too long") as *const _,
               &mut rem as *mut _).map_err(|_| rem)
} }

/// Sleep until the given time point.
///
/// # Failures
///
/// Returns `Err` if interrupted.
#[inline]
pub fn sleep_until(t: EpochTime) -> Result<(), ()> { unsafe {
    esyscall_!(CLOCK_NANOSLEEP, ::libc::CLOCK_REALTIME, ::libc::TIMER_ABSTIME,
               &(t - EpochTime(0)).to_c_timespec().expect("timespan too long") as *const _)
        .map_err(|_| ())
} }
