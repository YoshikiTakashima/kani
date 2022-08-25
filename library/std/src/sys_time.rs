// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

use crate::fmt;
use crate::time::Duration;

pub use self::inner::Instant;

const NSEC_PER_SEC: u64 = 1_000_000_000;
pub const UNIX_EPOCH: SystemTime = SystemTime { t: Timespec::zero() };

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SystemTime {
    pub(crate) t: Timespec,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Timespec {
    tv_sec: i64,
    tv_nsec: i64,
}

impl SystemTime {
    #[cfg_attr(target_os = "horizon", allow(unused))]
    pub fn _new(tv_sec: i64, tv_nsec: i64) -> SystemTime {
        SystemTime { t: Timespec::new(tv_sec, tv_nsec) }
    }

    pub fn sub_time(&self, other: &SystemTime) -> Result<Duration, Duration> {
        self.t.sub_timespec(&other.t)
    }

    #[inline]
    pub fn checked_add_duration(&self, other: &Duration) -> Option<SystemTime> {
        if let Some(t) = self.t.checked_add_duration(other) {
            Some(SystemTime { t })
        } else {
            None
        }
    }

    #[inline]
    pub fn checked_sub_duration(&self, other: &Duration) -> Option<SystemTime> {
        if let Some(t) = self.t.checked_sub_duration(other) {
            Some(SystemTime { t })
        } else {
            None
        }
    }
}

impl fmt::Debug for SystemTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SystemTime")
            .field("tv_sec", &self.t.tv_sec)
            .field("tv_nsec", &self.t.tv_nsec)
            .finish()
    }
}

impl Timespec {
    pub const fn zero() -> Timespec {
        Timespec { tv_sec: 0, tv_nsec: 0 }
    }

    #[inline]
    fn new(tv_sec: i64, tv_nsec: i64) -> Timespec {
        Timespec { tv_sec, tv_nsec }
    }

    pub fn sub_timespec(&self, other: &Timespec) -> Result<Duration, Duration> {
        if self >= other {
            // NOTE(eddyb) two aspects of this `if`-`else` are required for LLVM
            // to optimize it into a branchless form (see also #75545):
            //
            // 1. `self.tv_sec - other.tv_sec` shows up as a common expression
            //    in both branches, i.e. the `else` must have its `- 1`
            //    subtraction after the common one, not interleaved with it
            //    (it used to be `self.tv_sec - 1 - other.tv_sec`)
            //
            // 2. the `Duration::new` call (or any other additional complexity)
            //    is outside of the `if`-`else`, not duplicated in both branches
            //
            // Ideally this code could be rearranged such that it more
            // directly expresses the lower-cost behavior we want from it.
            let (secs, nsec) = if self.tv_nsec >= other.tv_nsec {
                ((self.tv_sec - other.tv_sec) as u64, (self.tv_nsec - other.tv_nsec) as u32)
            } else {
                (
                    (self.tv_sec - other.tv_sec - 1) as u64,
                    self.tv_nsec as u32 + (NSEC_PER_SEC as u32) - other.tv_nsec as u32,
                )
            };

            Ok(Duration::new(secs, nsec))
        } else {
            match other.sub_timespec(self) {
                Ok(d) => Err(d),
                Err(d) => Ok(d),
            }
        }
    }

    /// Stub of the following code.
    ///
    /// let mut secs = other
    ///     .as_secs()
    ///     .try_into() /// <- target type would be `i64`
    ///     .ok()
    ///     .and_then(|secs| self.tv_sec.checked_add(secs))?;
    #[inline]
    fn stub_checked_add(num1: u64, num2: i64) -> Option<i64> {
        if num1 < u64::MAX / 2 && num1 > u64::MIN / 2 {
            let unum1 = num1 as i64;
            num2.checked_add(unum1)
        } else {
            None
        }
    }

    #[inline]
    pub fn checked_add_duration(&self, other: &Duration) -> Option<Timespec> {
        // let mut secs = other
        //     .as_secs()
        //     .try_into() // <- target type would be `i64`
        //     .ok()
        //     .and_then(|secs| self.tv_sec.checked_add(secs))?;
        if let Some(mut secs) = Self::stub_checked_add(other.as_secs(), self.tv_sec){

            // Nano calculations can't overflow because nanos are <1B which fit
            // in a u32.
            let mut nsec = other.subsec_nanos() + self.tv_nsec as u32;
            if nsec >= NSEC_PER_SEC as u32 {
                nsec -= NSEC_PER_SEC as u32;
                if let Some(sec_final) = secs.checked_add(1) {
                    secs =  sec_final;
                } else {
                    return None;
                }
            }
            Some(Timespec::new(secs, nsec as i64))
        } else {
            None
        }
    }

    /// Stub of the following code.
    ///
    /// let mut secs = other
    ///     .as_secs()
    ///     .try_into() // <- target type would be `i64`
    ///     .ok()
    ///     .and_then(|secs| self.tv_sec.checked_sub(secs))?;
    #[inline]
    fn stub_checked_sub(num1: u64, num2: i64) -> Option<i64> {
        if num1 < u64::MAX / 2 && num1 > u64::MIN / 2 {
            let unum1 = num1 as i64;
            num2.checked_sub(unum1)
        } else {
            None
        }
    }

    #[inline]
    pub fn checked_sub_duration(&self, other: &Duration) -> Option<Timespec> {
        // let mut secs = other
        //     .as_secs()
        //     .try_into() // <- target type would be `i64`
        //     .ok()
        //     .and_then(|secs| self.tv_sec.checked_sub(secs))?;
        if let Some(mut secs) = Self::stub_checked_sub(other.as_secs(), self.tv_sec){

            // Similar to above, nanos can't overflow.
            let mut nsec = self.tv_nsec as i32 - other.subsec_nanos() as i32;
            if nsec < 0 {
                nsec += NSEC_PER_SEC as i32;
                if let Some(sec_final) = secs.checked_sub(1) {
                    secs = sec_final;
                } else {
                    return None;
                }
            }
            Some(Timespec::new(secs, nsec as i64))
        } else {
            None
        }
    }
}

mod inner {
    use crate::fmt;
    use crate::time::Duration;

    use super::Timespec;

    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Instant {
        t: Timespec,
    }

    impl Instant {
        pub fn now() -> Self {
            Self {
                t: Timespec {
                    tv_sec: kani::any(),
                    tv_nsec: kani::any(),
                },
            }
        }

        pub fn checked_sub_instant(&self, other: &Instant) -> Option<Duration> {
            self.t.sub_timespec(&other.t).ok()
        }

        pub fn checked_add_duration(&self, other: &Duration) -> Option<Instant> {
            if let Some(t) = self.t.checked_add_duration(other) {
                Some(Instant { t  })
            } else {
                None // altered for Option::branch not linked
            }
        }

        pub fn checked_sub_duration(&self, other: &Duration) -> Option<Instant> {
            if let Some(t) = self.t.checked_sub_duration(other) {
                Some(Instant { t })
            } else {
                None // altered for Option::branch not linked
            }
        }
    }

    impl fmt::Debug for Instant {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Instant")
                .field("tv_sec", &self.t.tv_sec)
                .field("tv_nsec", &self.t.tv_nsec)
                .finish()
        }
    }
}
