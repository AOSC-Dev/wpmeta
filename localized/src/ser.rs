use serde::ser::{Serialize, SerializeMap, Serializer};

use crate::{Locale, Localized};

impl Serialize for Locale {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<T> Serialize for Localized<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.len()))?;
        if let Some(def) = &self.default {
            map.serialize_entry("default", def)?;
        }
        for (k, v) in &self.content {
            map.serialize_entry(&k.to_string(), v)?;
        }
        map.end()
    }
}
