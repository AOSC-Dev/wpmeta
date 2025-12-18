//! KDE metadata generator.
//!
//! Produces a `metadata.json` compatible with KDE wallpaper plugins and optionally a
//! `contents/screenshot.jpg` preview.

use eyre::Result;
use log::info;
use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};

use super::{Author, MetadataGenerator, Resolution, Wallpaper, write_file};
use localized::Localized;
use std::path::Path;

#[derive(Clone, Debug)]
struct KPluginName<'a> {
    inner: &'a Localized<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct KPluginAuthor<'a> {
    email: &'a str,
    #[serde(flatten)]
    name: KPluginName<'a>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct KPluginMetadataInner<'a> {
    authors: Vec<KPluginAuthor<'a>>,
    id: &'a str,
    license: &'a str,
    #[serde(flatten)]
    name: KPluginName<'a>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct KPluginMetadata<'a> {
    k_plugin: KPluginMetadataInner<'a>,
}

/// Generates KDE wallpaper `metadata.json` for a single [`crate::generate::Wallpaper`].
#[derive(Copy, Clone, Debug)]
pub struct KDEMetadataGenerator;

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
    pub fn new(
        authors: Vec<KPluginAuthor<'a>>,
        id: &'a str,
        license: &'a str,
        name: KPluginName<'a>,
    ) -> Self {
        Self {
            authors,
            id,
            license,
            name,
        }
    }
}

impl<'a> KPluginMetadata<'a> {
    pub fn new(src: &'a Wallpaper<'a>) -> Result<Self> {
        let authors = src
            .authors
            .iter()
            .map(|a| KPluginAuthor::from(*a))
            .collect();
        Ok(Self {
            k_plugin: KPluginMetadataInner::new(
                authors,
                src.id,
                src.license.as_ref(),
                src.title.into(),
            ),
        })
    }
}

impl MetadataGenerator for KDEMetadataGenerator {
    fn generate_metadata(
        target_base: &Path,
        wallpaper: &Wallpaper,
        preview_resolution: Resolution,
    ) -> Result<()> {
        let id = wallpaper.id;
        let target_path = Self::get_wallpaper_base(target_base, id);
        let manifest_path = target_path.join("metadata.json");
        info!("{}: Generating manifest for KDE...", id);
        let metadata = serde_json::to_string_pretty(&KPluginMetadata::new(wallpaper)?)?;
        write_file(&manifest_path, metadata.as_bytes())?;
        if wallpaper.has_normal_wallpaper() && wallpaper.has_dark_wallpaper() {
            info!(
                "{}: Skipped generating preview - found both normal and dark wallpapers",
                id
            );
        } else {
            info!("{}: Generating preview ...", id);
            let preview_path = target_path.join("contents/screenshot.jpg");
            wallpaper.generate_preview(&preview_path, preview_resolution)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::borrow::Cow;
    use std::fs;

    use localized::Localized;

    use super::KDEMetadataGenerator;
    use crate::generate::test::{TempDir, localized_default_en_us, localized_default_zh_cn, wallpaper_file};
    use crate::generate::{MetadataGenerator, Resolution};
    use crate::generate::{Wallpaper, WallpaperKind};
    use crate::input::Author;

    #[test]
    fn test_writes_kde_metadata_json() {
        let tmp = TempDir::new("kde-metadata-json");
        let target_base = tmp.path();

        let author = Author {
            email: "yajuu.senpai@example.com".to_owned(),
            name: localized_default_zh_cn("Yajuu Senpai", "野兽先辈"),
        };
        let title: Localized<String> = localized_default_en_us("Kusa", "Grass");

        let normal_path =
            target_base.join("usr/share/wallpapers/Kusa/contents/images/7680x4320.jpg");
        let dark_path =
            target_base.join("usr/share/wallpapers/Kusa/contents/images_dark/7680x4320-dark.jpg");

        let wallpaper = Wallpaper {
            id: "Kusa",
            license: Cow::Borrowed("CC BY-SA 4.0"),
            authors: vec![&author],
            title: &title,
            files: vec![
                wallpaper_file(normal_path, WallpaperKind::Normal, 1, 1),
                wallpaper_file(dark_path, WallpaperKind::Dark, 1, 1),
            ],
            primary_color: hex_color::HexColor::rgb(2, 60, 136),
            secondary_color: hex_color::HexColor::rgb(87, 137, 202),
            color_shading_type: crate::input::ColorShadingType::Solid,
            options: crate::input::PictureOptions::Wallpaper,
        };

        KDEMetadataGenerator::generate_metadata(
            target_base,
            &wallpaper,
            Resolution { width: 500, height: 500 },
        )
        .unwrap();

        let manifest_path = target_base.join("usr/share/wallpapers/Kusa/metadata.json");
        let content = fs::read_to_string(&manifest_path).unwrap();
        let expected = r#"{
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
}"#;
        assert_eq!(content, expected);
    }
}
