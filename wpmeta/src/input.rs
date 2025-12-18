use hex_color::HexColor;
use serde::{Deserialize, Serialize};

use localized::Localized;

use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Author {
    pub email: String,
    pub name: Localized<String>,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum PictureOptions {
    None,
    #[default]
    Wallpaper,
    Centered,
    Scaled,
    Stretched,
    Zoom,
    Spanned,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ColorShadingType {
    Horizontal,
    Vertical,
    #[default]
    Solid,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum WallpaperPath {
    Single(PathBuf),
    Multiple(Vec<PathBuf>),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Wallpaper {
    pub id: String,
    pub path: WallpaperPath,
    pub title: Localized<String>,
    pub license: String,
    #[serde(default)]
    pub option: PictureOptions,
    #[serde(default)]
    pub shade_type: ColorShadingType,
    #[serde(default = "default_primary_color")]
    pub primary_color: HexColor,
    #[serde(default = "default_secondary_color")]
    pub secondary_color: HexColor,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Metadata {
    #[serde(default)]
    pub authors: Vec<Author>,
    #[serde(default)]
    pub wallpapers: Vec<Wallpaper>,
}

fn default_primary_color() -> HexColor {
    HexColor::rgb(2, 60, 136)
}

fn default_secondary_color() -> HexColor {
    HexColor::rgb(87, 137, 202)
}

impl Author {
    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn name(&self) -> &Localized<String> {
        &self.name
    }
}

impl WallpaperPath {
    pub fn get_paths(&self) -> Vec<&Path> {
        match self {
            WallpaperPath::Single(path) => vec![path],
            WallpaperPath::Multiple(paths) => paths.iter().map(|p| p.as_ref()).collect(),
        }
    }
}

impl Metadata {
    pub fn authors(&self) -> &[Author] {
        self.authors.as_ref()
    }

    pub fn wallpapers(&self) -> &[Wallpaper] {
        self.wallpapers.as_ref()
    }
}

#[cfg(test)]
pub mod test {
    use super::Metadata;

    pub static DUMMY_META_SINGLE_FILE: &str = r#"
    [[authors]]
    email = "yajuu.senpai@example.com"
    name.default = "Yajuu Senpai"
    name.zh-CN = "野兽先辈"

    [[wallpapers]]
    title.default = "Kusa"
    title.en-US = "Grass"
    license = "CC BY-SA 4.0"
    id = "Kusa"
    path = "test/example.jpg"
    "#;

    pub static DUMMY_META_MULTIPLE_FILE: &str = r#"
    [[authors]]
    email = "yajuu.senpai@example.com"
    name.default = "Yajuu Senpai"
    name.zh-CN = "野兽先辈"

    [[wallpapers]]
    title.default = "Kusa"
    title.en-US = "Grass"
    license = "CC BY-SA 4.0"
    id = "Kusa"
    path = [
        "test/example.jpg",
        "test/example-dark.jpg"
    ]
    "#;

    #[test]
    fn test_de() {
        let dummy_meta = toml::from_str::<Metadata>(DUMMY_META_SINGLE_FILE).unwrap();
        assert_eq!(dummy_meta.authors().len(), 1);
        assert_eq!(dummy_meta.wallpapers().len(), 1);
    }
}
