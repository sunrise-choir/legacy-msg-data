//! This mod defines the data model used by ssb legacy messages. It follows the same approach as
//! [serde](https://serde.rs/), separating the abstract model from specific encodings via traits.
//! This code assumes familiarity with how serde works.
//!
//! To capture precisely the notion of ssb values, we chose to not reuse serde traits directly, but
//! define our own. There are also some minor differences to serde, e.g. ssb collections must always
//! know their complete size when starting serialization, as some serialization formats rely on that
//! knowledge.

use std::cmp::Ordering;

pub mod serializer;
pub mod deserializer;
pub mod serialize;
pub mod deserialize;

/// A wrapper around `f64` to indicate that the float is compatible with the ssb legacy message
/// data model, i.e. it is neither an infinity, nor `-0.0`, nor a `NaN`. Putting one of these into
/// a `LegacyF64` may result in undefined behavior.
///
/// Because of these constrainst, it can implement `Eq` and `Ord`, which regular `f64` does not.
///
/// Use the `From` or `Into` implementation to access the wrapped value.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct LegacyF64(f64);

impl LegacyF64 {
    /// Safe conversion of an arbitrary `f64` into a `LegacyF64`.
    pub fn from_f64(f: f64) -> Option<LegacyF64> {
        if LegacyF64::is_valid(f) {
            Some(LegacyF64(f))
        } else {
            None
        }
    }

    /// Wraps the given `f64` as a `LegacyF64` without checking if it is valid.
    ///
    /// Using this with an invalid `f64` may result in undefined behavior.
    ///
    /// When the `debug_assertions` feature is enabled (when compiling without optimizations), this
    /// function panics when given an invalid `f64`.
    pub unsafe fn from_f64_unsafe(f: f64) -> LegacyF64 {
        debug_assert!(LegacyF64::is_valid(f));
        LegacyF64(f)
    }

    /// Checks whether a given `f64` may be used as a `LegacyF64`.
    pub fn is_valid(f: f64) -> bool {
        f.is_finite() && (f != -0.0)
    }
}

impl From<LegacyF64> for f64 {
    fn from(f: LegacyF64) -> Self {
        f.0
    }
}

impl Eq for LegacyF64 {}

impl Ord for LegacyF64 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}
