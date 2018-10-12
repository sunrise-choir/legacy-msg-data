//! Generic data structure serialization framework, corresponds to
//! [serde::ser](https://docs.serde.rs/serde/ser/index.html).

mod serialize;
mod serializer;

pub use self::serialize::*;
pub use self::serializer::*;
