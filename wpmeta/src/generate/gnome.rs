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
    write_file,
};

const GNOME_WP_LIST_TEMPLATE: &str = "gnome-wp-list";
static GNOME_WP_LIST_TEMPLATE_STR: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE wallpapers SYSTEM "gnome-wp-list.dtd">
<wallpapers>
    <wallpaper deleted="false">{{ if default_name }}
    <name>{ default_name }</name>{{ endif }}{{ for name in names }}
    <name xml:lang="{ name.locale }">{ name.name }</name>{{ endfor }}{{ if filename }}
    <filename>/{ filename }</filename>{{ endif }}{{ if filename_dark }}
    <filename-dark>/{ filename }</filename-dark>{{ endif }}
    <options>{ options }</options>
    <shade_type>{ shade_type }</shade_type>
    <pcolor>{ pcolor }</pcolor>
    <scolor>{ scolor }</scolor>
    </wallpaper>
</wallpapers>"#;

const GNOME_BACKGROUND_TEMPLATE: &str = "gnome-background";
static GNOME_BACKGROUND_TEMPLATE_STR: &str = r#"<background>
    <static>
        <duration>8640000.0</duration>
        <file>{{ for file in files }}
            <size width="{ file.width }" height="{ file.height }">{ file.path }</size>{{endfor}}
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
    file: &'a Path,
}

#[derive(Clone, Debug, Serialize)]
struct GNOMEWallpaperList<'a> {
    files: Vec<GNOMEWallpaperFile<'a>>,
}

#[derive(Copy, Clone, Debug)]
pub struct GNOMEMetadataGenerator;

impl<'a> Name<'a> {
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
        Ok(Self {
            default_name,
            names,
            filename: file,
            filename_dark: file_dark,
            options: wallpaper.options,
            shade_type: wallpaper.color_shading_type,
            pcolor: wallpaper.primary_color,
            scolor: wallpaper.secondary_color,
        })
    }
}

impl<'a> GNOMEWallpaperFile<'a> {
    fn from_file(value: &'a WallpaperFile, base_dir: &Path) -> Self {
        Self {
            width: value.resolution.width,
            height: value.resolution.height,
            file: value
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
                Some(wp_list)
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
                let wp_list = wallpaper_base.join("images-dark/gnome-list.xml");
                Self::write_wp_list(&wp_list, target_base, dark_wallpapers)?;
                Some(wp_list)
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

// #[cfg(test)]
// mod test {
//     use std::path::PathBuf;
//
//     use super::render_gnome;
//     use crate::input::Metadata;
//
//     #[test]
//     fn test_render() {
//         let dummy_meta = toml::from_str::<Metadata>(crate::input::test::DUMMY_META).unwrap();
//         let result = render_gnome(&dummy_meta, &PathBuf::from(".")).unwrap();
//         assert_eq!(
//             result.get("Kusa").unwrap(),
//             r#"<?xml version="1.0" encoding="UTF-8"?>
// <!DOCTYPE wallpapers SYSTEM "gnome-wp-list.dtd">
// <wallpapers>
//     <wallpaper deleted="false">
//     <name>Kusa</name>
//     <name xml:lang="en-US">Grass</name>
//     <filename>/usr/share/wallpapers/Kusa/contents/images/7680x4320.jpg</filename>
//     <options>wallpaper</options>
//     <shade_type>solid</shade_type>
//     <pcolor>#023C88</pcolor>
//     <scolor>#5789CA</scolor>
//     </wallpaper>
// </wallpapers>"#
//         );
//     }
// }
