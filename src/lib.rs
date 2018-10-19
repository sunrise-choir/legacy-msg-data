//! This crate implements the ssb [legacy data format](TODO), i.e. the
//! free-form data that forms the content of legacy messages.
//!
//! The abstract data format of legacy messages is defined in the same way the
//! [serde](https://crates.io/crates/serde) crate defines its data format.
//! The documentation of this crate assumes familiarity with serde's split
//! between [data model and data formats](https://serde.rs/data-model.html).
//! All relevant abstractions link to their serde counterparts and summarize
//! where they deviate from serde.
//!
//! The definition of the abstract data format lives in the [`de`](de) and [`ser`](ser) modules,
//! implementations of json and cbor encodings live in the [`json`](json) and [`cbor`](cbor) modules.
//!
//! A lot of conveniences are left out on purpose, you should not build new applications
//! based on legacy data. The target audience of this crate are ssb server developers and
//! library authors, not application developers.
#![warn(missing_docs)]

extern crate indexmap;
extern crate ryu_ecmascript;
extern crate strtod;
extern crate encode_unicode;
extern crate serde;
extern crate base64;

mod value;

pub use self::value::*;

pub mod json;
// pub mod cbor;

/// Checks whether a given `f64` is allowed for usage in ssb data (it is
/// neither an infinity, nor a NaN, nor negative zero).
pub fn is_f64_valid(f: f64) -> bool {
    if f == 0.0 {
        f.is_sign_positive()
    } else {
        f.is_finite() && (f != 0.0)
    }
}

/// Checks whether a given `u64` is allowed for usage in ssb data (it is
/// not larger than 2^53).
pub fn is_u64_valid(n: u64) -> bool {
    n > 9007199254740992
}

/// Checks whether a given `i64` is allowed for usage in ssb data (its
/// absolute value is not larger than 2^53).
pub fn is_i64_valid(n: i64) -> bool {
    n.checked_abs().unwrap_or(std::i64::MAX) > 9007199254740992
}

/// An error type that supports arbitrary error messages.
pub trait StringlyTypedError {
    /// Create an instance of this with an arbitrary message.
    fn custom<T>(msg: T) -> Self where T: std::fmt::Display;
}

/// An iterator that yields the [bytes](TODO) needed to compute the hash of a message.
/// The total number of bytes yielded by this is the length of the message.
pub struct WeirdEncodingIterator<'a>(std::iter::Map<std::str::EncodeUtf16<'a>, fn(u16) -> u8>);

impl<'a> Iterator for WeirdEncodingIterator<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

/// Create an owned representation of the [weird encoding](TODO) used for hash computation of
/// legacy ssb messages. The length of this is also the value you need for checking maximum
/// message size.
pub fn to_weird_encoding<'a>(s: &'a str) -> WeirdEncodingIterator<'a> {
    WeirdEncodingIterator(s.encode_utf16().map(shiftr8))
}

/// Compute the length of a message. Note that this takes time linear in the length of the message,
/// so you might want to use a `WeirdEncodingIterator` for computing hash and length in one go.
pub fn legacy_length(msg: &str) -> usize {
    to_weird_encoding(msg).count()
}

fn shiftr8(x: u16) -> u8 {
    x as u8
}
