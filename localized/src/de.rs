use serde::de::{Error, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};

use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Formatter;

pub use crate::{Locale, Localized};

impl<'de> Deserialize<'de> for Locale {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct LocaleVisitor {
            marker: std::marker::PhantomData<fn() -> Locale>,
        }

        impl<'de> Visitor<'de> for LocaleVisitor {
            type Value = Locale;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("Locale string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(Self::Value::new(v))
            }
        }

        deserializer.deserialize_str(LocaleVisitor {
            marker: std::marker::PhantomData,
        })
    }
}

impl<'de, T> Deserialize<'de> for Localized<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct LocalizedVisitor<T> {
            marker: std::marker::PhantomData<T>,
        }

        impl<'de, T> Visitor<'de> for LocalizedVisitor<T>
        where
            T: Deserialize<'de>,
        {
            type Value = Localized<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("Tagged localized data")
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut default = None;
                // False positive, the hash function won't read the mutable fields
                #[allow(clippy::mutable_key_type)]
                let mut content = BTreeMap::new();
                while let Some((k, v)) = map.next_entry::<String, T>()? {
                    if k.to_lowercase() == "default" {
                        default = Some(v);
                        continue;
                    }
                    content.insert(Locale::new(k), v);
                }

                Ok(Self::Value { default, content })
            }
        }

        deserializer.deserialize_map(LocalizedVisitor {
            marker: std::marker::PhantomData,
        })
    }
}
