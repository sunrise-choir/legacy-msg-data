use std::cmp::Ordering;
use std::fmt;

/// A wrapper around `f64` to indicate that the float is compatible with the ssb legacy message
/// data model, i.e. it is neither an infinity, nor `-0.0`, nor a `NaN`.
///
/// Because a `LegacyF64` is never `NaN`, it can implement `Eq` and `Ord`, which regular `f64`
/// does not.
///
/// To obtain the inner value, use the `From<LegacyF64> for f64` impl.
#[derive(Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct LegacyF64(f64);

impl LegacyF64 {
    /// Safe conversion of an arbitrary `f64` into a `LegacyF64`.
    ///
    /// ```
    /// use ssb_legacy_msg::data::LegacyF64;
    ///
    /// assert!(LegacyF64::from_f64(0.0).is_some());
    /// assert!(LegacyF64::from_f64(-1.1).is_some());
    /// assert!(LegacyF64::from_f64(-0.0).is_none());
    /// assert!(LegacyF64::from_f64(std::f64::INFINITY).is_none());
    /// assert!(LegacyF64::from_f64(std::f64::NEG_INFINITY).is_none());
    /// assert!(LegacyF64::from_f64(std::f64::NAN).is_none());
    /// ```
    pub fn from_f64(f: f64) -> Option<LegacyF64> {
        if LegacyF64::is_valid(f) {
            Some(LegacyF64(f))
        } else {
            None
        }
    }

    /// Wraps the given `f64` as a `LegacyF64` without checking if it is valid.
    ///
    /// When the `debug_assertions` feature is enabled (when compiling without optimizations), this
    /// function panics when given an invalid `f64`.
    ///
    /// # Safety
    /// You must not pass infinity, negative infinity, negative zero or a `NaN` to this
    /// function. Any method on the resulting `LegacyF64` could panic or exhibit undefined
    /// behavior.
    ///
    /// ```
    /// use ssb_legacy_msg::data::LegacyF64;
    ///
    /// let fine = unsafe { LegacyF64::from_f64_unchecked(1.1) };
    ///
    /// // Never do this:
    /// // let everything_is_terrible = unsafe { LegacyF64::from_f64_unchecked(-0.0) };
    /// ```
    pub unsafe fn from_f64_unchecked(f: f64) -> LegacyF64 {
        debug_assert!(LegacyF64::is_valid(f));
        LegacyF64(f)
    }

    /// Checks whether a given `f64` may be used as a `LegacyF64`.
    pub fn is_valid(f: f64) -> bool {
        if f == 0.0 {
            f.is_sign_positive()
        } else {
            f.is_finite() && (f != 0.0)
        }
    }
}

impl fmt::Display for LegacyF64 {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}

impl fmt::Debug for LegacyF64 {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}

impl Eq for LegacyF64 {}

impl Ord for LegacyF64 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl From<LegacyF64> for f64 {
    fn from(f: LegacyF64) -> Self {
        f.0
    }
}
