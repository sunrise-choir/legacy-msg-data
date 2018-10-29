use std::{error, fmt};

use serde::de::{self, Deserializer, Deserialize, DeserializeOwned, DeserializeSeed, Visitor,
                SeqAccess, MapAccess, EnumAccess, VariantAccess, IntoDeserializer};

use super::super::LegacyF64;

/// Everything that can go wrong during deserialization.
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum DecodeCborError {
    /// Needed more data but got EOF instead.
    UnexpectedEndOfInput,
    /// Encountered a major or additional type that is disallowed.
    ForbiddenType,
    /// A number is -0, an infinity or NaN
    InvalidNumber,
    /// The content of a string is not utf8
    InvalidStringContent,
    /// A string or collection claims a length longer than the remaining input
    InvalidLength,
    /// The input contained valid cbor followed by at least one more byte.
    TrailingBytes,
    /// Attempted to parse a number as an `i8` that was out of bounds.
    OutOfBoundsI8,
    /// Attempted to parse a number as an `i16` that was out of bounds.
    OutOfBoundsI16,
    /// Attempted to parse a number as an `i32` that was out of bounds.
    OutOfBoundsI32,
    /// Attempted to parse a number as an `i64` that was less than -2^53 or greater than 2^53.
    OutOfBoundsI64,
    /// Attempted to parse a number as an `u8` that was out of bounds.
    OutOfBoundsU8,
    /// Attempted to parse a number as an `u16` that was out of bounds.
    OutOfBoundsU16,
    /// Attempted to parse a number as an `u32` that was out of bounds.
    OutOfBoundsU32,
    /// Attempted to parse a number as an `u64` that was greater than 2^53.
    OutOfBoundsU64,
    /// Chars are represented as strings that contain one unicode scalar value.
    NotAChar,
    /// Attempted to read a string as base64-encoded bytes, but the string was not valid base64.
    Base64(base64::DecodeError),
    /// Expected a boolean, found something else.
    ExpectedBool,
    /// Expected a number, found something else.
    ExpectedNumber,
    /// Expected a string, found something else.
    ExpectedString,
    /// Expected null, found something else.
    ExpectedNull,
    /// Expected an array, found something else.
    ExpectedArray,
    /// Expected an object, found something else.
    ExpectedObject,
    /// Expected an enum, found something else.
    ExpectedEnum,
    /// Custom, stringly-typed error.
    Message(String),
}

impl fmt::Display for DecodeCborError {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::result::Result<(), fmt::Error> {
        fmt::Debug::fmt(self, f)
    }
}

impl error::Error for DecodeCborError {}

impl de::Error for DecodeCborError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        DecodeCborError::Message(msg.to_string())
    }
}

/// A structure that deserializes cbor encoded legacy message values.
pub struct CborDeserializer<'de> {
    input: &'de [u8],
}

impl<'de> CborDeserializer<'de> {
    /// Check whether end-of-input has been reached.
    pub fn end(&mut self) -> Result<(), DecodeCborError> {
        if self.input.len() == 0 {
            Ok(())
        } else {
            Err(DecodeCborError::TrailingBytes)
        }
    }
}

/// Try to parse data from the input. Validates that there are no trailing bytes.
pub fn from_slice<'de, T>(input: &'de [u8]) -> Result<T, DecodeCborError>
    where T: DeserializeOwned
{
    let mut de = CborDeserializer::from_slice(input);
    match Deserialize::deserialize(&mut de) {
        Ok(t) => de.end().map(|_| t),
        Err(e) => Err(e),
    }
}

/// Try to parse data from the input, returning the remaining input when done.
pub fn from_slice_partial<'de, T>(input: &'de [u8]) -> Result<(T, &'de [u8]), DecodeCborError>
    where T: DeserializeOwned
{
    let mut de = CborDeserializer::from_slice(input);
    match Deserialize::deserialize(&mut de) {
        Ok(t) => Ok((t, de.input)),
        Err(e) => Err(e),
    }
}

impl<'de> CborDeserializer<'de> {
    /// Creates a `Deserializer` from a `&[u8]`.
    pub fn from_slice(input: &'de [u8]) -> Self {
        CborDeserializer { input }
    }

    // Returns the next byte without consuming it.
    fn peek(&self) -> Result<u8, DecodeCborError> {
        match self.input.first() {
            Some(byte) => Ok(*byte),
            None => Err(DecodeCborError::UnexpectedEndOfInput),
        }
    }

    // Consumes the next byte and returns it.
    fn next(&mut self) -> Result<u8, DecodeCborError> {
        match self.input.split_first() {
            Some((head, tail)) => {
                self.input = tail;
                Ok(*head)
            }
            None => Err(DecodeCborError::UnexpectedEndOfInput),
        }
    }

    // Takes a tag and decodes the corresponding length of the string/collection.
    // Ignores the major type and assumes the additional type is between 0 and 27 (inclusive),
    // so don't call this with garbage.
    //
    // Only works on architectures where a u64 can be represented by a usize.
    #[cfg(target_pointer_width = "64")]
    fn decode_len(&mut self, mut tag: u8) -> Result<usize, DecodeCborError> {
        tag &= 0b000_11111;
        let len = match tag {
            len @ 0...23 => len as u64,
            24 => self.next()? as u64,
            25 => {
                let mut len = 0;

                for _ in 0..2 {
                    len <<= 8;
                    len |= self.next()? as u64;
                }

                u64::from_be(len)
            }
            26 => {
                let mut len = 0;

                for _ in 0..4 {
                    len <<= 8;
                    len |= self.next()? as u64;
                }

                u64::from_be(len)
            }
            27 => {
                let mut len = 0;

                for _ in 0..8 {
                    len <<= 8;
                    len |= self.next()? as u64;
                }

                u64::from_be(len)
            }
            _ => panic!(),
        };

        Ok(len as usize)
    }

    fn parse_bool(&mut self) -> Result<bool, DecodeCborError> {
        match self.next()? {
            0b111_10100 => Ok(false),
            0b111_10101 => Ok(true),
            _ => Err(DecodeCborError::ExpectedBool),
        }
    }

    fn parse_number(&mut self) -> Result<f64, DecodeCborError> {
        match self.next()? {
            0b111_11011 => {
                let mut raw_bits: u64 = 0;
                for _ in 0..8 {
                    raw_bits <<= 8;
                    let byte = self.next()?;
                    raw_bits |= byte as u64;
                }

                let parsed = f64::from_bits(raw_bits);

                if LegacyF64::is_valid(parsed) {
                    Ok(parsed)
                } else {
                    Err(DecodeCborError::InvalidNumber)
                }
            }
            _ => Err(DecodeCborError::ExpectedNumber),
        }
    }

    fn parse_str(&mut self) -> Result<&'de str, DecodeCborError> {
        match self.next()? {
            tag @ 0b011_00000...0b011_11011 => {
                let len = self.decode_len(tag)?;
                if self.input.len() < len {
                    return Err(DecodeCborError::InvalidLength);
                }

                let (s, remaining) = self.input.split_at(len);
                self.input = remaining;

                std::str::from_utf8(s).map_err(|_| DecodeCborError::InvalidStringContent)
            }

            _ => Err(DecodeCborError::ExpectedString),
        }
    }

    fn parse_string(&mut self) -> Result<String, DecodeCborError> {
        match self.next()? {
            tag @ 0b011_00000...0b011_11011 => {
                let len = self.decode_len(tag)?;
                if self.input.len() < len {
                    return Err(DecodeCborError::InvalidLength);
                }

                let mut data = Vec::with_capacity(len);
                data.extend_from_slice(&self.input[..len]);
                String::from_utf8(data).map_err(|_| DecodeCborError::InvalidStringContent)
            }

            _ => Err(DecodeCborError::ExpectedString),
        }
    }

    fn parse_null(&mut self) -> Result<(), DecodeCborError> {
        match self.next()? {
            0b111_10110 => Ok(()),
            _ => Err(DecodeCborError::ExpectedNull),
        }
    }
}

impl<'de, 'a> Deserializer<'de> for &'a mut CborDeserializer<'de> {
    type Error = DecodeCborError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        match self.peek()? {
            0b111_10100 => {
                let _ = self.next()?;
                visitor.visit_bool(false)
            }
            0b111_10101 => {
                let _ = self.next()?;
                visitor.visit_bool(true)
            }
            0b111_10110 => {
                let _ = self.next()?;
                visitor.visit_unit()
            }
            0b111_11011 => self.deserialize_f64(visitor),
            0b011_00000...0b011_11011 => self.deserialize_str(visitor),
            0b100_00000...0b100_11011 => self.deserialize_seq(visitor),
            0b101_00000...0b101_11011 => self.deserialize_map(visitor),
            _ => Err(DecodeCborError::ForbiddenType),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        visitor.visit_bool(self.parse_bool()?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        let f = self.parse_number()?;
        if f < std::i8::MIN as f64 || f > std::i8::MAX as f64 {
            Err(DecodeCborError::OutOfBoundsI8)
        } else {
            visitor.visit_i8(f as i8)
        }
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        let f = self.parse_number()?;
        if f < std::i16::MIN as f64 || f > std::i16::MAX as f64 {
            Err(DecodeCborError::OutOfBoundsI16)
        } else {
            visitor.visit_i16(f as i16)
        }
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        let f = self.parse_number()?;
        if f < std::i32::MIN as f64 || f > std::i32::MAX as f64 {
            Err(DecodeCborError::OutOfBoundsI32)
        } else {
            visitor.visit_i32(f as i32)
        }
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        let f = self.parse_number()?;
        if f < -9007199254740992.0f64 || f > 9007199254740992.0f64 {
            Err(DecodeCborError::OutOfBoundsI64)
        } else {
            visitor.visit_i64(f as i64)
        }
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        let f = self.parse_number()?;
        if f > std::u8::MAX as f64 {
            Err(DecodeCborError::OutOfBoundsU8)
        } else {
            visitor.visit_u8(f as u8)
        }
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        let f = self.parse_number()?;
        if f > std::u16::MAX as f64 {
            Err(DecodeCborError::OutOfBoundsU16)
        } else {
            visitor.visit_u16(f as u16)
        }
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        let f = self.parse_number()?;
        if f > std::u32::MAX as f64 {
            Err(DecodeCborError::OutOfBoundsU32)
        } else {
            visitor.visit_u32(f as u32)
        }
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        let f = self.parse_number()?;
        if f > 9007199254740992.0f64 {
            Err(DecodeCborError::OutOfBoundsU64)
        } else {
            visitor.visit_u64(f as u64)
        }
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        visitor.visit_f32(self.parse_number()? as f32)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        visitor.visit_f64(self.parse_number()?)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        let s = self.parse_string()?;
        let mut chars = s.chars();

        match chars.next() {
            None => return Err(DecodeCborError::NotAChar),
            Some(c) => {
                match chars.next() {
                    None => return visitor.visit_char(c),
                    Some(_) => return Err(DecodeCborError::NotAChar),
                }
            }
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        visitor.visit_str(self.parse_str()?)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        visitor.visit_string(self.parse_string()?)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        // We can't reference bytes directly since they are stored as base64 strings.
        // For the conversion, we need to allocate an owned buffer, so always do owned
        // deserialization.
        self.deserialize_byte_buf(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        match base64::decode(self.parse_str()?) {
            Ok(buf) => visitor.visit_byte_buf(buf),
            Err(e) => Err(DecodeCborError::Base64(e)),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        if self.input.starts_with(&[0b111_10110]) {
            self.input = &self.input[1..];
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        self.parse_null()?;
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(self,
                                  _name: &'static str,
                                  visitor: V)
                                  -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self,
                                     _name: &'static str,
                                     visitor: V)
                                     -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        let tag = self.next()?;
        if tag < 0b100_00000 || tag > 0b100_11011 {
            return Err(DecodeCborError::ExpectedArray);
        }

        let len = self.decode_len(tag)?;
        visitor.visit_seq(CollectionAccessor::new(&mut self, len))
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(self,
                                   _name: &'static str,
                                   _len: usize,
                                   visitor: V)
                                   -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        let tag = self.next()?;
        if tag < 0b101_00000 || tag > 0b101_11011 {
            return Err(DecodeCborError::ExpectedObject);
        }

        let len = self.decode_len(tag)?;
        visitor.visit_map(CollectionAccessor::new(&mut self, len))
    }

    fn deserialize_struct<V>(self,
                             _name: &'static str,
                             _fields: &'static [&'static str],
                             visitor: V)
                             -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(self,
                           _name: &'static str,
                           _variants: &'static [&'static str],
                           visitor: V)
                           -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        let tag = self.peek()?;
        if tag > 0b011_00000 && tag < 0b011_11011 {
            // Visit a unit variant.
            visitor.visit_enum(self.parse_string()?.into_deserializer())
        } else if tag < 0b101_00000 || tag > 0b101_11011 {
            Err(DecodeCborError::ExpectedEnum)
        } else {
            visitor.visit_enum(Enum::new(self))
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        self.deserialize_any(visitor)
    }
}

struct CollectionAccessor<'de, 'a> {
    des: &'a mut CborDeserializer<'de>,
    len: usize,
}

impl<'de, 'a> CollectionAccessor<'de, 'a> {
    fn new(des: &'a mut CborDeserializer<'de>, len: usize) -> CollectionAccessor<'de, 'a> {
        CollectionAccessor { des, len }
    }
}

impl<'de, 'a> SeqAccess<'de> for CollectionAccessor<'de, 'a> {
    type Error = DecodeCborError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, DecodeCborError>
        where T: DeserializeSeed<'de>
    {
        if self.len == 0 {
            return Ok(None);
        }

        self.len -= 1;
        seed.deserialize(&mut *self.des).map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

impl<'de, 'a> MapAccess<'de> for CollectionAccessor<'de, 'a> {
    type Error = DecodeCborError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, DecodeCborError>
        where K: DeserializeSeed<'de>
    {
        if self.len == 0 {
            return Ok(None);
        }

        self.len -= 1;

        seed.deserialize(&mut *self.des).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, DecodeCborError>
        where V: DeserializeSeed<'de>
    {
        seed.deserialize(&mut *self.des)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

struct Enum<'a, 'de: 'a> {
    des: &'a mut CborDeserializer<'de>,
}

impl<'a, 'de> Enum<'a, 'de> {
    fn new(des: &'a mut CborDeserializer<'de>) -> Self {
        Enum { des }
    }
}

impl<'de, 'a> EnumAccess<'de> for Enum<'a, 'de> {
    type Error = DecodeCborError;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), DecodeCborError>
        where V: DeserializeSeed<'de>
    {
        let val = seed.deserialize(&mut *self.des)?;
        Ok((val, self))
    }
}

impl<'de, 'a> VariantAccess<'de> for Enum<'a, 'de> {
    type Error = DecodeCborError;

    fn unit_variant(self) -> Result<(), DecodeCborError> {
        Err(DecodeCborError::ExpectedString)
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, DecodeCborError>
        where T: DeserializeSeed<'de>
    {
        seed.deserialize(self.des)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        Deserializer::deserialize_seq(self.des, visitor)
    }

    // Struct variants are represented in JSON as `{ NAME: { K: V, ... } }` so
    // deserialize the inner map here.
    fn struct_variant<V>(self,
                         _fields: &'static [&'static str],
                         visitor: V)
                         -> Result<V::Value, DecodeCborError>
        where V: Visitor<'de>
    {
        Deserializer::deserialize_map(self.des, visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{from_slice, to_vec};
    use super::super::super::Value;
    use super::super::super::LegacyF64;

    use std::collections::HashMap;
    use std::iter::repeat;

    fn repeat_n<T: Clone>(t: T, n: usize) -> Vec<T> {
        repeat(t).take(n).collect()
    }

    #[test]
    fn fixtures() {
        // A bunch of these are from https://tools.ietf.org/html/rfc7049#appendix-A

        assert!(from_slice::<Value>(&[0x00]).is_err()); // 0
        assert!(from_slice::<Value>(&[0x17]).is_err()); // 23
        assert!(from_slice::<Value>(&[0x18]).is_err()); // not enough input
        assert!(from_slice::<Value>(&[0x18, 0x18]).is_err()); // 24
        assert!(from_slice::<Value>(&[0x19, 0x03, 0xe8]).is_err()); // 1000
        assert!(from_slice::<Value>(&[0x20]).is_err()); // -1
        assert!(from_slice::<Value>(&[0x38, 0x63]).is_err()); // -100
        assert!(from_slice::<Value>(&[0xf9, 0x00, 0x00]).is_err()); // 0.0f16
        assert_eq!(from_slice::<Value>(&[0xf4]).unwrap(), Value::Bool(false));
        assert_eq!(from_slice::<Value>(&[0xf5]).unwrap(), Value::Bool(true));
        assert_eq!(from_slice::<Value>(&[0xf6]).unwrap(), Value::Null);
        assert_eq!(from_slice::<Value>(&[0xfb, 0x3f, 0xf1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9a])
                       .unwrap(),
                   Value::Float(LegacyF64::from_f64(1.1).unwrap()));
        assert!(from_slice::<Value>(&[0xf7]).is_err()); // undefined
        assert!(from_slice::<Value>(&[0xfb, 0x7f, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
                    .is_err()); // Infinity
        assert!(from_slice::<Value>(&[0xfb, 0x7f, 0xf8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
                    .is_err()); // NaN
        assert!(from_slice::<Value>(&[0xfb, 0xff, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
                    .is_err()); // -Infinity
        assert!(from_slice::<Value>(&[0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
                    .is_err()); // -0.0
        assert!(from_slice::<Value>(&[0xf0]).is_err()); // simple(16)
        assert!(from_slice::<Value>(&[0x40]).is_err()); // h''
        assert_eq!(from_slice::<Value>(&[0x60]).unwrap(),
                   Value::String("".to_string()));
        assert_eq!(from_slice::<Value>(&[0x61, 0x61]).unwrap(),
                   Value::String("a".to_string()));
        assert_eq!(from_slice::<Value>(&[0x80]).unwrap(), Value::Array(vec![]));
        assert_eq!(from_slice::<Value>(&[0x83, 0xf6, 0xf6, 0xf6]).unwrap(),
                   Value::Array(vec![Value::Null, Value::Null, Value::Null]));
        assert_eq!(from_slice::<Value>(&[0x98, 0x19, 0xf6, 0xf6, 0xf6, 0xf6, 0xf6, 0xf6, 0xf6,
                                         0xf6, 0xf6, 0xf6, 0xf6, 0xf6, 0xf6, 0xf6, 0xf6, 0xf6,
                                         0xf6, 0xf6, 0xf6, 0xf6, 0xf6, 0xf6, 0xf6, 0xf6, 0xf6])
                           .unwrap(),
                   Value::Array(repeat_n(Value::Null, 25)));
        assert_eq!(from_slice::<Value>(&[0xa0]).unwrap(),
                   Value::Object(HashMap::new()));
        assert!(from_slice::<Value>(&[0xa1, 0xf6, 0xf6]).is_err()); // {null: null}
        assert!(from_slice::<Value>(&[0xa2, 0xf6, 0xf6, 0xf6, 0xf6]).is_err()); // {null: null, null: null}

        let mut foo = HashMap::new();
        foo.insert("a".to_string(), Value::Null);
        foo.insert("b".to_string(),
                   Value::Array(vec![Value::Null, Value::Null]));
        assert_eq!(from_slice::<Value>(&[0xa2, 0x61, 0x61, 0xf6, 0x61, 0x62, 0x82, 0xf6, 0xf6])
                       .unwrap(),
                   Value::Object(foo));

        assert!(from_slice::<Value>(&[0xa2, 0x61, 0x61, 0xf6, 0x61, 0x61, 0x82, 0xf6, 0xf6])
                    .is_err()); // {"a": null, "a": [null, null]}
    }
}
