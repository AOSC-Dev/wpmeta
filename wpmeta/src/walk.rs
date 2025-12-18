use eyre::{Result, bail, ensure};
use log::{debug, info};

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::input::{Author, Metadata, Wallpaper};

static METADATA_FILENAME: &str = "metadata.toml";

#[derive(Clone, Debug)]
pub struct MetadataWrapper {
    parent: Option<Arc<MetadataWrapper>>,
    path: PathBuf,
    metadata: Metadata,
}

#[derive(Clone, Debug)]
pub struct DirectoryIter {
    paths: Vec<PathBuf>,
    parents: HashMap<PathBuf, Arc<MetadataWrapper>>,
}

impl MetadataWrapper {
    fn new(path: &Path, parent: Option<Arc<Self>>) -> Result<Arc<Self>> {
        info!("Parsing manifest at {}", path.display());
        let parent_path = path
            .parent()
            .expect("Failed to get parent path")
            .canonicalize()?;
        ensure!(parent_path.is_dir());

        let meta_content = fs::read_to_string(path)?;
        let metadata = toml::from_str::<Metadata>(&meta_content)?;

        if (!metadata.wallpapers.is_empty())
            && (metadata.authors.is_empty())
            && !parent
                .as_ref()
                .map(|p| p.authors().is_empty())
                .unwrap_or(true)
        {
            bail!(
                "{}: wallpaper defined, but no author definition found",
                path.display()
            );
        }

        Ok(Arc::new(Self {
            parent,
            path: parent_path,
            metadata: toml::from_str::<Metadata>(&meta_content)?,
        }))
    }

    pub fn authors(&self) -> Vec<&Author> {
        match &self.parent {
            None => self.metadata.authors.iter().collect(),
            Some(p) => p
                .authors()
                .into_iter()
                .chain(self.metadata.authors.iter())
                .collect(),
        }
    }

    pub fn wallpapers(&self) -> Vec<&Wallpaper> {
        self.metadata.wallpapers.iter().collect()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl DirectoryIter {
    pub fn start(path: &Path) -> Result<Self> {
        if !path.is_dir() {
            bail!("Starting path {} is not a directory", path.display());
        }

        Ok(Self {
            paths: vec![path.to_owned()],
            parents: HashMap::new(),
        })
    }
}

impl Iterator for DirectoryIter {
    type Item = Arc<MetadataWrapper>;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.paths.is_empty() {
            let dir = unsafe { self.paths.pop().unwrap_unchecked() };

            // Discover subdirectories
            fs::read_dir(&dir)
                .unwrap_or_else(|_| panic!("Failed to read directory {}", dir.display()))
                .for_each(|p| {
                    let entry = p
                        .unwrap_or_else(|_| panic!("Failed to list entries in {}", dir.display()))
                        .path();
                    if entry.is_dir() {
                        debug!("Discovered subdirectory {}", entry.display());
                        self.paths.push(entry);
                    }
                });

            // Check if metadata exists
            let metadata_path = dir.join(METADATA_FILENAME);
            if !metadata_path.is_file() {
                continue;
            }

            // Parse metadata
            let parent = dir.parent().and_then(|p| self.parents.get(p).cloned());
            let metadata =
                MetadataWrapper::new(&metadata_path, parent).expect("Failed to parse metadata");
            assert_eq!(dir, metadata.path);
            self.parents.insert(dir, metadata.clone());
            return Some(metadata);
        }
        None
    }
}
