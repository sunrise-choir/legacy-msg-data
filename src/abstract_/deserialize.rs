use std::marker::PhantomData;

use super::Deserializer;

// TODO impl these for std types

/// A **data structure** that can be deserialized from any ssb legacy message
/// data format.
pub trait Deserialize<'de>: Sized {
    /// Deserialize this value from the given deserializer.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de>;

    /// Deserializes a value into `self` from the given Deserializer.
    ///
    /// The purpose of this method is to allow the deserializer to reuse
    /// resources and avoid copies. As such, if this method returns an error,
    /// `self` will be in an indeterminate state where some parts of the struct
    /// have been overwritten. Although whatever state that is will be
    /// memory-safe.
    ///
    /// This is generally useful when repeatedly deserializing values that
    /// are processed one at a time, where the value of `self` doesn't matter
    /// when the next deserialization occurs.
    ///
    /// If you manually implement this, your recursive deserializations should
    /// use `deserialize_in_place`.
    fn deserialize_in_place<D>(deserializer: D, place: &mut Self) -> Result<(), D::Error>
        where D: Deserializer<'de>
    {
        // Default implementation just delegates to `deserialize` impl.
        *place = Deserialize::deserialize(deserializer)?;
        Ok(())
    }
}

/// A data structure that can be deserialized without borrowing any data from
/// the deserializer.
///
/// This is primarily useful for trait bounds on functions. For example a
/// `from_str` function may be able to deserialize a data structure that borrows
/// from the input string, but a `from_reader` function may only deserialize
/// owned data.
pub trait DeserializeOwned: for<'de> Deserialize<'de> {}
impl<T> DeserializeOwned for T where T: for<'de> Deserialize<'de> {}

/// `DeserializeSeed` is the stateful form of the `Deserialize` trait. If you
/// ever find yourself looking for a way to pass data into a `Deserialize` impl,
/// this trait is the way to do it.
pub trait DeserializeSeed<'de>: Sized {
    /// The type produced by using this seed.
    type Value;

    /// Equivalent to the more common `Deserialize::deserialize` method, except
    /// with some initial piece of data (the seed) passed in.
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where D: Deserializer<'de>;
}

impl<'de, T> DeserializeSeed<'de> for PhantomData<T>
    where T: Deserialize<'de>
{
    type Value = T;

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<T, D::Error>
        where D: Deserializer<'de>
    {
        T::deserialize(deserializer)
    }
}
