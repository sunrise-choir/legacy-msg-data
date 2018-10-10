use std::marker::PhantomData;

use super::{
    Deserialize,
    DeserializeSeed,
    super::LegacyF64
};

/// This trait defines the deserialization half of the ssb legacy message data model. Corresponds
/// to [serde::Deserializer](https://docs.serde.rs/serde/trait.Deserializer.html).
///
/// Unlike serde, relying on `Deserializer::deserialize_any` is completely fine, all ssb
/// legacy data formats must be self-describing.
pub trait Deserializer<'de>: Sized {
    /// The error type that can be returned if some error occurs during
    /// deserialization.
    type Error;

    /// Require the `Deserializer` to figure out how to drive the visitor based
    /// on what data type is in the input.
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting a `bool` value.
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting a `LegacyF64` value.
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting a string value and does
    /// not benefit from taking ownership of buffered data owned by the
    /// `Deserializer`.
    ///
    /// If the `Visitor` would benefit from taking ownership of `String` data,
    /// indiciate this to the `Deserializer` by using `deserialize_string`
    /// instead.
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting a string value and would
    /// benefit from taking ownership of buffered data owned by the
    /// `Deserializer`.
    ///
    /// If the `Visitor` would not benefit from taking ownership of `String`
    /// data, indicate that to the `Deserializer` by using `deserialize_str`
    /// instead.
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting a `null` value.
    fn deserialize_null<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting an array.
    fn deserialize_array<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting an object.
    fn deserialize_object<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>;
}

////////////////////////////////////////////////////////////////////////////////

/// This trait represents a visitor that walks through a deserializer.
///
/// Unlike the [serde equivalent](https://docs.serde.rs/serde/de/trait.Visitor.html), there is no
/// opinionated error handling, the methods thus don't have default implementations returning errors.
pub trait Visitor<'de>: Sized {
    /// The value produced by this visitor.
    type Value;

    /// The input contains a boolean.
    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>;

    /// The input contains a `LegacyF64`.
    fn visit_f64<E>(self, v: LegacyF64) -> Result<Self::Value, E>;

    /// The input contains a string. The lifetime of the string is ephemeral and
    /// it may be destroyed after this method returns.
    ///
    /// This method allows the `Deserializer` to avoid a copy by retaining
    /// ownership of any buffered data. `Deserialize` implementations that do
    /// not benefit from taking ownership of `String` data should indicate that
    /// to the deserializer by using `Deserializer::deserialize_str` rather than
    /// `Deserializer::deserialize_string`.
    ///
    /// It is never correct to implement `visit_string` without implementing
    /// `visit_str`. Implement neither, both, or just `visit_str`.
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>;

    /// The input contains a string that lives at least as long as the
    /// `Deserializer`.
    ///
    /// This enables zero-copy deserialization of strings in some formats. For
    /// example JSON input containing the JSON string `"borrowed"` can be
    /// deserialized with zero copying into a `&'a str` as long as the input
    /// data outlives `'a`.
    ///
    /// The default implementation forwards to `visit_str`.
    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E> {
        self.visit_str(v)
    }

    /// The input contains a string and ownership of the string is being given
    /// to the `Visitor`.
    ///
    /// This method allows the `Visitor` to avoid a copy by taking ownership of
    /// a string created by the `Deserializer`. `Deserialize` implementations
    /// that benefit from taking ownership of `String` data should indicate that
    /// to the deserializer by using `Deserializer::deserialize_string` rather
    /// than `Deserializer::deserialize_str`, although not every deserializer
    /// will honor such a request.
    ///
    /// It is never correct to implement `visit_string` without implementing
    /// `visit_str`. Implement neither, both, or just `visit_str`.
    ///
    /// The default implementation forwards to `visit_str` and then drops the
    /// `String`.
    fn visit_string<E>(self, v: String) -> Result<Self::Value, E> {
        self.visit_str(&v)
    }

    /// The input contains a `null`.
    fn visit_null<E>(self) -> Result<Self::Value, E>;

    /// The input contains an array.
    fn visit_array<A>(self, seq: A) -> Result<Self::Value, A::Error> where A: ArrayAccess<'de>;

    /// The input contains an object.
    fn visit_object<A>(self, object: A) -> Result<Self::Value, A::Error> where A: ObjectAccess<'de>;
}

////////////////////////////////////////////////////////////////////////////////

/// Provides a `Visitor` access to each element of an array in the input.
/// Corresponds to [serde::de::SeqAccess](https://docs.serde.rs/serde/de/trait.SeqAccess.html).
///
/// This is a trait that a `Deserializer` passes to a `Visitor` implementation,
/// which deserializes each item in an array.
pub trait ArrayAccess<'de> {
    /// The error type that can be returned if some error occurs during
    /// deserialization.
    type Error;

    /// This returns `Ok(Some(value))` for the next value in the sequence, or
    /// `Ok(None)` if there are no more remaining items.
    ///
    /// `Deserialize` implementations should typically use
    /// `ArrayAccess::next_element` instead.
    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
        where T: DeserializeSeed<'de>;

    /// This returns `Ok(Some(value))` for the next value in the sequence, or
    /// `Ok(None)` if there are no more remaining items.
    ///
    /// This method exists as a convenience for `Deserialize` implementations.
    /// `ArrayAccess` implementations should not override the default behavior.
    #[inline]
    fn next_element<T>(&mut self) -> Result<Option<T>, Self::Error>
        where T: Deserialize<'de>
    {
        self.next_element_seed(PhantomData)
    }

    /// Returns the number of elements remaining in the array, if known.
    fn size_hint(&self) -> Option<usize> {
        None
    }
}

impl<'de, 'a, A> ArrayAccess<'de> for &'a mut A
    where A: ArrayAccess<'de>
{
    type Error = A::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
        where T: DeserializeSeed<'de>
    {
        (**self).next_element_seed(seed)
    }

    fn next_element<T>(&mut self) -> Result<Option<T>, Self::Error>
        where T: Deserialize<'de>
    {
        (**self).next_element()
    }

    #[inline]
    fn size_hint(&self) -> Option<usize> {
        (**self).size_hint()
    }
}

////////////////////////////////////////////////////////////////////////////////

/// Trait for the state needed to deserialize objects.
///
/// Corresponds to [serde::de::MapAccess](https://docs.serde.rs/serde/de/trait.MapAccess.html).
pub trait ObjectAccessState {
    /// Check whether this key has already been encountered.
    fn has_key(self, key: &str) -> bool;
}

/// This implementation always returns true, which makes stateless decoders unusable. That's by
/// design, a valid ssb legacy format decoder must use state to check that no keys are used
/// multiple times.
impl<T> ObjectAccessState for PhantomData<T> {
    fn has_key(self, _key: &str) -> bool {
        true
    }
}

/// Provides a `Visitor` access to each entry of an object in the input.
///
/// This is a trait that a `Deserializer` passes to a `Visitor` implementation.
pub trait ObjectAccess<'de> {
    /// The error type that can be returned if some error occurs during
    /// deserialization.
    type Error;

    /// This returns `Ok(Some(key))` for the next key in the map, or `Ok(None)`
    /// if there are no more remaining entries.
    ///
    /// `Deserialize` implementations should typically use
    /// `ObjectAccess::next_key` or `ObjectAccess::next_entry` instead.
    fn next_key_seed<K: ObjectAccessState>(&mut self,
                                           seed: K)
                                           -> Result<Option<String>, Self::Error>;

    /// This returns a `Ok(value)` for the next value in the map.
    ///
    /// `Deserialize` implementations should typically use
    /// `ObjectAccess::next_value` instead.
    ///
    /// # Panics
    ///
    /// Calling `next_value_seed` before `next_key_seed` is incorrect and is
    /// allowed to panic or return bogus results.
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
        where V: DeserializeSeed<'de>;

    /// This returns `Ok(Some((key, value)))` for the next (key-value) pair in
    /// the map, or `Ok(None)` if there are no more remaining items.
    ///
    /// `ObjectAccess` implementations should override the default behavior if a
    /// more efficient implementation is possible.
    ///
    /// `Deserialize` implementations should typically use
    /// `ObjectAccess::next_entry` instead.
    #[inline]
    fn next_entry_seed<K, V>(&mut self,
                             kseed: K,
                             vseed: V)
                             -> Result<Option<(String, V::Value)>, Self::Error>
        where K: ObjectAccessState,
              V: DeserializeSeed<'de>
    {
        match try!(self.next_key_seed(kseed)) {
            Some(key) => {
                let value = try!(self.next_value_seed(vseed));
                Ok(Some((key, value)))
            }
            None => Ok(None),
        }
    }

    /// This returns `Ok(Some(key))` for the next key in the map, or `Ok(None)`
    /// if there are no more remaining entries.
    ///
    /// This method exists as a convenience for `Deserialize` implementations.
    /// `ObjectAccess` implementations should not override the default behavior.
    #[inline]
    fn next_key<K: ObjectAccessState>(&mut self) -> Result<Option<String>, Self::Error> {
        self.next_key_seed::<PhantomData<()>>(PhantomData)
    }

    /// This returns a `Ok(value)` for the next value in the map.
    ///
    /// This method exists as a convenience for `Deserialize` implementations.
    /// `ObjectAccess` implementations should not override the default behavior.
    ///
    /// # Panics
    ///
    /// Calling `next_value` before `next_key` is incorrect and is allowed to
    /// panic or return bogus results.
    #[inline]
    fn next_value<V>(&mut self) -> Result<V, Self::Error>
        where V: Deserialize<'de>
    {
        self.next_value_seed(PhantomData)
    }

    /// This returns `Ok(Some((key, value)))` for the next (key-value) pair in
    /// the map, or `Ok(None)` if there are no more remaining items.
    ///
    /// This method exists as a convenience for `Deserialize` implementations.
    /// `ObjectAccess` implementations should not override the default behavior.
    #[inline]
    fn next_entry<K, V>(&mut self) -> Result<Option<(String, V)>, Self::Error>
        where K: ObjectAccessState,
              V: Deserialize<'de>
    {
        self.next_entry_seed::<PhantomData<()>, _>(PhantomData, PhantomData)
    }

    /// Returns the number of elements remaining in the object, if known.
    fn size_hint(&self) -> Option<usize> {
        None
    }
}

impl<'de, 'a, A> ObjectAccess<'de> for &'a mut A
    where A: ObjectAccess<'de>
{
    type Error = A::Error;

    #[inline]
    fn next_key_seed<K: ObjectAccessState>(&mut self,
                                           seed: K)
                                           -> Result<Option<String>, Self::Error> {
        (**self).next_key_seed(seed)
    }

    #[inline]
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
        where V: DeserializeSeed<'de>
    {
        (**self).next_value_seed(seed)
    }

    #[inline]
    fn next_entry_seed<K, V>(&mut self,
                             kseed: K,
                             vseed: V)
                             -> Result<Option<(String, V::Value)>, Self::Error>
        where K: ObjectAccessState,
              V: DeserializeSeed<'de>
    {
        (**self).next_entry_seed(kseed, vseed)
    }

    #[inline]
    fn next_entry<K: ObjectAccessState, V>(&mut self) -> Result<Option<(String, V)>, Self::Error>
        where V: Deserialize<'de>
    {
        (**self).next_entry::<PhantomData<()>, _>()
    }

    #[inline]
    fn next_key<K: ObjectAccessState>(&mut self) -> Result<Option<String>, Self::Error> {
        (**self).next_key::<PhantomData<()>>()
    }

    #[inline]
    fn next_value<V>(&mut self) -> Result<V, Self::Error>
        where V: Deserialize<'de>
    {
        (**self).next_value()
    }

    #[inline]
    fn size_hint(&self) -> Option<usize> {
        (**self).size_hint()
    }
}
