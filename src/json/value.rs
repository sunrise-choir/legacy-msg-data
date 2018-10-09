use std::collections::{HashMap, BTreeMap};

use indexmap::IndexMap;

use super::super::abstract_::{
    LegacyF64,
    serialize::Serialize,
    serializer::{
        Serializer,
        SerializeArray,
        SerializeObject,
    },
    deserialize::Deserialize,
    deserializer::{
        Deserializer,
        Visitor,
        ArrayAccess,
        ObjectAccess,
        ObjectAccessState,
    }
};

// The maximum capacity of entries to preallocate for arrays and objects. Even if malicious input
// claims to contain a much larger collection, only this much memory will be blindly allocated.
static MAX_ALLOC: usize = 2048;

/// Represents any valid ssb legacy message value, analogous to [serde_json::Value](https://docs.serde.rs/serde_json/value/enum.Value.html).
#[derive(PartialEq, Eq, Debug, Clone)]
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

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}

struct ValueVisitor;

impl<'a, V> ObjectAccessState for &'a HashMap<String, V> {
    fn has_key(self, key: &str) -> bool {
        self.contains_key(key)
    }
}

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(v))
    }

    fn visit_f64<E>(self, v: LegacyF64) -> Result<Self::Value, E> {
        Ok(Value::Float(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
        self.visit_string(v.to_string())
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E> {
        Ok(Value::String(v))
    }

    fn visit_null<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_array<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: ArrayAccess<'de> {
        // use the size hint, but put a maximum to the allocation because we can't trust the input
        let mut v = Vec::with_capacity(std::cmp::min(seq.size_hint().unwrap_or(0), MAX_ALLOC));

        while let Some(inner) = seq.next_element()? {
            v.push(inner);
        }

        Ok(Value::Array(v))
    }

    fn visit_object<A>(self, mut object: A) -> Result<Self::Value, A::Error> where A: ObjectAccess<'de> {
        // use the size hint, but put a maximum to the allocation because we can't trust the input
        let mut m = HashMap::with_capacity(std::cmp::min(object.size_hint().unwrap_or(0), MAX_ALLOC));


        while let Some((key, val)) = object.next_entry_seed(&m, std::marker::PhantomData)? {
            let _ = m.insert(key, val);
        }

        Ok(Value::Object(m))
    }
}

/// Represents any valid ssb legacy message value, preserving the order of object entries. Prefer
/// using `Value` instead of this, this should only be used for checking message signatures.
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum ValueOrdered {
    Null,
    Bool(bool),
    Float(LegacyF64),
    String(String),
    Array(Vec<ValueOrdered>),
    Object {
        naturals: BTreeMap<String, ValueOrdered>,
        others: IndexMap<String, ValueOrdered>
    },
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
            ValueOrdered::Object {ref naturals, ref others } => {
                let mut s = serializer.serialize_object(naturals.len() + others.len())?;
                for (key, value) in naturals {
                    s.serialize_entry(key, value)?;
                }
                for (key, value) in others {
                    s.serialize_entry(key, value)?;
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

impl<'a, V> ObjectAccessState for (&'a BTreeMap<String, V>, &'a IndexMap<String, V>) {
    fn has_key(self, key: &str) -> bool {
        self.1.contains_key(key) || self.0.contains_key(key)
    }
}

impl<'de> Visitor<'de> for ValueOrderedVisitor {
    type Value = ValueOrdered;

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E> {
        Ok(ValueOrdered::Bool(v))
    }

    fn visit_f64<E>(self, v: LegacyF64) -> Result<Self::Value, E> {
        Ok(ValueOrdered::Float(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
        self.visit_string(v.to_string())
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E> {
        Ok(ValueOrdered::String(v))
    }

    fn visit_null<E>(self) -> Result<Self::Value, E> {
        Ok(ValueOrdered::Null)
    }

    fn visit_array<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: ArrayAccess<'de> {
        // use the size hint, but put a maximum to the allocation because we can't trust the input
        let mut v = Vec::with_capacity(std::cmp::min(seq.size_hint().unwrap_or(0), MAX_ALLOC));

        while let Some(inner) = seq.next_element()? {
            v.push(inner);
        }

        Ok(ValueOrdered::Array(v))
    }

    fn visit_object<A>(self, mut object: A) -> Result<Self::Value, A::Error> where A: ObjectAccess<'de> {
        // use the size hint, but put a maximum to the allocation because we can't trust the input
        let mut naturals = BTreeMap::new();
        let mut others = IndexMap::with_capacity(std::cmp::min(object.size_hint().unwrap_or(0), MAX_ALLOC));


        while let Some((key, val)) = object.next_entry_seed((&naturals, &others), std::marker::PhantomData)? {
            if key == "0" {
                let _ = naturals.insert(key, val);
            } else {
                if is_nat_str(&key) {
                    let _ = naturals.insert(key, val);
                } else {
                    let _ = others.insert(key, val);
                }
            }
        }

        Ok(ValueOrdered::Object { naturals, others })
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
