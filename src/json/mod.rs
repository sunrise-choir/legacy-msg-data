//! This module implements the [json encoding](TODO) of the legacy ssb
//! data format.
//!
//! Serialization methods take a boolean to switch between compact json
//! and the signing encoding.

mod de;
mod ser;

pub use self::de::{JsonDeserializer, from_slice};
pub use self::ser::{JsonSerializer, to_writer, to_vec, to_string};
