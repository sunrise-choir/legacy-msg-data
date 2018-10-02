use std::collections::HashMap;

use indexmap::IndexMap;

use super::super::abstract_::LegacyF64;

/// Represents any valid ssb legacy message value, analogous to [serde_json::Value](https://docs.serde.rs/serde_json/value/enum.Value.html).
pub enum Value {
    Null,
    Bool(bool),
    Float(LegacyF64),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
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
