use eyre::{eyre, Result};
use hex_color::HexColor;
use image::ImageReader;
use serde::{Deserialize, Serialize};

use locale::Localized;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Author {
    email: String,
    name: Localized<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PictureOptions {
    None,
    Wallpaper,
    Centered,
    Scaled,
    Stretched,
    Zoom,
    Spanned,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ColorShadingType {
    Horizontal,
    Vertical,
    Solid,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WallpaperFileMeta {
    target: PathBuf,
    dimensions: (u32, u32),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct WallpaperFile {
    id: String,
    path: PathBuf,
    #[serde(skip)]
    meta: OnceLock<WallpaperFileMeta>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Wallpaper {
    title: Localized<String>,
    license: String,
    #[serde(flatten)]
    file: WallpaperFile,
    #[serde(default)]
    option: PictureOptions,
    #[serde(default)]
    shade_type: ColorShadingType,
    #[serde(default = "default_primary_color")]
    primary_color: HexColor,
    #[serde(default = "default_secondary_color")]
    secondary_color: HexColor,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Metadata {
    #[serde(skip)]
    base: Option<PathBuf>,
    authors: Option<Vec<Author>>,
    wallpapers: Option<Vec<Wallpaper>>,
}

#[inline]
fn to_owned_option<T>(inner: Option<&T>) -> Option<T>
where
    T: Clone,
{
    inner.map(|t| t.to_owned())
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

impl Default for PictureOptions {
    fn default() -> Self {
        Self::Wallpaper
    }
}

impl Default for ColorShadingType {
    fn default() -> Self {
        Self::Solid
    }
}

impl WallpaperFileMeta {
    pub fn new(id: &str, file: &Path) -> Result<Self> {
        let img = ImageReader::open(file)?.decode()?;
        let (width, height) = (img.width(), img.height());
        let extension = file
            .extension()
            .ok_or_else(|| eyre!("cannot get file extension"))?
            .to_str()
            .ok_or_else(|| eyre!("cannot parse file extension"))?;
        // TODO: Implement automatic palette extraction
        Ok(Self {
            target: PathBuf::from(format!(
                "usr/share/wallpapers/{id}/contents/images/{width}x{height}.{extension}"
            )),
            dimensions: (width, height),
        })
    }

    pub fn target(&self) -> &Path {
        &self.target
    }

    pub fn dimensions(&self) -> (u32, u32) {
        self.dimensions
    }
}

impl WallpaperFile {
    pub fn src(&self) -> &Path {
        &self.path
    }

    pub fn get_meta(&self, base: &Path) -> &WallpaperFileMeta {
        self.meta.get_or_init(|| {
            let id = &self.id;
            let path = &base.join(&self.path);
            // TODO: Use get_or_try_init
            WallpaperFileMeta::new(id, path).unwrap_or_else(|_| panic!("{}: failed to process image metadata for image at {}",
                id,
                path.display()))
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

impl Wallpaper {
    pub fn id(&self) -> &str {
        self.file.id()
    }

    pub fn titles(&self) -> &Localized<String> {
        &self.title
    }

    pub fn license(&self) -> &str {
        &self.license
    }

    pub fn file(&self) -> &WallpaperFile {
        &self.file
    }

    pub fn src(&self) -> &Path {
        self.file().src()
    }

    pub fn target(&self, base: &Path) -> &Path {
        self.file().get_meta(base).target()
    }

    pub fn option(&self) -> &PictureOptions {
        &self.option
    }

    pub fn shade_type(&self) -> &ColorShadingType {
        &self.shade_type
    }

    pub fn colors(&self) -> (&HexColor, &HexColor) {
        (&self.primary_color, &self.secondary_color)
    }
}

impl Metadata {
    pub fn authors(&self) -> Option<&Vec<Author>> {
        self.authors.as_ref()
    }

    pub fn wallpapers(&self) -> Option<&Vec<Wallpaper>> {
        self.wallpapers.as_ref()
    }

    pub fn base(&self) -> Option<&Path> {
        self.base.as_deref()
    }

    pub fn flatten(&self, base: &Path, parent: Option<&Metadata>) -> Self {
        let mut authors = to_owned_option(self.authors());
        let mut wallpapers = to_owned_option(self.wallpapers());
        if let Some(p) = parent {
            if authors.is_none() {
                authors = to_owned_option(p.authors())
            }
            if wallpapers.is_none() {
                wallpapers = to_owned_option(p.wallpapers())
            }
        }
        Self {
            base: Some(base.into()),
            authors,
            wallpapers,
        }
    }
}

#[cfg(test)]
pub mod test {
    use super::Metadata;

    pub static DUMMY_META: &str = r#"
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

    #[test]
    fn test_de() {
        let dummy_meta = toml::from_str::<Metadata>(DUMMY_META).unwrap();
        assert_eq!(dummy_meta.authors().unwrap().len(), 1);
        assert_eq!(dummy_meta.wallpapers().unwrap().len(), 1);
    }
}
