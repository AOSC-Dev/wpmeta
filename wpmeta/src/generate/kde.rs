use eyre::{eyre, Result};
use serde::ser::{SerializeMap, Serializer};
use serde::Serialize;

use std::collections::HashMap;

use localized::Localized;

use crate::meta::{Author, Metadata};

#[derive(Clone, Debug)]
pub struct KPluginName<'a> {
    inner: &'a Localized<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct KPluginAuthor<'a> {
    email: &'a str,
    #[serde(flatten)]
    name: KPluginName<'a>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct KPluginMetadataInner<'a> {
    authors: Vec<KPluginAuthor<'a>>,
    id: &'a str,
    license: &'a str,
    #[serde(flatten)]
    name: KPluginName<'a>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct KPluginMetadata<'a> {
    k_plugin: KPluginMetadataInner<'a>,
}

impl<'a> Serialize for KPluginName<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.inner.len()))?;
        if let Some(default) = self.inner.get_default() {
            map.serialize_entry("Name", default)?;
        }
        let flattened = self.inner.to_hashmap(|l| l.to_string());
        if let Ok(names) = flattened {
            for (locale, name) in names {
                map.serialize_entry(&format!("Name[{}]", locale.replace('-', "_")), name)?;
            }
        }
        map.end()
    }
}

impl<'a> From<&'a Localized<String>> for KPluginName<'a> {
    fn from(value: &'a Localized<String>) -> Self {
        Self { inner: value }
    }
}

impl<'a> From<&'a Author> for KPluginAuthor<'a> {
    fn from(value: &'a Author) -> Self {
        Self {
            email: value.email(),
            name: value.name().into(),
        }
    }
}

impl<'a> KPluginMetadataInner<'a> {
    pub fn new(authors: Vec<KPluginAuthor<'a>>, id: &'a str, license: &'a str, name: KPluginName<'a>) -> Self {
        Self {
            authors,
            id,
            license,
            name,
        }
    }
}

impl<'a> KPluginMetadata<'a> {
    pub fn from_metadata(src: &'a Metadata) -> Result<HashMap<&'a str, Self>> {
        let authors = match src.authors() {
            Some(authors) => authors.iter().map(KPluginAuthor::from).collect(),
            None => Vec::new(),
        };
        let wallpapers = src
            .wallpapers()
            .ok_or_else(|| eyre!("Failed to get wallpaper list"))?;
        Ok(wallpapers
            .iter()
            .map(|w| {
                (
                    w.id(),
                    Self {
                        k_plugin: KPluginMetadataInner::new(authors.clone(), w.id(), w.license(), w.titles().into())
                    },
                )
            })
            .collect())
    }
}

pub fn render_kde(metadata: &Metadata) -> Result<HashMap<&str, String>> {
    Ok(KPluginMetadata::from_metadata(metadata)?
        .into_iter()
        .map(|(k, v)| {
            (
                k,
                serde_json::to_string_pretty(&v).expect("Unable to serialize KPlugin Metadata"),
            )
        })
        .collect())
}

#[cfg(test)]
mod test {
    use super::render_kde;
    use crate::meta::Metadata;

    #[test]
    fn test_render() {
        let dummy_meta = toml::from_str::<Metadata>(crate::meta::test::DUMMY_META).unwrap();
        let result = render_kde(&dummy_meta).unwrap();
        assert_eq!(
            result.get("Kusa").unwrap(),
            r#"{
  "KPlugin": {
    "Authors": [
      {
        "Email": "yajuu.senpai@example.com",
        "Name": "Yajuu Senpai",
        "Name[zh_CN]": "野兽先辈"
      }
    ],
    "Id": "Kusa",
    "License": "CC BY-SA 4.0",
    "Name": "Kusa",
    "Name[en_US]": "Grass"
  }
}"#
        );
    }
}
