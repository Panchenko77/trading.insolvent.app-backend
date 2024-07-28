use serde::de::{Error, MapAccess};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize};
use std::marker::PhantomData;

/// A wrapper for optional values that treat {} as None and vise versa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Emptiable<T>(Option<T>);
#[allow(dead_code)]
impl<T> Emptiable<T> {
    pub fn new(s: Option<T>) -> Self {
        Self(s)
    }
    pub fn as_ref(&self) -> Option<&T> {
        self.0.as_ref()
    }
    pub fn as_mut(&mut self) -> Option<&mut T> {
        self.0.as_mut()
    }
    pub fn into_option(self) -> Option<T> {
        self.0
    }
}
impl<T: Serialize> Serialize for Emptiable<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match &self.0 {
            Some(s) => s.serialize(serializer),
            None => {
                let map = serializer.serialize_map(Some(0))?;
                map.end()
            }
        }
    }
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Emptiable<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct OptionalVisitor<T>(PhantomData<T>);
        impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for OptionalVisitor<T> {
            type Value = Emptiable<T>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an optional value")
            }
            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(Emptiable(None))
            }
            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                let t = T::deserialize(deserializer)
                    .map(Some)
                    .map(Emptiable)
                    .map_err(Error::custom)?;
                Ok(t)
            }
            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                if map.size_hint() == Some(0) {
                    return Ok(Emptiable(None));
                }
                let deserializer = serde::de::value::MapAccessDeserializer::new(map);
                self.visit_some(deserializer)
            }
        }

        deserializer.deserialize_any(OptionalVisitor::<T>(PhantomData))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn test_deserialize_emptiable() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Foo {
            foo: String,
        }
        macro_rules! assert_eq_emptiable {
            ($value:expr, $expected:expr) => {
                assert_eq!(
                    serde_json::from_value::<Emptiable<Foo>>($value)
                        .unwrap()
                        .into_option(),
                    $expected
                );
            };
        }
        assert_eq_emptiable!(json!({}), None);
        assert_eq_emptiable!(
            json!({"foo": "foo"}),
            Some(Foo {
                foo: "foo".to_string()
            })
        );
    }
    #[test]
    fn test_serialize_emptiable() {
        #[derive(Debug, Serialize, PartialEq)]
        struct Foo {
            foo: String,
        }
        macro_rules! assert_eq_emptiable {
            ($value:expr, $expected:expr) => {
                assert_eq!(serde_json::to_value(Emptiable($value)).unwrap(), $expected);
            };
        }
        assert_eq_emptiable!(None::<Foo>, serde_json::json!({}));
        assert_eq_emptiable!(
            Some(Foo {
                foo: "foo".to_string()
            }),
            serde_json::json!({"foo": "foo"})
        );
    }
}
