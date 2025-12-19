//! GNOME metadata generator.
//!
//! Produces `gnome-background-properties/*.xml` and, when multiple resolutions exist, a GNOME
//! background list XML under the wallpaper's `contents/` directory.

use eyre::Result;
use hex_color::HexColor;
use log::{info, warn};
use serde::Serialize;
use tinytemplate::TinyTemplate;

use localized::{Locale, Localized};

use std::cell::LazyCell;
use std::path::Path;

use super::{
    ColorShadingType, MetadataGenerator, PictureOptions, Resolution, Wallpaper, WallpaperFile,
    WallpaperKind, write_file,
};

/// Name of the gnome-wp-list template.
const GNOME_WP_LIST_TEMPLATE: &str = "gnome-wp-list";

/// Template for gnome-wp-list.
static GNOME_WP_LIST_TEMPLATE_STR: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE wallpapers SYSTEM "gnome-wp-list.dtd">
<wallpapers>
    <wallpaper deleted="false">{{ if default_name }}
    <name>{ default_name }</name>{{ endif }}{{ for name in names }}
    <name xml:lang="{ name.locale }">{ name.name }</name>{{ endfor }}{{ if filename }}
    <filename>/{ filename }</filename>{{ endif }}{{ if filename_dark }}
    <filename-dark>/{ filename_dark }</filename-dark>{{ endif }}
    <options>{ options }</options>
    <shade_type>{ shade_type }</shade_type>
    <pcolor>{ pcolor }</pcolor>
    <scolor>{ scolor }</scolor>
    </wallpaper>
</wallpapers>"#;

/// Name of the GNOME background list template.
const GNOME_BACKGROUND_TEMPLATE: &str = "gnome-background";

/// Template for GNOME background list.
static GNOME_BACKGROUND_TEMPLATE_STR: &str = r#"<background>
    <static>
        <duration>8640000.0</duration>
        <file>{{ for file in files }}
            <size width="{ file.width }" height="{ file.height }">/{ file.path }</size>{{endfor}}
        </file>
    </static>
</background>"#;

thread_local! {
    static GNOME_TEMPLATES: LazyCell<TinyTemplate<'static>> = LazyCell::new(|| {
        let mut template = TinyTemplate::new();
        [
            (GNOME_WP_LIST_TEMPLATE, GNOME_WP_LIST_TEMPLATE_STR),
            (GNOME_BACKGROUND_TEMPLATE, GNOME_BACKGROUND_TEMPLATE_STR),
        ].into_iter().for_each(|(name, template_str)| {
            template.add_template(name, template_str).unwrap_or_else(|_| panic!("Failed to parse template {}", name));
        });
        template
    });
}

#[derive(Clone, Debug, Serialize)]
struct Name<'a> {
    locale: String,
    name: &'a str,
}

#[derive(Clone, Debug, Serialize)]
struct GNOMEWallpaperMeta<'a> {
    default_name: Option<&'a String>,
    names: Vec<Name<'a>>,
    filename: Option<&'a Path>,
    filename_dark: Option<&'a Path>,
    options: PictureOptions,
    shade_type: ColorShadingType,
    pcolor: HexColor,
    scolor: HexColor,
}

#[derive(Clone, Debug, Serialize)]
struct GNOMEWallpaperFile<'a> {
    width: usize,
    height: usize,
    path: &'a Path,
}

#[derive(Clone, Debug, Serialize)]
struct GNOMEWallpaperList<'a> {
    files: Vec<GNOMEWallpaperFile<'a>>,
}

/// Generates GNOME wallpaper manifests for a single [`Wallpaper`].
#[derive(Copy, Clone, Debug)]
pub struct GNOMEMetadataGenerator;

impl<'a> Name<'a> {
    /// Generate a vector of names from a [`Localized<String>`].
    pub fn flatten<F>(src: &'a Localized<String>, transform: F) -> Result<Vec<Self>>
    where
        F: Fn(&Locale) -> String,
    {
        Ok(src
            .to_hashmap(transform)?
            .into_iter()
            .map(|(locale, name)| Self { locale, name })
            .collect())
    }
}

impl<'a> GNOMEWallpaperMeta<'a> {
    pub fn new(
        wallpaper: &'a Wallpaper,
        file: Option<&'a Path>,
        file_dark: Option<&'a Path>,
    ) -> Result<Self> {
        let titles = wallpaper.title;
        let default_name = titles.get_default();
        // xml:lang tags uses "-" as the delimiter
        let names = Name::flatten(titles, |l| l.get_locale("-"))?;
        let (primary_color, accent_color) = wallpaper
            .get_colors(WallpaperKind::Normal)?
            .expect("No color definition found");
        info!("{}: Generating manifest for GNOME...", wallpaper.id);
        Ok(Self {
            default_name,
            names,
            filename: file,
            filename_dark: file_dark,
            options: wallpaper.options,
            shade_type: wallpaper.color_shading_type,
            pcolor: primary_color,
            scolor: accent_color,
        })
    }
}

impl<'a> GNOMEWallpaperFile<'a> {
    fn from_file(value: &'a WallpaperFile, base_dir: &Path) -> Self {
        Self {
            width: value.resolution.width,
            height: value.resolution.height,
            path: value
                .file_path
                .strip_prefix(base_dir)
                .expect("Failed to get relative path"),
        }
    }
}

impl<'a> GNOMEWallpaperList<'a> {
    fn from_files(value: Vec<&'a WallpaperFile>, base_dir: &Path) -> Self {
        Self {
            files: value
                .into_iter()
                .map(|f| GNOMEWallpaperFile::from_file(f, base_dir))
                .collect(),
        }
    }
}

impl GNOMEMetadataGenerator {
    /// Generate a list of multi-resolution wallpapers for GNOME.
    fn write_wp_list(
        file_path: &Path,
        target_base: &Path,
        files: Vec<&WallpaperFile>,
    ) -> Result<()> {
        let wp_list = GNOMEWallpaperList::from_files(files, target_base);
        let result = GNOME_TEMPLATES.with(|t| t.render(GNOME_BACKGROUND_TEMPLATE, &wp_list))?;
        write_file(file_path, result.as_bytes())?;
        Ok(())
    }
}

impl MetadataGenerator for GNOMEMetadataGenerator {
    fn generate_metadata(
        target_base: &Path,
        wallpaper: &Wallpaper,
        _preview_resolution: Resolution,
    ) -> Result<()> {
        let id = wallpaper.id;
        let wallpaper_base = Self::get_wallpaper_base(target_base, id).join("contents");

        let normal_wallpapers = wallpaper.get_normal_wallpapers();
        let normal_wallpaper_path = match normal_wallpapers.len() {
            0 => {
                warn!("{}: No normal wallpaper found", id);
                None
            }
            1 => Some(
                normal_wallpapers[0]
                    .file_path
                    .strip_prefix(target_base)
                    .expect("Failed to strip prefix")
                    .to_owned(),
            ),
            l => {
                info!(
                    "{}: Found multiple normal wallpapers, generating wallpaper list with {} versions...",
                    id, l
                );
                let wp_list = wallpaper_base.join("images/gnome-list.xml");
                Self::write_wp_list(&wp_list, target_base, normal_wallpapers)?;
                Some(
                    wp_list
                        .strip_prefix(target_base)
                        .expect("Failed to strip prefix")
                        .to_owned(),
                )
            }
        };

        let dark_wallpapers = wallpaper.get_dark_wallpapers();
        let dark_wallpaper_path = match dark_wallpapers.len() {
            0 => None,
            1 => Some(
                dark_wallpapers[0]
                    .file_path
                    .strip_prefix(target_base)
                    .expect("Failed to strip prefix")
                    .to_owned(),
            ),
            l => {
                info!(
                    "{}: Found multiple dark wallpapers, generating wallpaper list with {} versions...",
                    id, l
                );
                let wp_list = wallpaper_base.join("images_dark/gnome-list.xml");
                Self::write_wp_list(&wp_list, target_base, dark_wallpapers)?;
                Some(
                    wp_list
                        .strip_prefix(target_base)
                        .expect("Failed to strip prefix")
                        .to_owned(),
                )
            }
        };

        let manifest_path = target_base
            .join("usr/share/gnome-background-properties")
            .join(format!("{}.xml", id));
        let metadata = GNOMEWallpaperMeta::new(
            wallpaper,
            normal_wallpaper_path.as_deref(),
            dark_wallpaper_path.as_deref(),
        )?;
        let result = GNOME_TEMPLATES.with(|t| t.render(GNOME_WP_LIST_TEMPLATE, &metadata))?;
        write_file(&manifest_path, result.as_bytes())?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use hex_color::HexColor;

    use std::borrow::Cow;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::fs;

    use localized::Localized;

    use super::GNOMEMetadataGenerator;
    use crate::generate::test::{TempDir, localized_default_en_us, wallpaper_file};
    use crate::generate::{ColorShadingType, MetadataGenerator, PictureOptions, Resolution};
    use crate::generate::{Wallpaper, WallpaperKind};

    fn get_color_overrides() -> HashMap<WallpaperKind, (Option<HexColor>, Option<HexColor>)> {
        HashMap::from([
            (
                WallpaperKind::Normal,
                (
                    Some(HexColor::rgb(2, 60, 136)),
                    Some(HexColor::rgb(87, 137, 202)),
                ),
            ),
            (WallpaperKind::Dark, (None, None)),
        ])
    }

    #[test]
    fn test_manifest_includes_normal_and_dark_filenames() {
        let tmp = TempDir::new("gnome-manifest-normal-dark");
        let target_base = tmp.path();

        let title: Localized<String> = localized_default_en_us("Kusa", "Grass");
        let normal_path =
            target_base.join("usr/share/wallpapers/Kusa/contents/images/7680x4320.jpg");
        let dark_path =
            target_base.join("usr/share/wallpapers/Kusa/contents/images_dark/7680x4320-dark.jpg");
        let wallpaper = Wallpaper {
            id: "Kusa",
            license: Cow::Borrowed("CC BY-SA 4.0"),
            authors: vec![],
            title: &title,
            files: vec![
                wallpaper_file(normal_path, WallpaperKind::Normal, 7680, 4320),
                wallpaper_file(dark_path, WallpaperKind::Dark, 7680, 4320),
            ],
            color_shading_type: ColorShadingType::Solid,
            options: PictureOptions::Wallpaper,
            colors_overrides: get_color_overrides(),
            colors: RefCell::new(HashMap::new()),
        };

        GNOMEMetadataGenerator::generate_metadata(
            target_base,
            &wallpaper,
            Resolution {
                width: 500,
                height: 500,
            },
        )
        .unwrap();

        let manifest_path = target_base.join("usr/share/gnome-background-properties/Kusa.xml");
        let manifest = fs::read_to_string(&manifest_path).unwrap();
        let expected = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE wallpapers SYSTEM "gnome-wp-list.dtd">
<wallpapers>
    <wallpaper deleted="false">
    <name>Kusa</name>
    <name xml:lang="en-US">Grass</name>
    <filename>/usr/share/wallpapers/Kusa/contents/images/7680x4320.jpg</filename>
    <filename-dark>/usr/share/wallpapers/Kusa/contents/images_dark/7680x4320-dark.jpg</filename-dark>
    <options>wallpaper</options>
    <shade_type>solid</shade_type>
    <pcolor>#023C88</pcolor>
    <scolor>#5789CA</scolor>
    </wallpaper>
</wallpapers>"#;
        assert_eq!(manifest, expected);
    }

    #[test]
    fn test_generates_wallpaper_list_for_multiple_normals() {
        let tmp = TempDir::new("gnome-manifest-multiple-normals");
        let target_base = tmp.path();

        let title: Localized<String> = localized_default_en_us("Kusa", "Grass");
        let wp1 = target_base.join("usr/share/wallpapers/Kusa/contents/images/1920x1080.jpg");
        let wp2 = target_base.join("usr/share/wallpapers/Kusa/contents/images/3840x2160.jpg");
        let wallpaper = Wallpaper {
            id: "Kusa",
            license: Cow::Borrowed("CC BY-SA 4.0"),
            authors: vec![],
            title: &title,
            files: vec![
                wallpaper_file(wp1, WallpaperKind::Normal, 1920, 1080),
                wallpaper_file(wp2, WallpaperKind::Normal, 3840, 2160),
            ],
            color_shading_type: ColorShadingType::Solid,
            options: PictureOptions::Wallpaper,
            colors_overrides: get_color_overrides(),
            colors: RefCell::new(HashMap::new()),
        };

        GNOMEMetadataGenerator::generate_metadata(
            target_base,
            &wallpaper,
            Resolution {
                width: 500,
                height: 500,
            },
        )
        .unwrap();

        let list_path =
            target_base.join("usr/share/wallpapers/Kusa/contents/images/gnome-list.xml");
        let list_xml = fs::read_to_string(&list_path).unwrap();
        let expected_list = r#"<background>
    <static>
        <duration>8640000.0</duration>
        <file>
            <size width="1920" height="1080">/usr/share/wallpapers/Kusa/contents/images/1920x1080.jpg</size>
            <size width="3840" height="2160">/usr/share/wallpapers/Kusa/contents/images/3840x2160.jpg</size>
        </file>
    </static>
</background>"#;
        assert_eq!(list_xml, expected_list);

        let manifest_path = target_base.join("usr/share/gnome-background-properties/Kusa.xml");
        let manifest = fs::read_to_string(&manifest_path).unwrap();
        let expected_manifest = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE wallpapers SYSTEM "gnome-wp-list.dtd">
<wallpapers>
    <wallpaper deleted="false">
    <name>Kusa</name>
    <name xml:lang="en-US">Grass</name>
    <filename>/usr/share/wallpapers/Kusa/contents/images/gnome-list.xml</filename>
    <options>wallpaper</options>
    <shade_type>solid</shade_type>
    <pcolor>#023C88</pcolor>
    <scolor>#5789CA</scolor>
    </wallpaper>
</wallpapers>"#;
        assert_eq!(manifest, expected_manifest);
    }
}
