use super::super::abstract_::{self, Visitor, LegacyF64, DeserializeSeed};

/// Everything that can go wrong during deserialization.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum Error {
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

pub type Result<T> = std::result::Result<T, Error>;

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
            Err(Error::TrailingBytes)
        }
    }
}

/// Try to parse data from the input. Validates that there are no trailing bytes.
pub fn from_slice<'de, T>(input: &'de [u8]) -> Result<T>
    where T: abstract_::DeserializeOwned
{
    let mut de = Deserializer::from_slice(input);
    match abstract_::Deserialize::deserialize(&mut de) {
        Ok(t) => de.end().map(|_| t),
        Err(e) => Err(e),
    }
}

impl<'de> Deserializer<'de> {
    pub fn from_slice(input: &'de [u8]) -> Self {
        Deserializer { input }
    }

    // Returns the next byte without consuming it.
    fn peek(&self) -> Result<u8> {
        match self.input.first() {
            Some(byte) => Ok(*byte),
            None => Err(Error::UnexpectedEndOfInput),
        }
    }

    // Consumes the next byte and returns it.
    fn next(&mut self) -> Result<u8> {
        match self.input.split_first() {
            Some((head, tail)) => {
                self.input = tail;
                Ok(*head)
            }
            None => Err(Error::UnexpectedEndOfInput),
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
            _ => Err(Error::ExpectedBool),
        }
    }

    fn parse_number(&mut self) -> Result<LegacyF64> {
        match self.next()? {
            0b111_11011 => {
                let mut raw_bits_big_endian: u64 = 0;
                for _ in 0..8 {
                    raw_bits_big_endian <<= 8;
                    let byte = self.next()?;
                    raw_bits_big_endian |= byte as u64;
                }

                let raw_bits_host_endian = u64::from_be(raw_bits_big_endian);
                let parsed = f64::from_bits(raw_bits_host_endian);

                match LegacyF64::from_f64(parsed) {
                    Some(f) => Ok(f),
                    None => Err(Error::InvalidNumber),
                }
            }
            _ => Err(Error::ExpectedNumber),
        }
    }

    fn parse_str(&mut self) -> Result<&'de str> {
        match self.next()? {
            tag @ 0b011_00000...0b011_11011 => {
                let len = self.decode_len(tag)?;
                if self.input.len() < len {
                    return Err(Error::InvalidLength);
                }

                std::str::from_utf8(&self.input[..len]).map_err(|_| Error::InvalidStringContent)
            }

            _ => Err(Error::ExpectedString),
        }
    }

    fn parse_string(&mut self) -> Result<String> {
        match self.next()? {
            tag @ 0b011_00000...0b011_11011 => {
                let len = self.decode_len(tag)?;
                if self.input.len() < len {
                    return Err(Error::InvalidLength);
                }

                let mut data = Vec::with_capacity(len);
                data.extend_from_slice(&self.input[..len]);
                String::from_utf8(data).map_err(|_| Error::InvalidStringContent)
            }

            _ => Err(Error::ExpectedString),
        }
    }

    fn parse_null(&mut self) -> Result<()> {
        match self.next()? {
            0b111_10110 => Ok(()),
            _ => Err(Error::ExpectedNull),
        }
    }
}

impl<'de, 'a> abstract_::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        match self.peek()? {
            0b111_10100 => visitor.visit_bool(false),
            0b111_10101 => visitor.visit_bool(true),
            0b111_10110 => visitor.visit_null(),
            0b111_11011 => self.deserialize_f64(visitor),
            0b011_00000...0b011_11011 => self.deserialize_str(visitor),
            0b100_00000...0b100_11011 => self.deserialize_array(visitor),
            0b101_00000...0b101_11011 => self.deserialize_object(visitor),
            _ => Err(Error::ForbiddenType),
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
            return Err(Error::ExpectedArray);
        }

        let len = self.decode_len(tag)?;
        visitor.visit_array(CollectionAccessor::new(&mut self, len))
    }

    fn deserialize_object<V>(mut self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        let tag = self.next()?;
        if tag < 0b100_00000 || tag > 0b100_11011 {
            return Err(Error::ExpectedObject);
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

impl<'de, 'a> abstract_::deserializer::ArrayAccess<'de> for CollectionAccessor<'de, 'a> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
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

// TODO figure out how to do a version of this trait that allows borrowing keys
impl<'de, 'a> abstract_::deserializer::ObjectAccess<'de> for CollectionAccessor<'de, 'a> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<String>>
        where K: abstract_::deserializer::ObjectAccessState
    {
        if self.len == 0 {
            return Ok(None);
        }

        self.len -= 1;

        let key = self.des.parse_str()?;

        if seed.has_key(key) {
            Err(Error::DuplicateKey)
        } else {
            Ok(Some(key.to_string()))
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
        where V: DeserializeSeed<'de>
    {
        seed.deserialize(&mut *self.des)
    }

    /// Can't correctly decode ssb messages without using state for detecting duplicat keys.
    fn next_key<K>(&mut self) -> Result<Option<String>>
        where K: abstract_::deserializer::ObjectAccessState
    {
        panic!()
    }

    /// Can't correctly decode ssb messages without using state for detecting duplicat keys.
    fn next_entry<K, V>(&mut self) -> Result<Option<(String, V)>>
        where K: abstract_::deserializer::ObjectAccessState,
              V: abstract_::deserialize::Deserialize<'de>
    {
        panic!()
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Value, from_slice, to_vec};
}