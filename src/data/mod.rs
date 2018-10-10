//! This module implements the ssb [legacy data format](TODO), i.e. the
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

mod legacy_f64;
mod value;

pub use self::value::*;
pub use self::legacy_f64::*;

pub mod de;
pub mod ser;

pub use self::de::Deserialize;
pub use self::de::Deserializer;
pub use self::ser::Serialize;
pub use self::ser::Serializer;

pub mod json;
pub mod cbor;
