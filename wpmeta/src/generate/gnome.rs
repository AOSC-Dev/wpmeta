use eyre::{eyre, Result};
use hex_color::HexColor;
use serde::Serialize;
use tinytemplate::TinyTemplate;

use locale::{Locale, Localized};

use std::collections::HashMap;
use std::path::Path;

use crate::meta::{ColorShadingType, Metadata, PictureOptions, Wallpaper};

static GNOME_WP_LIST_TEMPLATE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE wallpapers SYSTEM "gnome-wp-list.dtd">
<wallpapers>
    <wallpaper deleted="false">{{ if default_name }}
    <name>{ default_name }</name>{{ endif }}{{ for name in names }}
    <name xml:lang="{ name.locale }">{ name.name }</name>{{ endfor }}
    <filename>/{ filename }</filename>
    <options>{ options }</options>
    <shade_type>{ shade_type }</shade_type>
    <pcolor>{ pcolor }</pcolor>
    <scolor>{ scolor }</scolor>
    </wallpaper>
</wallpapers>"#;

#[derive(Clone, Debug, Serialize)]
pub struct Name<'a> {
    locale: &'a str,
    name: &'a str,
}

#[derive(Clone, Debug, Serialize)]
pub struct GNOMEWallpaperMeta<'a> {
    default_name: Option<&'a String>,
    names: Vec<Name<'a>>,
    filename: &'a Path,
    options: &'a PictureOptions,
    shade_type: &'a ColorShadingType,
    pcolor: &'a HexColor,
    scolor: &'a HexColor,
}

impl<'a> Name<'a> {
    pub fn flatten<F>(src: &'a Localized<String>, transform: F) -> Result<Vec<Self>>
    where
        F: Fn(&Locale) -> &str,
    {
        Ok(src
            .generate_hashmap(transform)?
            .into_iter()
            .map(|(locale, name)| Self { locale, name })
            .collect())
    }
}

impl<'a> GNOMEWallpaperMeta<'a> {
    pub fn new(wallpaper: &'a Wallpaper, base: &Path) -> Result<Self> {
        let titles = wallpaper.titles();
        let default_name = titles.get_default();
        let names = Name::flatten(titles, |l| l.to_locale())?;
        let (pcolor, scolor) = wallpaper.colors();
        Ok(Self {
            default_name,
            names,
            filename: wallpaper.target(base),
            options: wallpaper.option(),
            shade_type: wallpaper.shade_type(),
            pcolor,
            scolor,
        })
    }
}

pub fn render_gnome<'a>(metadata: &'a Metadata, base: &Path) -> Result<HashMap<&'a str, String>> {
    let mut template = TinyTemplate::new();
    template.add_template("gnome-wp-list", GNOME_WP_LIST_TEMPLATE)?;
    let wallpapers = metadata
        .wallpapers()
        .ok_or_else(|| eyre!("Failed to get wallpaper list"))?;
    let mut ret = HashMap::new();
    for wallpaper in wallpapers {
        let target = GNOMEWallpaperMeta::new(wallpaper, base)?;
        ret.insert(wallpaper.id(), template.render("gnome-wp-list", &target)?);
    }
    Ok(ret)
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use super::render_gnome;
    use crate::meta::Metadata;

    #[test]
    fn test_render() {
        let dummy_meta = toml::from_str::<Metadata>(crate::meta::test::DUMMY_META).unwrap();
        let result = render_gnome(&dummy_meta, &PathBuf::from(".")).unwrap();
        assert_eq!(
            result.get("Kusa").unwrap(),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE wallpapers SYSTEM "gnome-wp-list.dtd">
<wallpapers>
    <wallpaper deleted="false">
    <name>Kusa</name>
    <name xml:lang="en-US">Grass</name>
    <filename>/usr/share/wallpapers/Kusa/contents/images/7680x4320.jpg</filename>
    <options>wallpaper</options>
    <shade_type>solid</shade_type>
    <pcolor>#023C88</pcolor>
    <scolor>#5789CA</scolor>
    </wallpaper>
</wallpapers>"#
        );
    }
}
