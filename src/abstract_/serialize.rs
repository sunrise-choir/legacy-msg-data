use super::Serializer;

// TODO impl this for std types

/// A **data structure** that can be serialized into any ssb legacy message
/// data format.
pub trait Serialize {
    /// Serialize this value into the given serializer.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer;
}
