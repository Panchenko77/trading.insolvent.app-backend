use serde::de::Error;
use std::fmt;

pub struct CowStrVisitor;
impl<'de> serde::de::Visitor<'de> for CowStrVisitor {
    type Value = std::borrow::Cow<'de, str>;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(std::borrow::Cow::Borrowed(v))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(std::borrow::Cow::Owned(v))
    }
}
