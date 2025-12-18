pub mod generate;
pub mod input;
pub mod walk;

use clap::Parser;
use eyre::Result;
use log::debug;
use rayon::prelude::*;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use generate::{
    GNOMEMetadataGenerator, KDEMetadataGenerator, MetadataGenerator, Resolution, Wallpaper,
    WallpaperCollection,
};
use walk::{DirectoryIter, MetadataWrapper};

#[derive(Clone, Debug, Parser)]
#[command(author, version, about)]
pub struct Args {
    #[arg(short, long)]
    src: PathBuf,
    #[arg(short, long)]
    dst: PathBuf,
    #[arg(short, long, default_value = "500,500")]
    preview_resolution_limit: Resolution,
}

fn generate_metadata(
    dst: &Path,
    wallpaper: &Wallpaper,
    preview_resolution_limit: Resolution,
) -> Result<()> {
    KDEMetadataGenerator::generate_metadata(dst, wallpaper, preview_resolution_limit)?;
    GNOMEMetadataGenerator::generate_metadata(dst, wallpaper, preview_resolution_limit)?;
    Ok(())
}

fn main() -> Result<()> {
    pretty_env_logger::init_custom_env("WPMETA_LOG");
    let args = Args::parse();
    debug!("Arguments: {:?}", &args);
    let iter = DirectoryIter::start(&args.src)?;
    let metas: Vec<Arc<MetadataWrapper>> = iter.collect();
    let _ = metas
        .par_iter()
        .map(|m| {
            WallpaperCollection::new(m.as_ref(), &args.dst)
                .expect("Failed to process wallpapers")
                .inner
        })
        .flatten()
        .map(|w| generate_metadata(&args.dst, &w, args.preview_resolution_limit))
        .collect::<Result<Vec<()>>>()?;
    Ok(())
}
