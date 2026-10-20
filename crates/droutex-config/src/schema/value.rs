use serde::de::{
    self, Deserialize, DeserializeSeed, IntoDeserializer, MapAccess, SeqAccess, Visitor,
};
use serde_json::{Map, Number, Value};
use std::{fmt, marker::PhantomData};

// Internally tagged enums use deserialize_any and accept positional arrays too.
pub(super) fn object<'de, D: de::Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<T, D::Error> {
    deserializer.deserialize_map(ObjectVisitor(PhantomData))
}

struct ObjectVisitor<T>(PhantomData<T>);

impl<'de, T: Deserialize<'de>> Visitor<'de> for ObjectVisitor<T> {
    type Value = T;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a configuration object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mapping: A) -> Result<T, A::Error> {
        T::deserialize(de::value::MapAccessDeserializer::new(mapping))
    }
}

struct Object<T>(T);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Object<T> {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        object(deserializer).map(Self)
    }
}

pub(super) fn objects<'de, D: de::Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Vec<T>, D::Error> {
    Vec::<Object<T>>::deserialize(deserializer)
        .map(|objects| objects.into_iter().map(|object| object.0).collect())
}

/// Bounded intermediate input, never exported or logged.
pub(crate) struct BoundedValue(pub Value);

impl<'de> Deserialize<'de> for BoundedValue {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Seed(0).deserialize(deserializer).map(Self)
    }
}

impl<'de> IntoDeserializer<'de, de::value::Error> for BoundedValue {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self {
        self
    }
}

// Value's default struct deserializer accepts positional arrays; this boundary
// requires objects recursively while reusing Serde's map/sequence adapters.
impl<'de> de::Deserializer<'de> for BoundedValue {
    type Error = de::value::Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.0 {
            Value::Null => visitor.visit_unit(),
            Value::Bool(value) => visitor.visit_bool(value),
            Value::String(value) => visitor.visit_string(value),
            Value::Number(value) => {
                if let Some(value) = value.as_i64() {
                    visitor.visit_i64(value)
                } else if let Some(value) = value.as_u64() {
                    visitor.visit_u64(value)
                } else {
                    visitor.visit_f64(
                        value
                            .as_f64()
                            .ok_or_else(|| de::Error::custom("invalid number"))?,
                    )
                }
            }
            Value::Array(values) => {
                de::value::SeqDeserializer::new(values.into_iter().map(BoundedValue))
                    .deserialize_any(visitor)
            }
            Value::Object(values) => de::value::MapDeserializer::new(
                values
                    .into_iter()
                    .map(|(key, value)| (key, BoundedValue(value))),
            )
            .deserialize_any(visitor),
        }
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        if !self.0.is_object() {
            return Err(de::Error::custom("expected configuration object"));
        }
        self.deserialize_any(visitor)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        if self.0.is_null() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        match self.0 {
            Value::String(value) => visitor.visit_enum(value.into_deserializer()),
            _ => Err(de::Error::custom("expected configuration enum name")),
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes byte_buf
        unit unit_struct seq tuple tuple_struct map identifier ignored_any
    }
}

struct Seed(usize);

impl<'de> DeserializeSeed<'de> for Seed {
    type Value = Value;
    fn deserialize<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        deserializer.deserialize_any(ValueVisitor(self.0))
    }
}

struct ValueVisitor(usize);

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a bounded configuration value")
    }
    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }
    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Value, E> {
        Ok(value.into())
    }
    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Value, E> {
        Ok(value.into())
    }
    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Value, E> {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("non-finite number"))
    }
    fn visit_str<E: de::Error>(self, value: &str) -> Result<Value, E> {
        Ok(value.into())
    }
    fn visit_string<E: de::Error>(self, value: String) -> Result<Value, E> {
        Ok(Value::String(value))
    }
    fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_none<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Value, A::Error> {
        if self.0 >= super::MAX_DEPTH {
            return Err(de::Error::custom("configuration depth limit"));
        }
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element_seed(Seed(self.0 + 1))? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut mapping: A) -> Result<Value, A::Error> {
        if self.0 >= super::MAX_DEPTH {
            return Err(de::Error::custom("configuration depth limit"));
        }
        let mut values = Map::new();
        while let Some(key) = mapping.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(de::Error::custom("duplicate configuration key"));
            }
            let value = mapping.next_value_seed(Seed(self.0 + 1))?;
            values.insert(key, value);
        }
        Ok(Value::Object(values))
    }
}
