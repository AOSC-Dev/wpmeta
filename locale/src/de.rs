use serde::de::{MapAccess, Visitor};
use serde::Deserialize;

use std::collections::HashMap;
use std::fmt;

pub use crate::{Locale, Localized};

impl<'de, T> Deserialize<'de> for Localized<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
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
                let mut content = HashMap::new();
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

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use super::{Locale, Localized};

    #[test]
    fn test_de() {
        let example = r#"
        default = "Kusa"
        en-US = "Grass"
        zh-CN = "草"
        "#;

        let de_result =
            toml::from_str::<Localized<String>>(example).expect("Unable to deserialize");
        assert_eq!(
            Localized::<String> {
                default: Some("Kusa".into()),
                content: HashMap::from([
                    (Locale::new("zh-CN"), "草".into()),
                    (Locale::new("en-US"), "Grass".into()),
                ]),
            },
            de_result
        );
    }
}
