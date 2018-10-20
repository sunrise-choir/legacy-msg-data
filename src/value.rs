// Data structures for manipulating arbitrary legacy data.

use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, btree_map};
use std::fmt;

use indexmap::{IndexMap, map};
use serde::{
    ser::{Serialize, Serializer, SerializeSeq, SerializeMap},
    de::{Deserialize, Deserializer, Visitor, SeqAccess, MapAccess, Error},
};

use super::LegacyF64;

// The maximum capacity of entries to preallocate for arrays and objects. Even if malicious input
// claims to contain a much larger collection, only this much memory will be blindly allocated.
static MAX_ALLOC: usize = 2048;

/// Represents any valid ssb legacy message [value](https://spec.scuttlebutt.nz/datamodel.html#abstract-data-model), analogous to [serde_json::Value](https://docs.serde.rs/serde_json/value/enum.Value.html).
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Value {
    /// The [null](TODO) value.
    Null,
    /// A [boolean](TODO).
    Bool(bool),
    /// A [float](TODO).
    Float(LegacyF64),
    /// A [utf8 string](TODO).
    String(String),
    /// An [array](TODO).
    Array(Vec<Value>),
    /// An [object](TODO).
    Object(HashMap<String, Value>),
}

impl Serialize for Value {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        match *self {
            Value::Null => serializer.serialize_unit(),
            Value::Bool(b) => serializer.serialize_bool(b),
            Value::Float(f) => serializer.serialize_f64(f.into()),
            Value::String(ref s) => serializer.serialize_str(s),
            Value::Array(ref v) => {
                let mut s = serializer.serialize_seq(Some(v.len()))?;
                for inner in v {
                    s.serialize_element(inner)?;
                }
                s.end()
            }
            Value::Object(ref m) => {
                let mut s = serializer.serialize_map(Some(m.len()))?;
                for (key, value) in m {
                    s.serialize_entry(key, value)?;
                }
                s.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Value, D::Error>
        where D: Deserializer<'de>
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("any valid legacy ssb value")
            }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(v))
    }

    fn visit_f64<E: Error>(self, v: f64) -> Result<Self::Value, E> {
        match LegacyF64::from_f64(v) {
            Some(f) => Ok(Value::Float(f)),
            None => Err(E::custom("invalid float"))
        }
    }

    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E>
    {
        self.visit_string(v.to_string())
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E> {
        Ok(Value::String(v))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where A: SeqAccess<'de>
    {
        // use the size hint, but put a maximum to the allocation because we can't trust the input
        let mut v = Vec::with_capacity(std::cmp::min(seq.size_hint().unwrap_or(0), MAX_ALLOC));

        while let Some(inner) = seq.next_element()? {
            v.push(inner);
        }

        Ok(Value::Array(v))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where A: MapAccess<'de>
    {
        // use the size hint, but put a maximum to the allocation because we can't trust the input
        let mut m = HashMap::with_capacity(std::cmp::min(map.size_hint().unwrap_or(0),
                                                         MAX_ALLOC));

        while let Some((key, val)) = map.next_entry()? {
            if let Some(_) = m.insert(key, val) {
                return Err(A::Error::custom("map had duplicate key"));
            }
        }

        Ok(Value::Object(m))
    }
}

//////////////////////////////////////////////////////////////////////////////////////////////

/// Represents any valid ssb legacy message value, preserving the order of object entries. Prefer
/// using `Value` instead of this, this should only be used for checking message signatures.
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum ValueOrdered {
    /// The [null](TODO) value.
    Null,
    /// A [boolean](TODO).
    Bool(bool),
    /// A [float](TODO).
    Float(LegacyF64),
    /// A [utf8 string](TODO).
    String(String),
    /// An [array](TODO).
    Array(Vec<ValueOrdered>),
    /// An order-preserving [object](TODO).
    Object(RidiculousStringMap<ValueOrdered>),
}

impl Serialize for ValueOrdered {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            ValueOrdered::Null => serializer.serialize_unit(),
            ValueOrdered::Bool(b) => serializer.serialize_bool(b),
            ValueOrdered::Float(f) => serializer.serialize_f64(f.into()),
            ValueOrdered::String(ref s) => serializer.serialize_str(s),
            ValueOrdered::Array(ref v) => {
                let mut s = serializer.serialize_seq(Some(v.len()))?;
                for inner in v {
                    s.serialize_element(inner)?;
                }
                s.end()
            },
            ValueOrdered::Object(ref m) => {
                let mut s = serializer.serialize_map(Some(m.len()))?;
                for (key, value) in m {
                    s.serialize_entry(&key, value)?;
                }
                s.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for ValueOrdered {
    fn deserialize<D>(deserializer: D) -> Result<ValueOrdered, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueOrderedVisitor)
    }
}

struct ValueOrderedVisitor;

impl<'de> Visitor<'de> for ValueOrderedVisitor {
    type Value = ValueOrdered;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("any valid legacy ssb value")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E> {
        Ok(ValueOrdered::Bool(v))
    }

    fn visit_f64<E: Error>(self, v: f64) -> Result<Self::Value, E> {
        match LegacyF64::from_f64(v) {
            Some(f) => Ok(ValueOrdered::Float(f)),
            None => Err(E::custom("invalid float"))
        }
    }

    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        self.visit_string(v.to_string())
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E> {
        Ok(ValueOrdered::String(v))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(ValueOrdered::Null)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de> {
        // use the size hint, but put a maximum to the allocation because we can't trust the input
        let mut v = Vec::with_capacity(std::cmp::min(seq.size_hint().unwrap_or(0), MAX_ALLOC));

        while let Some(inner) = seq.next_element()? {
            v.push(inner);
        }

        Ok(ValueOrdered::Array(v))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: MapAccess<'de> {
        // use the size hint, but put a maximum to the allocation because we can't trust the input

        // use the size hint, but put a maximum to the allocation because we can't trust the input
        let mut m = RidiculousStringMap::with_capacity(std::cmp::min(map.size_hint().unwrap_or(0),
                                                         MAX_ALLOC));

        while let Some((key, val)) = map.next_entry()? {
            if let Some(_) = m.insert(key, val) {
                return Err(A::Error::custom("map had duplicate key"));
            }
        }

        Ok(ValueOrdered::Object(m))
    }
}

fn is_nat_str(s: &str) -> bool {
    match s.as_bytes().split_first() {
        Some((0x31...0x39, tail)) => {
            if tail.iter().all(|byte| *byte >= 0x30 && *byte <= 0x39) {
                true
            } else {
                false
            }
        }
        _ => {
            false
        },
    }
}

// A map with string keys that sorts strings that look like natural numbers by numeric
// value, and preserves insertion order for everything else.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct RidiculousStringMap<V> {
    /// Keys that parse as natural numbers, sorted numerically.
    ///
    /// From the spec:
    ///
    /// > - if there is an entry with the key `"0"` (`0x30`), the entry must be the first to be serialized
    /// >   - all entries whose keys begin with a nonzero decimal digit (1 - 9 (`0x31` - `0x39`)) followed by zero or more arbitrary decimal digits (0 - 9 (`0x30` - `0x39`)) and consists solely of such digits, must be serialized before all other entries (but after an entry with key `"0"` if one exists). Amongst themselves, these keys are sorted:
    /// >   - by length first (ascending), using
    /// >   - numeric value as a tie breaker (the key whose raw bytes interpreted as a natural number are larger is serialized later)
    /// >     - note that this coincides with sorting the decimally encoded numbers by numeric value
    naturals: BTreeMap<GraphicolexicalString, V>,
    /// The remaining keys, sorted in insertion order.
    others: IndexMap<String, V>,
}

impl<V> RidiculousStringMap<V> {
    pub fn with_capacity(capacity: usize) -> RidiculousStringMap<V> {
        RidiculousStringMap {
            naturals: BTreeMap::new(),
            others: IndexMap::with_capacity(capacity),
        }
    }

    pub fn len(&self) -> usize {
        self.naturals.len() + self.others.len()
    }

    pub fn insert(&mut self, key: String, val: V) -> Option<V> {
        if key == "0" {
            self.naturals.insert(GraphicolexicalString(key), val)
        } else {
            if is_nat_str(&key) {
                self.naturals.insert(GraphicolexicalString(key), val)
            } else {
                self.others.insert(key, val)
            }
        }
    }

    pub fn iter(&self) -> Iter<V> {
        Iter { naturals: self.naturals.iter(), others: self.others.iter(), nats: true }
    }
}

impl<'a, V> IntoIterator for &'a RidiculousStringMap<V> {
    type Item = (&'a String, &'a V);
    type IntoIter = Iter<'a, V>;

    fn into_iter(self) -> Iter<'a, V> {
        self.iter()
    }
}

pub struct Iter<'a, V> {
    naturals: btree_map::Iter<'a, GraphicolexicalString, V>,
    others: map::Iter<'a, String, V>,
    nats: bool,
}

impl<'a, V> Iterator for Iter<'a, V> {
    type Item = (&'a String, &'a V);

    fn next(&mut self) -> Option<(&'a String, &'a V)> {
        if self.nats {
            match self.naturals.next() {
                None => {
                    self.nats = false;
                    self.next()
                }
                Some((key, val)) => Some((&key.0, val)),
            }
        } else {
            self.others.next()
        }
    }
}

/// A wrapper around String, that compares by length first and uses lexicographical order as a
/// tie-breaker.
#[derive(PartialEq, Eq, Clone, Hash)]
pub struct GraphicolexicalString(String);

impl PartialOrd for GraphicolexicalString {
    fn partial_cmp(&self, other: &GraphicolexicalString) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GraphicolexicalString {
    fn cmp(&self, other: &GraphicolexicalString) -> Ordering {
        match self.0.len().cmp(&other.0.len()) {
            Ordering::Greater => Ordering::Greater,
            Ordering::Less => Ordering::Less,
            Ordering::Equal => self.0.cmp(&other.0),
        }
    }
}

impl fmt::Debug for GraphicolexicalString {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}

impl Borrow<str> for GraphicolexicalString {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl From<String> for GraphicolexicalString {
    fn from(s: String) -> Self {
        GraphicolexicalString(s)
    }
}

impl From<GraphicolexicalString> for String {
    fn from(s: GraphicolexicalString) -> Self {
        s.0
    }
}
