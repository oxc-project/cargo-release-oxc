use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::versioning::cargo::CargoWorkspace;

const RELEASE_CONFIG: &str = "oxc_release.toml";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseConfig {
    #[serde(rename = "releases")]
    pub release_sets: Vec<ReleaseSet>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseSet {
    pub name: String,

    versioned_files: Vec<VersionedFile>,
}

impl ReleaseSet {
    #[must_use]
    pub fn versioned_packages(&self) -> Vec<VersionedPackage> {
        self.versioned_files.iter().flat_map(|v| v.content.versioned_packages()).collect::<Vec<_>>()
    }

    pub fn update_version(&self, version: &str) -> Result<()> {
        for versioned_file in &self.versioned_files {
            versioned_file.content.update_version(version)?;
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct VersionedFile {
    path: PathBuf,

    #[serde(skip)]
    content: VersionedContent,
}

#[derive(Debug, Default)]
pub enum VersionedContent {
    #[default]
    None,
    Cargo(CargoWorkspace),
}

#[derive(Debug, Clone)]
pub struct VersionedPackage {
    pub name: String,
    pub dir: PathBuf,
    pub path: PathBuf,
}

impl VersionedContent {
    fn read(path: &Path) -> Result<Self> {
        let file_name =
            path.file_name().with_context(|| format!("{path:?} does not have a filename."))?;
        let content = match file_name.to_string_lossy().as_ref() {
            "Cargo.toml" => Self::Cargo(CargoWorkspace::new(path)?),
            _ => anyhow::bail!("{path:?} is not recognized"),
        };
        Ok(content)
    }

    #[must_use]
    pub fn versioned_packages(&self) -> Vec<VersionedPackage> {
        match self {
            Self::None => vec![],
            Self::Cargo(cargo) => cargo.packages.clone(),
        }
    }

    pub fn update_version(&self, version: &str) -> Result<()> {
        match self {
            Self::None => Ok(()),
            Self::Cargo(cargo) => cargo.update_version(version),
        }
    }
}

impl ReleaseConfig {
    pub fn new(cwd: &Path) -> Result<Self> {
        let s =
            fs::read_to_string(cwd.join(RELEASE_CONFIG)).context("failed to read release.toml")?;
        let mut config: Self = toml::from_str(&s).context("failed to parse release.toml")?;
        for release_set in &mut config.release_sets {
            for versioned_file in &mut release_set.versioned_files {
                versioned_file.content = VersionedContent::read(&cwd.join(&versioned_file.path))?;
            }
        }
        Ok(config)
    }
}
