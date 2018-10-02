use super::{LegacyF64, Serialize};

/// This trait defines the serialization half of the ssb legacy message data model. It is analogous
/// to [serde::Serializer](https://docs.serde.rs/serde/trait.Serializer.html). Some doc comments
/// have been blatantly stolen from the serde docs.
pub trait Serializer: Sized {
    /// The output type produced by this `Serializer` during successful
    /// serialization. Most serializers that produce text or binary output
    /// should set `Ok = ()` and serialize into an [`io::Write`] or buffer
    /// contained within the `Serializer` instance. Serializers that build
    /// in-memory data structures may be simplified by using `Ok` to propagate
    /// the data structure around.
    ///
    /// [`io::Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
    type Ok;

    /// Type returned from [`serialize_array`] for serializing into an array.
    ///
    /// [`serialize_array`]: #tymethod.serialize_array
    type SerializeArray: SerializeArray<Ok = Self::Ok>;

    /// Type returned from [`serialize_object`] for serializing into an object.
    ///
    /// [`serialize_object`]: #tymethod.serialize_object
    type SerializeObject: SerializeObject<Ok = Self::Ok>;

    /// Serialize a `bool` value.
    fn serialize_bool(self, v: bool) -> Self::Ok;

    /// Serialize a `LegacyF64` value.
    fn serialize_f64(self, v: LegacyF64) -> Self::Ok;

    /// Serialize a `&str`.
    fn serialize_str(self, v: &str) -> Self::Ok;

    /// Serialize to `null`.
    fn serialize_null(self) -> Self::Ok;

    /// Begin to serialize to an array. This call must be followed by zero or more calls to
    /// `serialize_element`, then a call to `end`.
    ///
    /// The argument is the number of elements in the sequence. Unlike serde, ssb always requires
    /// this to be computable up front.
    fn serialize_array(self, len: usize) -> Self::SerializeArray;

    /// Begin to serialize to an object. This call must be followed by zero or more
    /// calls to `serialize_key` and `serialize_value`, then a call to `end`.
    ///
    /// The argument is the number of elements in the sequence. Unlike serde, ssb always requires
    /// this to be computable up front.
    fn serialize_object(self, len: usize) -> Self::SerializeObject;
}

/// Returned from `Serializer::serialize_array`.
pub trait SerializeArray {
    /// Must match the `Ok` type of our `Serializer`.
    type Ok;

    /// Serialize a sequence element.
    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> () where T: Serialize;

    /// Finish serializing a sequence.
    fn end(self) -> Self::Ok;
}

/// Returned from `Serializer::serialize_object`.
pub trait SerializeObject {
    /// Must match the `Ok` type of our `Serializer`.
    type Ok;

    /// Serialize a map key.
    ///
    /// If possible, `Serialize` implementations are encouraged to use
    /// `serialize_entry` instead as it may be implemented more efficiently in
    /// some formats compared to a pair of calls to `serialize_key` and
    /// `serialize_value`.
    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> () where T: Serialize;

    /// Serialize a map value.
    ///
    /// # Panics
    ///
    /// Calling `serialize_value` before `serialize_key` is incorrect and is
    /// allowed to panic or produce bogus results.
    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> () where T: Serialize;

    /// Serialize an object entry consisting of a key and a value.
    ///
    /// Some `Serialize` types are not able to hold a key and value in memory
    /// at the same time so `SerializeObject` implementations are required to
    /// support [`serialize_key`] and [`serialize_value`] individually. The
    /// `serialize_entry` method allows serializers to optimize for the case
    /// where key and value are both available. `Serialize` implementations
    /// are encouraged to use `serialize_entry` if possible.
    ///
    /// The default implementation delegates to [`serialize_key`] and
    /// [`serialize_value`]. This is appropriate for serializers that do not
    /// care about performance or are not able to optimize `serialize_entry` any
    /// better than this.
    ///
    /// [`serialize_key`]: #tymethod.serialize_key
    /// [`serialize_value`]: #tymethod.serialize_value
    fn serialize_entry<K: ?Sized, V: ?Sized>(&mut self, key: &K, value: &V) -> ()
        where K: Serialize,
              V: Serialize
    {
        self.serialize_key(key);
        self.serialize_value(value);
    }

    /// Finish serializing an object.
    fn end(self) -> Self::Ok;
}
