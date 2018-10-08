use std::io;

use super::super::abstract_::{
    serialize::Serialize,
    serializer::{
        Serializer,
        SerializeArray,
        SerializeObject
    },
    LegacyF64
};

/// A structure for serializing ssb legacy values into the cbor encoding.
pub struct CborSerializer<W> {
    writer: W,
}

impl<W> CborSerializer<W>
    where W: io::Write
{
    /// Creates a new serializer.
    pub fn new(writer: W) -> Self {
        CborSerializer { writer }
    }

    /// Unwrap the `Writer` from the `Serializer`.
    pub fn into_inner(self) -> W {
        self.writer
    }

    // Writes the given length. Only the three most significant bytes of `tag` are used (to
    // distinguish between )
    #[cfg(target_pointer_width = "64")]
    fn write_len(&mut self, len: usize, major: LenMajor) -> Result<(), io::Error> {
        let mut tag = match major {
            LenMajor::Utf8String => 0b011_00000,
            LenMajor::Array => 0b100_00000,
            LenMajor::Map => 0b101_00000,
        };

        match len {
            0...23 => {
                tag |= len as u8;
                self.writer.write_all(&[tag])
            },
            24...255 => {
                tag |= 24;
                self.writer.write_all(&[tag])?;
                let len_be = len as u8;
                self.writer.write_all(&[len_be])
            },
            256...65535 => {
                tag |= 25;
                self.writer.write_all(&[tag])?;
                let len_be: [u8; 2] = unsafe {
                    std::mem::transmute(u16::to_be(len as u16))
                };
                self.writer.write_all(&len_be[..])
            },
            65536...4294967295 => {
                tag |= 26;
                self.writer.write_all(&[tag])?;
                let len_be: [u8; 4] = unsafe {
                    std::mem::transmute(u32::to_be(len as u32))
                };
                self.writer.write_all(&len_be[..])
            },
            _ => {
                tag |= 27;
                self.writer.write_all(&[tag])?;
                let len_be: [u8; 8] = unsafe {
                    std::mem::transmute(u64::to_be(len as u64))
                };
                self.writer.write_all(&len_be[..])
            },
        }
    }
}

enum LenMajor {
    Utf8String,
    Array,
    Map
}

/// Serialize the given data structure as cbor into the IO stream.
pub fn to_writer<W, T: ?Sized>(writer: W, value: &T) -> Result<(), io::Error>
    where W: io::Write,
          T: Serialize
{
    let mut ser = CborSerializer::new(writer);
    value.serialize(&mut ser)
}

/// Serialize the given data structure as a JSON byte vector.
pub fn to_vec<T: ?Sized>(value: &T) -> Vec<u8>
    where T: Serialize
{
    let mut writer = Vec::with_capacity(128);
    to_writer(&mut writer, value).unwrap();
    writer
}

/// Serialize the given data structure as a String of JSON.
pub fn to_string<T: ?Sized>(value: &T) -> String
where
    T: Serialize,
{
    let vec = to_vec(value);
    let string = unsafe {
        // We do not emit invalid UTF-8.
        String::from_utf8_unchecked(vec)
    };
    string
}

impl<'a, W> Serializer for &'a mut CborSerializer<W>
    where W: io::Write,
{
    type Ok = ();
    type Error = io::Error;
    type SerializeArray = CollectionSerializer<'a, W>;
    type SerializeObject = CollectionSerializer<'a, W>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.writer.write_all(if v {
            &[0b111_10101]
        } else {
            &[0b111_10100]
        })
    }

    fn serialize_f64(self, v: LegacyF64) -> Result<Self::Ok, Self::Error> {
        self.writer.write_all(&[0b111_11011])?;

        let bytes: [u8; 8] = unsafe {
            std::mem::transmute(u64::to_be(f64::to_bits(v.into())))
        };

        self.writer.write_all(&bytes[..])
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.write_len(v.len(), LenMajor::Utf8String)?;
        self.writer.write_all(v.as_bytes())
    }

    fn serialize_null(self) -> Result<Self::Ok, Self::Error> {
        self.writer.write_all(&[0b111_10110])
    }

    fn serialize_array(self, len: usize) -> Result<Self::SerializeArray, Self::Error> {
        self.write_len(len, LenMajor::Array)?;
        Ok(CollectionSerializer::new(&mut *self))
    }

    fn serialize_object(self, len: usize) -> Result<Self::SerializeObject, Self::Error> {
        self.write_len(len, LenMajor::Map)?;
        Ok(CollectionSerializer::new(&mut *self))
    }
}

#[doc(hidden)]
pub struct CollectionSerializer<'a, W> {
    ser: &'a mut CborSerializer<W>
}

impl<'a, W: io::Write> CollectionSerializer<'a, W> {
    fn new(ser: &'a mut CborSerializer<W>) -> CollectionSerializer<'a, W> {
        CollectionSerializer {
            ser
        }
    }
}

impl<'a, W> SerializeArray for CollectionSerializer<'a, W>
where W: io::Write
{
    type Ok = ();
    type Error = io::Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> where T: Serialize {
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, W> SerializeObject for CollectionSerializer<'a, W>
where W: io::Write
{
    type Ok = ();
    type Error = io::Error;

    fn serialize_key(&mut self, value: &str) -> Result<(), Self::Error> {
        self.ser.serialize_str(value)
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> where T: Serialize {
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}
