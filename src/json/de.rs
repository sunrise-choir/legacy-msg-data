use std::str::FromStr;

use encode_unicode::{Utf8Char, Utf16Char, U16UtfExt};

use super::super::abstract_::{self, Visitor, LegacyF64, DeserializeSeed};

/// Everything that can go wrong during deserialization.
pub enum Error {
    /// Needed more data but got EOF instead.
    UnexpectedEndOfInput,
    /// A generic syntax error. Any valid json would have been ok, but alas...
    Syntax,
    /// A number is valid json but it evaluates to -0 or an infinity
    InvalidNumber,
    /// The content of a string is not utf8, uses wrong escape sequences, etc.
    InvalidStringContent, // TODO implement a bunch of `From` traits for clearer parsing code with the `?` operator
    ExpectedBool,
    ExpectedNumber,
    ExpectedString,
    ExpectedNull,
    ExpectedArray,
    ExpectedObject,
}

pub type Result<T> = std::result::Result<T, Error>;

/// A structure that deserializes json encoded legacy message values.
pub struct Deserializer<'de> {
    input: &'de [u8],
    first: bool, // state for deserializing collections
}

pub fn from_slice<'de, T>(input: &'de [u8]) -> Result<T>
    where T: abstract_::DeserializeOwned
{
    let mut de = Deserializer::from_slice(input);
    abstract_::Deserialize::deserialize(&mut de)
}

fn is_ws(byte: u8) -> bool {
    byte == 0x09 || byte == 0x0A || byte == 0x0D || byte == 0x20
}

fn not_is_ws(byte: u8) -> bool {
    !is_ws(byte)
}

fn is_digit(byte: u8) -> bool {
    byte.is_ascii_digit()
}

impl<'de> Deserializer<'de> {
    pub fn from_slice(input: &'de [u8]) -> Self {
        Deserializer {
            input,
            first: false,
        }
    }

    // Returns the next byte without consuming it.
    fn peek(&self) -> Result<u8> {
        match self.input.first() {
            Some(byte) => Ok(*byte),
            None => Err(Error::UnexpectedEndOfInput),
        }
    }

    // Unsafely advance the input slice by 1 byte, to be used only after peeking.
    unsafe fn advance(&mut self) {
        self.input = std::slice::from_raw_parts(self.input.as_ptr().offset(1), self.input.len());
    }

    // Unsafely advance the input slice by some bytes.
    unsafe fn advance_by(&mut self, offset: isize) {
        self.input = std::slice::from_raw_parts(self.input.as_ptr().offset(offset),
                                                self.input.len());
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

    // Skips values while the predicate returns true, returns the first non-true value, consuming
    // it as well.
    fn consume_including(&mut self, pred: fn(u8) -> bool) -> Result<u8> {
        loop {
            let next = self.next()?;
            if pred(next) {
                return Ok(next);
            }
        }
    }

    // Consumes as much whitespace as possible, then consumes the next non-whitespace byte.
    fn next_ws(&mut self) -> Result<u8> {
        self.consume_including(not_is_ws)
    }

    fn expect_ws_err(&mut self, exp: u8, err: Error) -> Result<()> {
        if self.next_ws()? == exp {
            Ok(())
        } else {
            Err(err)
        }
    }

    // Skips values while the predicate returns true, returns the first non-true value but does
    // not consume it.
    fn consume_until(&mut self, pred: fn(u8) -> bool) -> Result<u8> {
        loop {
            let peeked = self.peek()?;
            if pred(peeked) {
                unsafe { self.advance() };
            } else {
                return Ok(peeked);
            }
        }
    }

    // Consumes as much whitespace as possible, then peeks at the next non-whitespace byte.
    fn peek_ws(&mut self) -> Result<u8> {
        self.consume_until(is_ws)
    }

    // Consumes the expected byt, gives the given error if it is something else
    fn expect_err(&mut self, expected: u8, err: Error) -> Result<()> {
        if self.next()? == expected {
            Ok(())
        } else {
            Err(err)
        }
    }

    // Same as expect, but using a predicate.
    fn expect_pred(&mut self, pred: fn(u8) -> bool) -> Result<()> {
        if pred(self.next()?) {
            Ok(())
        } else {
            Err(Error::Syntax)
        }
    }

    fn parse_bool(&mut self) -> Result<bool> {
        if self.input.starts_with(b"true") {
            self.input = &self.input[4..];
            return Ok(true);
        } else if self.input.starts_with(b"false") {
            self.input = &self.input[5..];
            return Ok(false);
        } else {
            Err(Error::ExpectedBool)
        }
    }

    fn parse_number(&mut self) -> Result<LegacyF64> {
        let original_input = self.input;

        // trailing `-`
        if self.peek()? == 0x2D {
            unsafe { self.advance() };
        }

        let next = self.next()?;
        match next {
            // first digit `0` must be followed by `.`
            0x30 => unsafe { self.advance() },
            // first digit nonzero, may be followed by more digits until the `.`
            0x31...0x39 => {
                unsafe {
                    self.advance();
                }
                self.consume_until(is_digit)?;
            }
            _ => return Err(Error::ExpectedNumber),
        }

        // `.`, followed by many1 digits
        if self.peek()? == 0x2E {
            unsafe {
                self.advance();
            }
            self.expect_pred(is_digit)?;
            self.consume_until(is_digit)?;
        }

        // `e` or `E`, followed by an optional sign and many1 digits
        if self.peek()? == 0x45 || self.peek()? == 0x65 {
            unsafe {
                self.advance();
            }

            // optional `+` or `-`
            if self.peek()? == 0x2B || self.peek()? == 0x2D {
                unsafe {
                    self.advance();
                }
            }

            // many1 digits
            self.expect_pred(is_digit)?;
            self.consume_until(is_digit)?;
        }

        // done parsing the number, convert it to a rust value
        let parsed = f64::from_str(unsafe {
                                       std::str::from_utf8_unchecked(&original_input
                                                                          [..(original_input.len() -
                                                                         self.input.len())])
                                   })
                .unwrap();

        match LegacyF64::from_f64(parsed) {
            Some(f) => Ok(f),
            None => Err(Error::InvalidNumber),
        }
    }

    fn parse_string(&mut self) -> Result<String> {
        self.expect_err(0x22, Error::ExpectedString)?;

        let mut decoded = String::new();

        loop {
            match self.peek()? {
                // terminating `"`, return the decoded string
                0x22 => {
                    unsafe {
                        self.advance();
                    }
                    return Ok(decoded);
                }

                // `\` introduces an escape sequence
                0x5C => {
                    unsafe {
                        self.advance();
                    }

                    match self.next()? {
                        // single character escape sequences
                        0x22 => decoded.push_str("\u{22}"), // `\"`
                        0x5C => decoded.push_str("\u{5C}"), // `\\`
                        0x2F => decoded.push_str("\u{2F}"), // `\/`
                        0x62 => decoded.push_str("\u{08}"), // `\b`
                        0x66 => decoded.push_str("\u{0C}"), // `\f`
                        0x6E => decoded.push_str("\u{0A}"), // `\n`
                        0x72 => decoded.push_str("\u{0D}"), // `\r`
                        0x74 => decoded.push_str("\u{09}"), // `\t`

                        // unicode escape sequences
                        0x75 => {
                            if self.input.len() < 4 {
                                return Err(Error::InvalidStringContent);
                            }

                            match u16::from_str_radix(unsafe {
                                std::str::from_utf8_unchecked(&self.input[..4])
                            }, 16) {
                                Ok(code_point) => {
                                    unsafe {
                                        self.advance_by(4);
                                    }

                                    if code_point.is_utf16_leading_surrogate() {
                                        // the unicode escape was for a leading ssurrogate, which
                                        // must be followed by another unicode escape which is a
                                        // trailing surrogate
                                        self.expect_err(0x5C, Error::InvalidStringContent)?;
                                        self.expect_err(0x75, Error::InvalidStringContent)?;
                                        if self.input.len() < 4 {
                                            return Err(Error::InvalidStringContent);
                                        }

                                        match u16::from_str_radix(unsafe {
                                            std::str::from_utf8_unchecked(&self.input[..4])
                                        }, 16) {
                                            Ok(code_point2) => {
                                                match Utf16Char::from_tuple((code_point, Some(code_point2))) {
                                                    Ok(c) => decoded.push(c.into()),
                                                    Err(_) => return Err(Error::InvalidStringContent),
                                                }
                                            }
                                            Err(_) => return Err(Error::InvalidStringContent),
                                        }
                                    } else {
                                        match std::char::from_u32(code_point as u32) {
                                            Some(c) => decoded.push(c),
                                            None => return Err(Error::InvalidStringContent),
                                        }
                                    }
                                }
                                Err(_) => return Err(Error::InvalidStringContent),
                            }
                        }

                        // Nothing else may follow an unescaped `\`
                        _ => return Err(Error::InvalidStringContent),
                    }
                }

                // the control code points must be escaped
                0x00...0x19 => return Err(Error::InvalidStringContent),

                // a regular utf8-encoded code point (unless it is malformed)
                _ => {
                    match Utf8Char::from_slice_start(self.input) {
                        Err(_) => return Err(Error::InvalidStringContent),
                        Ok((_, len)) => unsafe {
                            decoded.push_str(std::str::from_utf8_unchecked(&self.input[..len]));
                            self.advance_by(len as isize);
                        },
                    }
                }
            }
        }
    }

    fn parse_null(&mut self) -> Result<()> {
        if self.input.starts_with(b"null") {
            self.input = &self.input[4..];
            return Ok(());
        } else {
            Err(Error::ExpectedNull)
        }
    }
}

impl<'de, 'a> abstract_::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        match self.peek_ws()? {
            0x6E => {
                if self.input[1..].starts_with(b"ull") {
                    self.input = &self.input[4..];
                    visitor.visit_null()
                } else {
                    Err(Error::Syntax)
                }
            }
            0x66 => {
                if self.input[1..].starts_with(b"alse") {
                    self.input = &self.input[5..];
                    visitor.visit_bool(false)
                } else {
                    Err(Error::Syntax)
                }
            }
            0x74 => {
                if self.input[1..].starts_with(b"rue") {
                    self.input = &self.input[4..];
                    visitor.visit_bool(true)
                } else {
                    Err(Error::Syntax)
                }
            }
            0x22 => self.deserialize_str(visitor),
            0x5B => self.deserialize_array(visitor),
            0x7B => self.deserialize_object(visitor),
            0x2D | 0x30...0x39 => self.deserialize_f64(visitor),
            _ => Err(Error::Syntax),
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
        // We can't reference json strings directly since they contain escape sequences.
        // For the conversion, we need to allocate an owned buffer, so always do owned
        // deserialization.
        self.deserialize_string(visitor)
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
        self.expect_err(0x5B, Error::ExpectedArray)?;
        self.first = true;
        let value = visitor.visit_array(&mut self)?;
        self.expect_ws_err(0x5D, Error::Syntax)?;
        Ok(value)
    }

    fn deserialize_object<V>(mut self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.expect_err(0x7B, Error::ExpectedObject)?;
        self.first = true;
        let value = visitor.visit_object(&mut self)?;
        self.expect_ws_err(0x7D, Error::Syntax)?;
        Ok(value)
    }
}

impl<'de, 'a> abstract_::deserializer::ArrayAccess<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
        where T: DeserializeSeed<'de>
    {
        // Array ends at `]`
        if let 0x5D = self.peek_ws()? {
            return Ok(None);
        }

        // expect `,` before every item except the first
        if self.first {
            self.first = false;
        } else {
            self.expect_ws_err(0x2C, Error::Syntax)?;
        }

        self.consume_until(is_ws)?;

        seed.deserialize(&mut **self).map(Some)
    }
}

impl<'de, 'a> abstract_::deserializer::ObjectAccess<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, _seed: K) -> Result<Option<String>> {
        // Object ends at `}`
        if let 0x7D = self.peek_ws()? {
            return Ok(None);
        }

        // expect `,` before every item except the first
        if self.first {
            self.first = false;
        } else {
            self.expect_ws_err(0x2C, Error::Syntax)?;
        }

        self.consume_until(is_ws)?;
        self.parse_string().map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
        where V: DeserializeSeed<'de>
    {
        // `:`
        self.expect_ws_err(0x3A, Error::Syntax)?;

        self.consume_until(is_ws)?;
        seed.deserialize(&mut **self)
    }
}
