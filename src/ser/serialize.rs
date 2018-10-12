use super::Serializer;

/// A **data structure** that can be serialized into any ssb legacy message
/// data format, corresponds to
/// [serde::Serialize](https://docs.serde.rs/serde/ser/trait.Serialize.html).
pub trait Serialize {
    /// Serialize this value into the given serializer.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer;
}
