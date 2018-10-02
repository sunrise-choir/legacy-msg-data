use std::collections::HashMap;

use indexmap::IndexMap;

use super::super::abstract_::{
    LegacyF64,
    serialize::Serialize,
    serializer::{
        Serializer,
        SerializeArray,
        SerializeObject,
    }
};

/// Represents any valid ssb legacy message value, analogous to [serde_json::Value](https://docs.serde.rs/serde_json/value/enum.Value.html).
pub enum Value {
    Null,
    Bool(bool),
    Float(LegacyF64),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

impl Serialize for Value {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            Value::Null => serializer.serialize_null(),
            Value::Bool(b) => serializer.serialize_bool(b),
            Value::Float(f) => serializer.serialize_f64(f),
            Value::String(ref s) => serializer.serialize_str(s),
            Value::Array(ref v) => {
                let mut s = serializer.serialize_array(v.len())?;
                for inner in v {
                    s.serialize_element(inner)?;
                }
                s.end()
            },
            Value::Object(ref m) => {
                let mut s = serializer.serialize_object(m.len())?;
                for (key, value) in m {
                    s.serialize_entry(key, value)?;
                }
                s.end()
            }
        }
    }
}

/// Represents any valid ssb legacy message value, preserving the order of object entries. Prefer
/// using `Value` instead of this, this should only be used for checking message signatures.
pub enum ValueOrdered {
    Null,
    Bool(bool),
    Float(LegacyF64),
    String(String),
    Array(Vec<Value>),
    Object(IndexMap<String, Value>),
}

impl Serialize for ValueOrdered {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            ValueOrdered::Null => serializer.serialize_null(),
            ValueOrdered::Bool(b) => serializer.serialize_bool(b),
            ValueOrdered::Float(f) => serializer.serialize_f64(f),
            ValueOrdered::String(ref s) => serializer.serialize_str(s),
            ValueOrdered::Array(ref v) => {
                let mut s = serializer.serialize_array(v.len())?;
                for inner in v {
                    s.serialize_element(inner)?;
                }
                s.end()
            },
            ValueOrdered::Object(ref m) => {
                let mut s = serializer.serialize_object(m.len())?;
                for (key, value) in m {
                    s.serialize_entry(key, value)?;
                }
                s.end()
            }
        }
    }
}
