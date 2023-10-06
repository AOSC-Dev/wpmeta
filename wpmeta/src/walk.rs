use log::{info, warn};
use eyre::{Result, bail};

use std::path::Path;
use std::fs;

use crate::meta::Metadata;

static METADATA_FILE: &str = "metadata.toml";

pub fn extract_meta<'a>(base: &Path, meta: Option<Metadata>, parent: Option<&Metadata>) -> Option<Metadata> {
    if meta.is_none() {
        return None
    }
    let m = meta.unwrap();
    if m.wallpapers().is_none() {
        return None
    }
    let ret = m.flatten(base, parent);
    if ret.authors().is_none() || ret.wallpapers().is_none() {
        warn!("incomplete manifest found at {}, ignoring ...", base.display());
        return None
    }
    Some(ret)
}

pub fn walk<'a>(path: &Path, parent: Option<&Metadata>) -> Result<Vec<Metadata>> {
    info!("Visiting {}", path.display());
    if ! path.exists() {
        bail!("path {:?} does not exist.", path);
    }
    if ! path.is_dir() {
        bail!("path {:?} is not a directory", path);
    }
    let meta_file = path.join(METADATA_FILE);
    let meta = if meta_file.exists() {
        let meta_content = fs::read_to_string(meta_file)?;
        Some(toml::from_str::<Metadata>(&meta_content)?)
    } else {
        None
    };
    let mut ret = Vec::new();
    let meta_flattened = extract_meta(path, meta.clone(), parent);
    if meta_flattened.is_some() {
        ret.push(meta_flattened.unwrap());
    }
    for path in fs::read_dir(path)? {
        let entry = path?;
        if ! entry.file_type()?.is_dir() {
            continue;
        }
        let mut res = walk(&entry.path(), meta.as_ref())?;
        ret.append(&mut res);
    }
    Ok(ret)
}
