//! Streaming JSON preflight: every array element is charged before buffering it.
use super::CatalogImportError as Error;
use serde::de::{self, DeserializeSeed, IgnoredAny, MapAccess, SeqAccess, Visitor};
use std::fmt;

pub(super) fn json_entries(input: &[u8], remaining: &mut usize) -> Result<(), Error> {
    let mut deserializer = serde_json::Deserializer::from_slice(input);
    Scan {
        depth: 0,
        remaining,
    }
    .deserialize(&mut deserializer)
    .and_then(|()| deserializer.end())
    .map_err(|e| Error::Document {
        line: e.line(),
        column: e.column(),
    })
}
struct Scan<'a> {
    depth: usize,
    remaining: &'a mut usize,
}
impl<'de> DeserializeSeed<'de> for Scan<'_> {
    type Value = ();
    fn deserialize<D: de::Deserializer<'de>>(self, d: D) -> Result<(), D::Error> {
        d.deserialize_any(self)
    }
}
struct Element<'a>(Scan<'a>);
impl<'de> DeserializeSeed<'de> for Element<'_> {
    type Value = ();
    fn deserialize<D: de::Deserializer<'de>>(self, d: D) -> Result<(), D::Error> {
        *self.0.remaining = self
            .0
            .remaining
            .checked_sub(1)
            .ok_or_else(|| de::Error::custom("catalog entry limit"))?;
        self.0.deserialize(d)
    }
}
impl<'de> Visitor<'de> for Scan<'_> {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("bounded catalog JSON")
    }
    fn visit_bool<E: de::Error>(self, _: bool) -> Result<(), E> {
        Ok(())
    }
    fn visit_i64<E: de::Error>(self, _: i64) -> Result<(), E> {
        Ok(())
    }
    fn visit_u64<E: de::Error>(self, _: u64) -> Result<(), E> {
        Ok(())
    }
    fn visit_f64<E: de::Error>(self, _: f64) -> Result<(), E> {
        Ok(())
    }
    fn visit_str<E: de::Error>(self, _: &str) -> Result<(), E> {
        Ok(())
    }
    fn visit_unit<E: de::Error>(self) -> Result<(), E> {
        Ok(())
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<(), A::Error> {
        if self.depth >= crate::schema::MAX_DEPTH {
            return Err(de::Error::custom("catalog depth limit"));
        }
        while a
            .next_element_seed(Element(Scan {
                depth: self.depth + 1,
                remaining: self.remaining,
            }))?
            .is_some()
        {}
        Ok(())
    }
    fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<(), A::Error> {
        if self.depth >= crate::schema::MAX_DEPTH {
            return Err(de::Error::custom("catalog depth limit"));
        }
        while a.next_key::<IgnoredAny>()?.is_some() {
            a.next_value_seed(Scan {
                depth: self.depth + 1,
                remaining: self.remaining,
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use droutex_policy::MAX_POLICY_ENTRIES;

    #[test]
    fn streaming_scan_checks_exact_depth_and_shared_array_budget_without_buffering() {
        let depth = crate::schema::MAX_DEPTH;
        let valid = format!("{}true{}", "[".repeat(depth), "]".repeat(depth));
        let mut remaining = MAX_POLICY_ENTRIES;
        json_entries(valid.as_bytes(), &mut remaining).unwrap();
        let invalid = format!("[{valid}]");
        let mut remaining = MAX_POLICY_ENTRIES;
        assert!(json_entries(invalid.as_bytes(), &mut remaining).is_err());
        let mut remaining = 3;
        json_entries(br#"{"entries":[1,2],"dependencies":[3]}"#, &mut remaining).unwrap();
        assert_eq!(remaining, 0);
        assert!(json_entries(b"[4]", &mut remaining).is_err());
    }
}
