//! Input metadata structures.
//!
//! This module defines the TOML schema consumed by `wpmeta`. `metadata.toml` files are parsed by
//! the directory walker (`crate::walk`) into these types.

use hex_color::HexColor;
use serde::{Deserialize, Serialize};

use localized::Localized;

use std::path::{Path, PathBuf};

/// A wallpaper author.
///
/// Authors can be defined at a directory level and inherited by subdirectories.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Author {
    /// Contact email address.
    pub email: String,
    /// Display name (optionally localized).
    pub name: Localized<String>,
}

/// How the wallpaper should be rendered by the desktop environment.
///
/// Serialized as lowercase strings (e.g. `"wallpaper"`, `"centered"`).
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum PictureOptions {
    /// Do not set an option.
    None,
    /// Default wallpaper mode.
    #[default]
    Wallpaper,
    /// Center the image.
    Centered,
    /// Scale the image.
    Scaled,
    /// Stretch the image.
    Stretched,
    /// Zoom to fill.
    Zoom,
    /// Span across displays.
    Spanned,
}

/// How primary/secondary colors are applied when used as a background fill.
///
/// Serialized as lowercase strings (e.g. `"solid"`, `"horizontal"`).
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ColorShadingType {
    /// Horizontal gradient.
    Horizontal,
    /// Vertical gradient.
    Vertical,
    /// Single solid color.
    #[default]
    Solid,
}

/// A wallpaper file path specification.
///
/// Supports either a single relative file path or a list of relative file paths.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum WallpaperPath {
    /// One wallpaper file.
    Single(PathBuf),
    /// Multiple wallpaper files (e.g. multiple resolutions, normal+dark variants).
    Multiple(Vec<PathBuf>),
}

/// A wallpaper entry as defined in `metadata.toml`.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Wallpaper {
    /// Stable identifier used for installation paths and generated manifests.
    pub id: String,
    /// Path(s) to wallpaper image files, relative to the `metadata.toml` directory.
    pub path: WallpaperPath,
    /// Wallpaper title (optionally localized).
    pub title: Localized<String>,
    /// SPDX license identifier or a free-form license string.
    pub license: String,
    /// Rendering option (desktop-environment specific).
    #[serde(default)]
    pub option: PictureOptions,
    /// Background color shading type.
    #[serde(default)]
    pub shade_type: ColorShadingType,
    /// Primary background color.
    pub primary_color: Option<HexColor>,
    /// Accent color override.
    pub accent_color: Option<HexColor>,
    /// Dark accent color override.
    pub dark_accent_color: Option<HexColor>,
}

/// The top-level metadata document read from a `metadata.toml`.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Metadata {
    /// Author definitions available to wallpapers in the same directory.
    #[serde(default)]
    pub authors: Vec<Author>,
    /// Wallpaper entries defined in this directory.
    #[serde(default)]
    pub wallpapers: Vec<Wallpaper>,
}

impl Author {
    /// Author email address.
    pub fn email(&self) -> &str {
        &self.email
    }

    /// Author name.
    pub fn name(&self) -> &Localized<String> {
        &self.name
    }
}

impl WallpaperPath {
    /// Returns the list of wallpaper paths.
    ///
    /// For a single path this returns a 1-element `Vec`.
    pub fn get_paths(&self) -> Vec<&Path> {
        match self {
            WallpaperPath::Single(path) => vec![path],
            WallpaperPath::Multiple(paths) => paths.iter().map(|p| p.as_ref()).collect(),
        }
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
    fn test_de_single_file() {
        let dummy_meta = toml::from_str::<Metadata>(DUMMY_META_SINGLE_FILE).unwrap();
        assert_eq!(dummy_meta.authors.len(), 1);
        assert_eq!(dummy_meta.wallpapers.len(), 1);
        assert_eq!(dummy_meta.wallpapers[0].path.get_paths().len(), 1);
    }

    #[test]
    fn test_de_multiple_file() {
        let dummy_meta = toml::from_str::<Metadata>(DUMMY_META_MULTIPLE_FILE).unwrap();
        assert_eq!(dummy_meta.authors.len(), 1);
        assert_eq!(dummy_meta.wallpapers.len(), 1);
        assert_eq!(dummy_meta.wallpapers[0].path.get_paths().len(), 2);
    }
}
