use std::io;

use ryu_ecmascript;

use super::super::abstract_::{
    serialize::Serialize,
    serializer::{
        Serializer,
        SerializeArray,
        SerializeObject
    },
    LegacyF64
};

/// A structure for serializing ssb legacy values into the json encoding.
pub struct JsonSerializer<W> {
    writer: W,
    compact: bool, // if true omits whitespace, else produces the signing format
    indent: usize,
    first_elem: bool, // state for collection serialization
}

impl<W> JsonSerializer<W>
    where W: io::Write
{
    /// Creates a new serializer.
    ///
    /// If `compact`, this omits all whitespace. For signing or signature checking,
    /// set `compact` to `false`.
    #[inline]
    pub fn new(writer: W, compact: bool) -> Self {
        JsonSerializer { writer, compact, indent: 0, first_elem: true }
    }

    /// Unwrap the `Writer` from the `Serializer`.
    pub fn into_inner(self) -> W {
        self.writer
    }

    // Writes the correct number of spaces as indentation.
    fn write_indent(&mut self) -> Result<(), io::Error> {
        for _ in 0..self.indent {
            self.writer.write_all(b"  ")?;
        }
        Ok(())
    }
}

/// Serialize the given data structure as JSON into the IO stream.
pub fn to_writer<W, T: ?Sized>(writer: W, value: &T, compact: bool) -> Result<(), io::Error>
    where W: io::Write,
          T: Serialize
{
    let mut ser = JsonSerializer::new(writer, compact);
    value.serialize(&mut ser)
}

/// Serialize the given data structure as a JSON byte vector.
pub fn to_vec<T: ?Sized>(value: &T, compact: bool) -> Vec<u8>
    where T: Serialize
{
    let mut writer = Vec::with_capacity(128);
    to_writer(&mut writer, value, compact).unwrap();
    writer
}

/// Serialize the given data structure as a String of JSON.
pub fn to_string<T: ?Sized>(value: &T, compact: bool) -> String
where
    T: Serialize,
{
    let vec = to_vec(value, compact);
    let string = unsafe {
        // We do not emit invalid UTF-8.
        String::from_utf8_unchecked(vec)
    };
    string
}

impl<'a, W> Serializer for &'a mut JsonSerializer<W>
    where W: io::Write,
{
    type Ok = ();
    type Error = io::Error;
    type SerializeArray = Self;
    type SerializeObject = Self;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        let s = if v {
            b"true" as &[u8]
        } else {
            b"false" as &[u8]
        };
        self.writer.write_all(s)
    }

    fn serialize_f64(self, v: LegacyF64) -> Result<Self::Ok, Self::Error> {
        let mut buffer = ryu_ecmascript::Buffer::new();
        let s = buffer.format::<f64>(v.into());
        self.writer.write_all(s.as_bytes())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.writer.write_all(b"\"")?;

        for byte in v.bytes() {
            match byte {
                0x00 => self.writer.write_all(br"\u0000")?,
                0x01 => self.writer.write_all(br"\u0001")?,
                0x02 => self.writer.write_all(br"\u0002")?,
                0x03 => self.writer.write_all(br"\u0003")?,
                0x04 => self.writer.write_all(br"\u0004")?,
                0x05 => self.writer.write_all(br"\u0005")?,
                0x06 => self.writer.write_all(br"\u0006")?,
                0x07 => self.writer.write_all(br"\u0007")?,
                0x08 => self.writer.write_all(br"\b")?,
                0x09 => self.writer.write_all(br"\t")?,
                0x0A => self.writer.write_all(br"\n")?,
                0x0B => self.writer.write_all(br"\u000B")?,
                0x0C => self.writer.write_all(br"\f")?,
                0x0D => self.writer.write_all(br"\r")?,
                0x0E => self.writer.write_all(br"\u000e")?,
                0x0F => self.writer.write_all(br"\u000f")?,
                0x10 => self.writer.write_all(br"\u0010")?,
                0x11 => self.writer.write_all(br"\u0011")?,
                0x12 => self.writer.write_all(br"\u0012")?,
                0x13 => self.writer.write_all(br"\u0013")?,
                0x14 => self.writer.write_all(br"\u0014")?,
                0x15 => self.writer.write_all(br"\u0015")?,
                0x16 => self.writer.write_all(br"\u0016")?,
                0x17 => self.writer.write_all(br"\u0017")?,
                0x18 => self.writer.write_all(br"\u0018")?,
                0x19 => self.writer.write_all(br"\u0019")?,
                0x1A => self.writer.write_all(br"\u001a")?,
                0x1B => self.writer.write_all(br"\u001b")?,
                0x1C => self.writer.write_all(br"\u001c")?,
                0x1D => self.writer.write_all(br"\u001d")?,
                0x1E => self.writer.write_all(br"\u001e")?,
                0x1F => self.writer.write_all(br"\u001f")?,
                0x22 => self.writer.write_all(b"\\\"")?,
                0x5C => self.writer.write_all(br"\\")?,
                other => self.writer.write_all(&[other])?,
            }
        }

        self.writer.write_all(b"\"")
    }

    fn serialize_null(self) -> Result<Self::Ok, Self::Error> {
        self.writer.write_all(b"null")
    }

    fn serialize_array(self, _len: usize) -> Result<Self::SerializeArray, Self::Error> {
        self.writer.write_all(b"[")?;

        if !self.compact {
            self.writer.write_all(b"\n")?;
            self.indent += 1;
            self.write_indent()?;
        }

        self.first_elem = true;

        Ok(self)
    }

    fn serialize_object(self, _len: usize) -> Result<Self::SerializeObject, Self::Error> {
        self.writer.write_all(b"{")?;

        if !self.compact {
            self.writer.write_all(b"\n")?;
            self.indent += 1;
            self.write_indent()?;
        }

        self.first_elem = true;

        Ok(self)
    }
}

impl<'a, W> SerializeArray for &'a mut JsonSerializer<W>
where W: io::Write
{
    type Ok = ();
    type Error = io::Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> where T: Serialize {
        if !self.compact {
            self.write_indent()?;
        }

        if self.first_elem {
            self.first_elem = false;
        } else {
            self.writer.write_all(b",")?;
        }

        value.serialize(&mut **self)?;

        if !self.compact {
            self.writer.write_all(b"\n")?;
        }

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        if !self.compact {
            self.indent -= 1;
            self.write_indent()?;
        }

        self.writer.write_all(b"]")
    }
}

impl<'a, W> SerializeObject for &'a mut JsonSerializer<W>
where W: io::Write
{
    type Ok = ();
    type Error = io::Error;

    fn serialize_key(&mut self, value: &str) -> Result<(), Self::Error> {
        if !self.compact {
            self.write_indent()?;
        }

        if self.first_elem {
            self.first_elem = false;
        } else {
            self.writer.write_all(b",")?;
        }

        self.serialize_str(value)?;

        if self.compact {
            self.writer.write_all(b":")
        } else {
            self.writer.write_all(b": ")
        }
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> where T: Serialize {
        value.serialize(&mut **self)?;

        if !self.compact {
            self.writer.write_all(b"\n")?;
        }

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        if !self.compact {
            self.indent -= 1;
            self.write_indent()?;
        }

        self.writer.write_all(b"}")
    }
}
