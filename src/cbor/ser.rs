use std::{error, fmt, io};

use serde::ser::{self, Serializer, Serialize, SerializeSeq, SerializeStructVariant,
                 SerializeStruct, SerializeMap, SerializeTupleVariant, SerializeTupleStruct,
                 SerializeTuple};

use super::super::{LegacyF64, is_i64_valid, is_u64_valid};

/// Everything that can go wrong during cbor serialization.
#[derive(Debug)]
pub enum EncodeCborError {
    /// An IO error occured on the underlying writer.
    ///
    /// When serializing directly into a `Vec<u8>` or `String`, this error never occurs.
    Io(io::Error),
    /// Tried to serialize a number forbidden by the ssb data format (an inifinity, NaN or -0.0).
    InvalidFloat(f64),
    /// Tried to serialize an unsigned integer larger than 2^53 (these are not
    /// guaranteed to be represented correctly in a 64 bit float).
    InvalidUnsignedInteger(u64),
    /// Tried to serialize an signed integer with absolute value larger than 2^53 (these are not
    /// guaranteed to be represented correctly in a 64 bit float).
    InvalidSignedInteger(i64),
    /// Can only serialize collections whose length is known upfront.
    UnknownLength,
    /// Custom, stringly-typed error.
    Message(String),
}

impl fmt::Display for EncodeCborError {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::result::Result<(), fmt::Error> {
        fmt::Debug::fmt(self, f)
    }
}

impl error::Error for EncodeCborError {}

impl ser::Error for EncodeCborError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        EncodeCborError::Message(msg.to_string())
    }
}

impl From<io::Error> for EncodeCborError {
    fn from(e: io::Error) -> Self {
        EncodeCborError::Io(e)
    }
}

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
            }
            24...255 => {
                tag |= 24;
                self.writer.write_all(&[tag])?;
                let len_be = len as u8;
                self.writer.write_all(&[len_be])
            }
            256...65535 => {
                tag |= 25;
                self.writer.write_all(&[tag])?;
                let len_be: [u8; 2] = unsafe { std::mem::transmute(u16::to_be(len as u16)) };
                self.writer.write_all(&len_be[..])
            }
            65536...4294967295 => {
                tag |= 26;
                self.writer.write_all(&[tag])?;
                let len_be: [u8; 4] = unsafe { std::mem::transmute(u32::to_be(len as u32)) };
                self.writer.write_all(&len_be[..])
            }
            _ => {
                tag |= 27;
                self.writer.write_all(&[tag])?;
                let len_be: [u8; 8] = unsafe { std::mem::transmute(u64::to_be(len as u64)) };
                self.writer.write_all(&len_be[..])
            }
        }
    }
}

enum LenMajor {
    Utf8String,
    Array,
    Map,
}

/// Serialize the given data structure as cbor into the IO stream.
pub fn to_writer<W, T: ?Sized>(writer: W, value: &T) -> Result<(), EncodeCborError>
    where W: io::Write,
          T: Serialize
{
    let mut ser = CborSerializer::new(writer);
    value.serialize(&mut ser)
}

/// Serialize the given data structure as a cbor byte vector.
pub fn to_vec<T: ?Sized>(value: &T) -> Result<Vec<u8>, EncodeCborError>
    where T: Serialize
{
    let mut writer = Vec::with_capacity(128);
    to_writer(&mut writer, value).map(|_| writer)
}

impl<'a, W> Serializer for &'a mut CborSerializer<W>
    where W: io::Write
{
    type Ok = ();
    type Error = EncodeCborError;

    type SerializeSeq = CollectionSerializer<'a, W>;
    type SerializeTuple = CollectionSerializer<'a, W>;
    type SerializeTupleStruct = CollectionSerializer<'a, W>;
    type SerializeTupleVariant = CollectionSerializer<'a, W>;
    type SerializeMap = CollectionSerializer<'a, W>;
    type SerializeStruct = CollectionSerializer<'a, W>;
    type SerializeStructVariant = CollectionSerializer<'a, W>;

    fn is_human_readable(&self) -> bool {
        false
    }

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(self.writer
               .write_all(if v { &[0b111_10101] } else { &[0b111_10100] })?)
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        if is_i64_valid(v) {
            self.serialize_f64(v as f64)
        } else {
            Err(EncodeCborError::InvalidSignedInteger(v))
        }
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        if is_u64_valid(v) {
            self.serialize_f64(v as f64)
        } else {
            Err(EncodeCborError::InvalidUnsignedInteger(v))
        }
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.serialize_f64(v as f64)
    }

    // https://spec.scuttlebutt.nz/datamodel.html#signing-encoding-floats
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        if LegacyF64::is_valid(v) {
            self.writer.write_all(&[0b111_11011])?;

            let bytes: [u8; 8] = unsafe { std::mem::transmute(u64::to_be(f64::to_bits(v.into()))) };

            Ok(self.writer.write_all(&bytes[..])?)
        } else {
            Err(EncodeCborError::InvalidFloat(v))
        }
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.write_len(v.len(), LenMajor::Utf8String)?;
        Ok(self.writer.write_all(v.as_bytes())?)
    }

    // Serializing as base64.
    //
    // This not mandated by the spec in any way. From the spec's perspective, this
    // outputs a string like any other.
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(&base64::encode(v))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
        where T: ?Sized + Serialize
    {
        value.serialize(self)
    }

    // https://spec.scuttlebutt.nz/datamodel.html#signing-encoding-null
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.writer.write_all(&[0b111_10110])?)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(self,
                              _name: &'static str,
                              _variant_index: u32,
                              variant: &'static str)
                              -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T>(self,
                                   _name: &'static str,
                                   value: &T)
                                   -> Result<Self::Ok, Self::Error>
        where T: ?Sized + Serialize
    {
        value.serialize(self)
    }

    // https://spec.scuttlebutt.nz/datamodel.html#signing-encoding-objects
    fn serialize_newtype_variant<T: ?Sized>(self,
                                            _name: &'static str,
                                            _variant_index: u32,
                                            variant: &'static str,
                                            value: &T)
                                            -> Result<Self::Ok, Self::Error>
        where T: Serialize
    {
        self.write_len(1, LenMajor::Map)?;
        variant.serialize(&mut *self)?;
        value.serialize(&mut *self)
    }

    // https://spec.scuttlebutt.nz/datamodel.html#signing-encoding-arrays
    fn serialize_seq(self, len_: Option<usize>) -> Result<Self::SerializeSeq, EncodeCborError> {
        match len_ {
            None => return Err(EncodeCborError::UnknownLength),
            Some(len) => {
                self.write_len(len, LenMajor::Array)?;
                Ok(CollectionSerializer::new(&mut *self))
            }
        }
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, EncodeCborError> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(self,
                              _name: &'static str,
                              len: usize)
                              -> Result<Self::SerializeTupleStruct, EncodeCborError> {
        self.serialize_seq(Some(len))
    }

    // https://spec.scuttlebutt.nz/datamodel.html#signing-encoding-objects
    // https://spec.scuttlebutt.nz/datamodel.html#signing-encoding-arrays
    fn serialize_tuple_variant(self,
                               _name: &'static str,
                               _variant_index: u32,
                               variant: &'static str,
                               len: usize)
                               -> Result<Self::SerializeTupleVariant, EncodeCborError> {
        self.write_len(1, LenMajor::Map)?;
        variant.serialize(&mut *self)?;
        self.write_len(len, LenMajor::Array)?;
        Ok(CollectionSerializer::new(&mut *self))
    }

    // https://spec.scuttlebutt.nz/datamodel.html#signing-encoding-objects
    fn serialize_map(self, len_: Option<usize>) -> Result<Self::SerializeMap, EncodeCborError> {
        match len_ {
            None => return Err(EncodeCborError::UnknownLength),
            Some(len) => {
                self.write_len(len, LenMajor::Map)?;
                Ok(CollectionSerializer::new(&mut *self))
            }
        }
    }

    fn serialize_struct(self,
                        _name: &'static str,
                        len: usize)
                        -> Result<Self::SerializeStruct, EncodeCborError> {
        self.serialize_map(Some(len))
    }

    // https://spec.scuttlebutt.nz/datamodel.html#signing-encoding-objects
    fn serialize_struct_variant(self,
                                _name: &'static str,
                                _variant_index: u32,
                                variant: &'static str,
                                len: usize)
                                -> Result<Self::SerializeStructVariant, EncodeCborError> {
        self.write_len(1, LenMajor::Map)?;
        variant.serialize(&mut *self)?;
        self.write_len(len, LenMajor::Map)?;
        Ok(CollectionSerializer::new(&mut *self))
    }
}

#[doc(hidden)]
pub struct CollectionSerializer<'a, W> {
    ser: &'a mut CborSerializer<W>,
}

impl<'a, W: io::Write> CollectionSerializer<'a, W> {
    fn new(ser: &'a mut CborSerializer<W>) -> CollectionSerializer<'a, W> {
        CollectionSerializer { ser }
    }
}

impl<'a, W> SerializeSeq for CollectionSerializer<'a, W>
    where W: io::Write
{
    type Ok = ();
    type Error = EncodeCborError;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
        where T: Serialize
    {
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, W> SerializeTuple for CollectionSerializer<'a, W>
    where W: io::Write
{
    type Ok = ();
    type Error = EncodeCborError;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
        where T: Serialize
    {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeSeq::end(self)
    }
}

impl<'a, W> SerializeTupleStruct for CollectionSerializer<'a, W>
    where W: io::Write
{
    type Ok = ();
    type Error = EncodeCborError;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
        where T: Serialize
    {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeSeq::end(self)
    }
}

impl<'a, W> SerializeTupleVariant for CollectionSerializer<'a, W>
    where W: io::Write
{
    type Ok = ();
    type Error = EncodeCborError;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
        where T: Serialize
    {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, W> SerializeMap for CollectionSerializer<'a, W>
    where W: io::Write
{
    type Ok = ();
    type Error = EncodeCborError;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
        where T: Serialize
    {
        key.serialize(&mut *self.ser)
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
        where T: Serialize
    {
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, W> SerializeStruct for CollectionSerializer<'a, W>
    where W: io::Write
{
    type Ok = ();
    type Error = EncodeCborError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), EncodeCborError>
        where T: ?Sized + Serialize
    {
        SerializeMap::serialize_entry(self, key, value)
    }

    fn end(self) -> Result<(), EncodeCborError> {
        SerializeMap::end(self)
    }
}

impl<'a, W> SerializeStructVariant for CollectionSerializer<'a, W>
    where W: io::Write
{
    type Ok = ();
    type Error = EncodeCborError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), EncodeCborError>
        where T: ?Sized + Serialize
    {
        SerializeMap::serialize_entry(self, key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}
