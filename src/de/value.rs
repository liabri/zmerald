use std::fmt;

use serde::{
    de::{Error, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::de;
use crate::value::{ Map, Number, Value };

impl std::str::FromStr for Value {
    type Err = de::Error;

    fn from_str(s: &str) -> de::Result<Self> {
        let mut de = super::Deserializer::from_str(s)?;

        let val = Value::deserialize(&mut de)?;
        de.end()?;

        Ok(val)
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        deserializer.deserialize_any(ValueVisitor)
    }
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a zmerald value")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where E: Error {
        Ok(Value::Bool(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where E: Error {
        Ok(Value::Number(Number::new(v)))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where E: Error {
        Ok(Value::Number(Number::new(v)))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where E: Error {
        Ok(Value::Number(Number::new(v)))
    }

    fn visit_char<E>(self, v: char) -> Result<Self::Value, E>
    where E: Error {
        Ok(Value::Char(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where E: Error {
        self.visit_string(v.to_owned())
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where E: Error {
        Ok(Value::String(v))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where E: Error {
        self.visit_byte_buf(v.to_vec())
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where E: Error {
        self.visit_string(String::from_utf8(v).map_err(|e| Error::custom(format!("{}", e)))?)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where E: Error {
        Ok(Value::Option(None))
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where D: Deserializer<'de> {
        Ok(Value::Option(Some(Box::new(
            deserializer.deserialize_any(ValueVisitor)?,
        ))))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where E: Error {
        Ok(Value::Unit)
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where D: Deserializer<'de> {
        deserializer.deserialize_any(ValueVisitor)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de> {
        let mut vec = Vec::new();
        if let Some(cap) = seq.size_hint() {
            vec.reserve_exact(cap);
        }

        while let Some(x) = seq.next_element()? {
            vec.push(x);
        }

        Ok(Value::Seq(vec))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: MapAccess<'de> {
        let mut res: Map = Map::new();

        while let Some(entry) = map.next_entry()? {
            res.insert(entry.0, entry.1);
        }

        Ok(Value::Map(res))
    }
}