pub mod generate;
pub mod meta;
pub mod walk;

use clap::Parser;
use eyre::{bail, Result, WrapErr};
use image::imageops::FilterType;
use image::io::Reader as ImageReader;
use image::ImageFormat;
use log::{debug, info};
use rayon::prelude::*;

use std::fs::{copy, create_dir_all, remove_file, File};
use std::io::Write;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use generate::{render_gnome, render_kde};
use meta::Metadata;

static MATE_META_BASE: &str = "usr/share/mate-background-properties";
static GNOME_META_BASE: &str = "usr/share/gnome-background-properties";
static KDE_META_BASE: &str = "usr/share/wallpapers";

#[derive(Parser)]
#[command(author, version, about)]
pub struct Args {
    #[arg(short, long)]
    src: PathBuf,
    #[arg(short, long)]
    dst: PathBuf,
}

fn ensure_dir(dir: &Path) -> Result<()> {
    if !dir.is_dir() {
        debug!("creating directory at {}", dir.display());
        create_dir_all(dir)?;
    }
    Ok(())
}

fn ensure_parent(file: &Path) -> Result<()> {
    if let Some(parent) = file.parent() {
        ensure_dir(parent)
    } else {
        bail!("invalid path");
    }
}

fn write_file(target: &Path, content: &[u8]) -> Result<()> {
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

fn generate_preview(src: &Path, target: &Path) -> Result<()> {
    let img = ImageReader::open(src)?.decode()?;
    let img = img.resize(500, 500, FilterType::Lanczos3);
    ensure_parent(target)?;
    img.save_with_format(target, ImageFormat::Jpeg)?;
    Ok(())
}

fn copy_file(src: &Path, dst: &Path) -> Result<()> {
    if !src.is_file() {
        bail!("src {} is not a file", src.display());
    }
    if let Some(parent) = dst.parent() {
        ensure_dir(parent)?;
    } else {
        bail!("invalid destination {}", dst.display());
    }
    info!("copying {} to {}", src.display(), dst.display());
    copy(src, dst)?;
    Ok(())
}

fn process_meta(meta: Metadata, dst: &Path) -> Result<()> {
    info!("processing meta at {:?}", meta.base());
    let cur = PathBuf::from(".");
    let base = meta.base().unwrap_or(&cur);
    let gnome_metas = render_gnome(&meta, base)?;
    let kde_metas = render_kde(&meta)?;
    for wallpaper in meta.wallpapers().unwrap() {
        let id = wallpaper.id();
        let src = base.join(wallpaper.src());
        let target = wallpaper.target(base);
        let gnome_meta = gnome_metas.get(id).unwrap();
        let kde_meta = kde_metas.get(id).unwrap();

        info!("{}: writing metadata", id);
        let gnome_meta_file = format!("{}.xml", id);
        write_file(
            &dst.join(GNOME_META_BASE).join(&gnome_meta_file),
            gnome_meta.as_bytes(),
        )?;
        write_file(
            &dst.join(KDE_META_BASE).join(id).join("metadata.json"),
            kde_meta.as_bytes(),
        )?;
        // Generate symlink for MATE
        let mate_meta_path = dst.join(MATE_META_BASE).join(&gnome_meta_file);
        if mate_meta_path.read_link().is_ok() {
            remove_file(&mate_meta_path)?;
        }
        ensure_parent(&mate_meta_path)?;
        symlink(
            PathBuf::from("/")
                .join(GNOME_META_BASE)
                .join(&gnome_meta_file),
            mate_meta_path,
        )?;

        let wallpaper_dst = dst.join(target);
        info!(
            "{}: copying wallpaper file {} -> {}",
            id,
            src.display(),
            wallpaper_dst.display()
        );
        copy_file(&src, &wallpaper_dst)?;

        info!("{}: generating preview ...", id);
        generate_preview(
            &src,
            &dst.join(KDE_META_BASE)
                .join(id)
                .join("contents/screenshot.jpg"),
        )?;
    }
    Ok(())
}

fn main() -> Result<()> {
    pretty_env_logger::init_custom_env("WPMETA_LOG");
    let args = Args::parse();
    let metas = walk::walk(&args.src, None)?;

    debug!("processing: {:?}", metas);
    let _: Vec<()> = metas
        .into_par_iter()
        .map(|m| {
            process_meta(m, &args.dst)
                .wrap_err("failed to process wallpapers")
                .unwrap();
        })
        .collect();
    Ok(())
}
