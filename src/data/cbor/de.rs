use std::{error, fmt};

use super::super::{
    LegacyF64,
    de::{
        self,
        Visitor,
    }
};

/// Everything that can go wrong during deserialization.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
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
    /// An object has multiple entries with the equal keys.
    DuplicateKey,
    /// The input contained valid cbor followed by at least one more byte.
    TrailingBytes,
    ExpectedBool,
    ExpectedNumber,
    ExpectedString,
    ExpectedNull,
    ExpectedArray,
    ExpectedObject,
}

impl fmt::Display for DecodeCborError {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::result::Result<(), fmt::Error> {
        fmt::Debug::fmt(self, f)
    }
}

impl error::Error for DecodeCborError {}

pub type Result<T> = std::result::Result<T, DecodeCborError>;

/// A structure that deserializes cbor encoded legacy message values.
pub struct Deserializer<'de> {
    input: &'de [u8],
}

impl<'de> Deserializer<'de> {
    /// Check whether end-of-input has been reached.
    pub fn end(&mut self) -> Result<()> {
        if self.input.len() == 0 {
            Ok(())
        } else {
            Err(DecodeCborError::TrailingBytes)
        }
    }
}

/// Try to parse data from the input. Validates that there are no trailing bytes.
pub fn from_slice<'de, T>(input: &'de [u8]) -> Result<T>
    where T: de::DeserializeOwned
{
    let mut de = Deserializer::from_slice(input);
    match de::Deserialize::deserialize(&mut de) {
        Ok(t) => de.end().map(|_| t),
        Err(e) => Err(e),
    }
}

impl<'de> Deserializer<'de> {
    /// Creates a `Deserializer` from a `&[u8]`.
    pub fn from_slice(input: &'de [u8]) -> Self {
        Deserializer { input }
    }

    // Returns the next byte without consuming it.
    fn peek(&self) -> Result<u8> {
        match self.input.first() {
            Some(byte) => Ok(*byte),
            None => Err(DecodeCborError::UnexpectedEndOfInput),
        }
    }

    // Consumes the next byte and returns it.
    fn next(&mut self) -> Result<u8> {
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
    fn decode_len(&mut self, mut tag: u8) -> Result<usize> {
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

    fn parse_bool(&mut self) -> Result<bool> {
        match self.next()? {
            0b111_10100 => Ok(false),
            0b111_10101 => Ok(true),
            _ => Err(DecodeCborError::ExpectedBool),
        }
    }

    fn parse_number(&mut self) -> Result<LegacyF64> {
        match self.next()? {
            0b111_11011 => {
                let mut raw_bits: u64 = 0;
                for _ in 0..8 {
                    raw_bits <<= 8;
                    let byte = self.next()?;
                    raw_bits |= byte as u64;
                }

                let parsed = f64::from_bits(raw_bits);

                match LegacyF64::from_f64(parsed) {
                    Some(f) => Ok(f),
                    None => Err(DecodeCborError::InvalidNumber),
                }
            }
            _ => Err(DecodeCborError::ExpectedNumber),
        }
    }

    fn parse_str(&mut self) -> Result<&'de str> {
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

    fn parse_string(&mut self) -> Result<String> {
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

    fn parse_null(&mut self) -> Result<()> {
        match self.next()? {
            0b111_10110 => Ok(()),
            _ => Err(DecodeCborError::ExpectedNull),
        }
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = DecodeCborError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
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
                visitor.visit_null()
            }
            0b111_11011 => self.deserialize_f64(visitor),
            0b011_00000...0b011_11011 => self.deserialize_str(visitor),
            0b100_00000...0b100_11011 => self.deserialize_array(visitor),
            0b101_00000...0b101_11011 => self.deserialize_object(visitor),
            _ => Err(DecodeCborError::ForbiddenType),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_bool(self.parse_bool()?)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_f64(self.parse_number()?)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_str(self.parse_str()?)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_string(self.parse_string()?)
    }

    fn deserialize_null<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.parse_null()?;
        visitor.visit_null()
    }

    fn deserialize_array<V>(mut self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        let tag = self.next()?;
        if tag < 0b100_00000 || tag > 0b100_11011 {
            return Err(DecodeCborError::ExpectedArray);
        }

        let len = self.decode_len(tag)?;
        visitor.visit_array(CollectionAccessor::new(&mut self, len))
    }

    fn deserialize_object<V>(mut self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        let tag = self.next()?;
        if tag < 0b101_00000 || tag > 0b101_11011 {
            return Err(DecodeCborError::ExpectedObject);
        }

        let len = self.decode_len(tag)?;
        visitor.visit_object(CollectionAccessor::new(&mut self, len))
    }
}

struct CollectionAccessor<'de, 'a> {
    des: &'a mut Deserializer<'de>,
    len: usize,
}

impl<'de, 'a> CollectionAccessor<'de, 'a> {
    fn new(des: &'a mut Deserializer<'de>, len: usize) -> CollectionAccessor<'de, 'a> {
        CollectionAccessor { des, len }
    }
}

impl<'de, 'a> de::ArrayAccess<'de> for CollectionAccessor<'de, 'a> {
    type Error = DecodeCborError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
        where T: de::DeserializeSeed<'de>
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

// TODO figure out how to do a version of this trait that allows borrowing keys
impl<'de, 'a> de::ObjectAccess<'de> for CollectionAccessor<'de, 'a> {
    type Error = DecodeCborError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<String>>
        where K: de::ObjectAccessState
    {
        if self.len == 0 {
            return Ok(None);
        }

        self.len -= 1;

        let key = self.des.parse_str()?;

        if seed.has_key(key) {
            Err(DecodeCborError::DuplicateKey)
        } else {
            Ok(Some(key.to_string()))
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
        where V: de::DeserializeSeed<'de>
    {
        seed.deserialize(&mut *self.des)
    }

    /// Can't correctly decode ssb messages without using state for detecting duplicat keys.
    fn next_key<K>(&mut self) -> Result<Option<String>>
        where K: de::ObjectAccessState
    {
        panic!()
    }

    /// Can't correctly decode ssb messages without using state for detecting duplicat keys.
    fn next_entry<K, V>(&mut self) -> Result<Option<(String, V)>>
        where K: de::ObjectAccessState,
              V: de::Deserialize<'de>
    {
        panic!()
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
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
