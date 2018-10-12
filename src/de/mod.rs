//! Generic data structure deserialization framework, corresponds to
//! [serde::de](https://docs.serde.rs/serde/de/index.html).

mod deserialize;
mod deserializer;

pub use self::deserialize::*;
pub use self::deserializer::*;
