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

// fn process_meta(meta: Metadata, dst: &Path) -> Result<()> {
//     info!("processing meta at {:?}", meta.base());
//     let cur = PathBuf::from(".");
//     let base = meta.base().unwrap_or(&cur);
//     for wallpaper in meta.wallpapers().unwrap() {
//         let id = wallpaper.id();
//         let target = wallpaper.target(base);
//         let gnome_meta = gnome_metas.get(id).unwrap();
//         let kde_meta = kde_metas.get(id).unwrap();
//
//         info!("{id}: writing metadata");
//         let gnome_meta_file = format!("{id}.xml");
//         write_file(
//             &dst.join(GNOME_META_BASE).join(&gnome_meta_file),
//             gnome_meta.as_bytes(),
//         )?;
//         write_file(
//             &dst.join(KDE_META_BASE).join(id).join("metadata.json"),
//             kde_meta.as_bytes(),
//         )?;
//         // Generate symlink for MATE
//         let mate_meta_path = dst.join(MATE_META_BASE).join(&gnome_meta_file);
//         if mate_meta_path.read_link().is_ok() {
//             remove_file(&mate_meta_path)?;
//         }
//         ensure_parent(&mate_meta_path)?;
//         symlink(
//             PathBuf::from("/")
//                 .join(GNOME_META_BASE)
//                 .join(&gnome_meta_file),
//             mate_meta_path,
//         )?;
//
//         let files = wallpaper.file().get_meta(base);
//         files.iter().for_each(|file| {
//             let src = file.source();
//             let dst = dst.join(file.target());
//             info!(
//                 "{}: copying wallpaper file {} -> {}",
//                 id,
//                 src.display(),
//                 dst.display()
//             );
//             copy_file(&src, &dst).expect("Failed to copy file");
//         });
//
//         let file_max = files.iter().max_by_key(|f| {
//             let dimensions = f.dimensions();
//             dimensions.0 * dimensions.1
//         }).expect("Failed to get the file with the highest resolution");
//         info!("{id}: generating preview ...");
//         generate_preview(
//             &file_max.source(),
//             &dst.join(KDE_META_BASE)
//                 .join(id)
//                 .join("contents/screenshot.jpg"),
//         )?;
//     }
//     Ok(())
// }

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
