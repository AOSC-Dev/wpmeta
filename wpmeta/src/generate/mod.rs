//! Metadata generation for desktop environments.
//!
//! This module converts [`crate::input`] metadata into a normalized [`WallpaperCollection`] and
//! writes desktop-environment specific manifests (GNOME/KDE) into a staging directory.

mod gnome;
mod kde;

use eyre::{Result, bail, eyre};
use hex_color::HexColor;
use image::{ImageFormat, ImageReader};
use localized::Localized;
use log::{debug, warn};
use spdx::Expression;

use image::imageops::FilterType;
use std::borrow::Cow;
use std::fs::{File, copy, create_dir_all};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::input::Wallpaper as InputWallpaper;
pub use crate::input::{Author, ColorShadingType, PictureOptions};
use crate::walk::MetadataWrapper;

pub use gnome::GNOMEMetadataGenerator;
pub use kde::KDEMetadataGenerator;

/// Ensure a directory exists, creating it if needed.
pub fn ensure_dir(dir: &Path) -> Result<()> {
    if !dir.is_dir() {
        debug!("creating directory at {}", dir.display());
        create_dir_all(dir)?;
    }
    Ok(())
}

/// Ensure a file's parent directory exists.
pub fn ensure_parent(file: &Path) -> Result<()> {
    if let Some(parent) = file.parent() {
        ensure_dir(parent)
    } else {
        bail!("invalid path");
    }
}

/// Write bytes to a file, creating parent directories as needed.
pub fn write_file(target: &Path, content: &[u8]) -> Result<()> {
    ensure_parent(target)?;
    debug!("writing to {}", target.display());
    let mut f = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(target)?;
    f.write_all(content)?;
    Ok(())
}

/// Generate a 500×500 JPEG preview from an image file.
pub fn generate_preview(src: &Path, target: &Path) -> Result<()> {
    let img = ImageReader::open(src)?.decode()?;
    let img = img.resize(500, 500, FilterType::Lanczos3);
    ensure_parent(target)?;
    img.save_with_format(target, ImageFormat::Jpeg)?;
    Ok(())
}

/// Copy a file to `dst`, creating the destination parent directory as needed.
pub fn copy_file(src: &Path, dst: &Path) -> Result<()> {
    if !src.is_file() {
        bail!("src {} is not a file", src.display());
    }
    if let Some(parent) = dst.parent() {
        ensure_dir(parent)?;
    } else {
        bail!("invalid destination {}", dst.display());
    }
    debug!("copying {} to {}", src.display(), dst.display());
    copy(src, dst)?;
    Ok(())
}

/// A desktop-environment specific metadata generator.
pub trait MetadataGenerator {
    /// Returns the base installation directory for a wallpaper id.
    fn get_wallpaper_base(target_path: &Path, id: &str) -> PathBuf {
        target_path.join("usr/share/wallpapers").join(id)
    }

    /// Generate and write metadata into `target_base` for a single wallpaper.
    fn generate_metadata(
        target_base: &Path,
        wallpaper: &Wallpaper,
        preview_resolution: Resolution,
    ) -> Result<()>;
}

/// Image size in pixels.
#[derive(Copy, Clone, Debug)]
pub struct Resolution {
    /// Image width in pixels.
    pub width: usize,
    /// Image height in pixels.
    pub height: usize,
}

/// Whether a wallpaper file is a normal or dark variant.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WallpaperKind {
    /// Normal (light) variant.
    Normal,
    /// Dark variant.
    Dark,
}

/// A discovered (and usually copied) wallpaper file with derived metadata.
#[derive(Clone, Debug)]
pub struct WallpaperFile {
    /// File path in the staging directory.
    pub file_path: PathBuf,
    /// Image resolution.
    pub resolution: Resolution,
    /// Detected image format.
    pub format: ImageFormat,
    /// Variant type (normal/dark).
    pub kind: WallpaperKind,
    // primary_color: HexColor,  // TODO: Add automatic primary/secondary color extraction
    // secondary_color: HexColor,
}

/// A normalized wallpaper ready for metadata generation.
#[derive(Clone, Debug)]
pub struct Wallpaper<'a> {
    /// Wallpaper id.
    pub id: &'a str,
    /// Canonicalized SPDX license expression when possible.
    pub license: Cow<'a, str>,
    /// Authors applicable to this wallpaper.
    pub authors: Vec<&'a Author>,
    /// Wallpaper title.
    pub title: &'a Localized<String>,
    /// Available files (normal/dark and/or multiple resolutions).
    pub files: Vec<WallpaperFile>,
    /// Primary background color.
    pub primary_color: HexColor,
    /// Secondary background color.
    pub secondary_color: HexColor,
    /// Background shading type.
    pub color_shading_type: ColorShadingType,
    /// Desktop rendering option.
    pub options: PictureOptions,
}

/// A set of wallpapers built from a metadata tree.
#[derive(Clone, Debug)]
pub struct WallpaperCollection<'a> {
    /// The normalized wallpapers.
    pub inner: Vec<Wallpaper<'a>>,
}

impl FromStr for Resolution {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let values: Vec<&str> = s.split(',').collect();
        if values.len() != 2 {
            return Err(format!(
                "expected exactly 2 comma-separated positive integers, got {}",
                values.len()
            ));
        }
        let mut results = values.into_iter().enumerate().map(|(i, segment)| {
            segment
                .trim()
                .parse::<usize>()
                .map_err(|e| format!("Failed to parse argument {}: {}", i, e))
        });
        let width = unsafe { results.next().unwrap_unchecked() }?;
        let height = unsafe { results.next().unwrap_unchecked() }?;

        Ok(Self { width, height })
    }
}

impl WallpaperKind {
    /// Directory name used under `.../contents/` for this kind.
    pub const fn get_dir_name(&self) -> &str {
        match self {
            Self::Normal => "images",
            Self::Dark => "images_dark",
        }
    }
}

impl WallpaperFile {
    /// Read image metadata from an existing file path.
    ///
    /// The file's kind is inferred from the filename suffix (`*dark.*` => [`WallpaperKind::Dark`]).
    pub fn from_file(source_path: &Path) -> Result<Self> {
        let path_canonicalized = source_path.canonicalize()?;
        let filename = path_canonicalized
            .file_prefix()
            .ok_or_else(|| {
                eyre!(
                    "Failed to extract file name from path {}",
                    path_canonicalized.display()
                )
            })?
            .to_string_lossy();
        let kind = if filename.to_ascii_lowercase().ends_with("dark") {
            WallpaperKind::Dark
        } else {
            WallpaperKind::Normal
        };
        let img_reader = ImageReader::open(&path_canonicalized)?;
        let img_format = img_reader.format().ok_or_else(|| {
            eyre!(
                "Failed to determine file format for {}",
                path_canonicalized.display()
            )
        })?;
        let img = img_reader.decode()?;
        let resolution = Resolution {
            width: img.width() as usize,
            height: img.height() as usize,
        };

        Ok(Self {
            resolution,
            file_path: path_canonicalized,
            format: img_format,
            kind,
        })
    }

    /// Copy the wallpaper file to the target directory.
    pub fn copy_file(&self, target_directory: &Path) -> Result<Self> {
        let filename = format!(
            "{}x{}.{}",
            self.resolution.width,
            self.resolution.height,
            self.format.extensions_str()[0]
        );
        let target_path = target_directory
            .join("contents")
            .join(self.kind.get_dir_name())
            .join(filename);

        copy_file(&self.file_path, &target_path)?;
        Ok(Self {
            file_path: target_path.canonicalize()?,
            resolution: self.resolution,
            format: self.format,
            kind: self.kind,
        })
    }

    /// Generate a preview image for this wallpaper file.
    pub fn generate_preview(&self, output: &Path, resolution: Resolution) -> Result<()> {
        let img = ImageReader::open(&self.file_path)?.decode()?;
        let img = img.resize(
            resolution.width as u32,
            resolution.height as u32,
            FilterType::Lanczos3,
        );
        ensure_parent(output)?;
        img.save_with_format(output, ImageFormat::Jpeg)?;
        Ok(())
    }
}

impl<'a> Wallpaper<'a> {
    fn new(
        wp: &'a InputWallpaper,
        authors: &[&'a Author],
        source_dir: &Path,
        target_dir: &Path,
    ) -> Result<Self> {
        let license = match Expression::canonicalize(wp.license.as_str()) {
            Ok(Some(res)) => Cow::Owned(res),
            _ => {
                warn!(
                    "{}: {} is not a valid SPDX license identifier",
                    wp.id.as_str(),
                    wp.license.as_str()
                );
                Cow::Borrowed(wp.license.as_str())
            }
        };

        let files = wp
            .path
            .get_paths()
            .iter()
            .map(|p| WallpaperFile::from_file(&source_dir.join(p)))
            .collect::<Result<Vec<_>>>()?;
        if files.is_empty() {
            bail!("{}: Wallpaper defined but no files given", wp.id);
        }

        // Copy files over
        let target_directory = target_dir.join("usr/share/wallpapers").join(&wp.id);
        let files = files
            .into_iter()
            .map(|wp| wp.copy_file(&target_directory))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            id: &wp.id,
            license,
            title: &wp.title,
            authors: authors.to_owned(),
            files,
            primary_color: wp.primary_color,
            secondary_color: wp.secondary_color,
            color_shading_type: wp.shade_type,
            options: wp.option,
        })
    }

    fn get_wallpapers<F>(&self, predicate: F) -> Vec<&WallpaperFile>
    where
        F: Fn(&WallpaperFile) -> bool,
    {
        self.files.iter().filter(|w| predicate(w)).collect()
    }

    pub fn get_normal_wallpapers(&self) -> Vec<&WallpaperFile> {
        self.get_wallpapers(|w| w.kind == WallpaperKind::Normal)
    }

    /// Returns dark-variant wallpaper files.
    pub fn get_dark_wallpapers(&self) -> Vec<&WallpaperFile> {
        self.get_wallpapers(|w| w.kind == WallpaperKind::Dark)
    }

    /// Returns `true` if any normal wallpaper file exists.
    pub fn has_normal_wallpaper(&self) -> bool {
        !self.get_normal_wallpapers().is_empty()
    }

    /// Returns `true` if any dark wallpaper file exists.
    pub fn has_dark_wallpaper(&self) -> bool {
        !self.get_dark_wallpapers().is_empty()
    }

    /// Generate a preview image for this wallpaper.
    ///
    /// Picks the largest available file from the normal variant if present, otherwise the dark
    /// variant.
    pub fn generate_preview(&self, output: &Path, resolution: Resolution) -> Result<()> {
        if self.files.is_empty() {
            bail!("No wallpaper file definition found");
        }
        if self.has_normal_wallpaper() {
            self.get_normal_wallpapers()
        } else {
            self.get_dark_wallpapers()
        }
        .iter()
        .max_by_key(|w| w.resolution.width * w.resolution.height)
        .unwrap()
        .generate_preview(output, resolution)
    }
}

impl<'a> WallpaperCollection<'a> {
    /// Build a [`WallpaperCollection`] from a parsed [`MetadataWrapper`], copying files into the
    /// staging directory.
    pub fn new(value: &'a MetadataWrapper, base_directory: &Path) -> Result<Self> {
        let authors = value.authors();
        let wallpapers = value
            .wallpapers()
            .iter()
            .map(|w| Wallpaper::new(w, &authors, value.path(), base_directory))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self { inner: wallpapers })
    }
}

#[cfg(test)]
/// Shared helpers for unit tests in `crate::generate`.
pub(crate) mod test {
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use image::ImageFormat;
    use localized::{Locale, Localized};

    use super::{Resolution, WallpaperFile, WallpaperKind};

    /// A best-effort temporary directory that is removed on drop.
    pub(crate) struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        /// Create a new temp directory.
        pub(crate) fn new(prefix: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time went backwards")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "wpmeta-test-{}-{}-{unique}",
                std::process::id(),
                prefix
            ));
            std::fs::create_dir_all(&path).expect("Failed to create temp dir");
            Self { path }
        }

        pub(crate) fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    /// Build a localized string with a default and an `en-US` entry.
    pub(crate) fn localized_default_en_us(default: &str, en_us: &str) -> Localized<String> {
        let mut localized = Localized::new(Some(default.to_owned()));
        localized.insert(Locale::new("en-US"), en_us.to_owned());
        localized
    }

    /// Build a localized string with a default and a `zh-CN` entry.
    pub(crate) fn localized_default_zh_cn(default: &str, zh_cn: &str) -> Localized<String> {
        let mut localized = Localized::new(Some(default.to_owned()));
        localized.insert(Locale::new("zh-CN"), zh_cn.to_owned());
        localized
    }

    /// Construct a [`WallpaperFile`] for tests without reading an image from disk.
    pub(crate) fn wallpaper_file(
        path: PathBuf,
        kind: WallpaperKind,
        width: usize,
        height: usize,
    ) -> WallpaperFile {
        WallpaperFile {
            file_path: path,
            resolution: Resolution { width, height },
            format: ImageFormat::Jpeg,
            kind,
        }
    }
}
