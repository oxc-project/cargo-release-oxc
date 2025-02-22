use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::versioning::{cargo::CargoToml, package_json::PackageJson};

const RELEASE_CONFIG: &str = "oxc_release.toml";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseConfig {
    #[serde(rename = "releases")]
    release_sets: Vec<ReleaseSet>,
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

    pub fn get_release(self, release_name: &str) -> Result<ReleaseSet> {
        match self.release_sets.into_iter().find(|r| r.name == release_name) {
            Some(release_set) => Ok(release_set),
            _ => {
                anyhow::bail!("release {} not found", release_name);
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseSet {
    pub name: String,

    pub scopes_for_breaking_change: Option<Vec<String>>,

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

    pub fn commits_range(&self, version: &str) -> String {
        format!("{}_v{version}..HEAD", &self.name)
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
    Cargo(CargoToml),
    PackageJson(PackageJson),
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
            "Cargo.toml" => Self::Cargo(CargoToml::new(path)?),
            "package.json" => Self::PackageJson(PackageJson::new(path)?),
            _ => anyhow::bail!("{path:?} is not recognized"),
        };
        Ok(content)
    }

    #[must_use]
    pub fn versioned_packages(&self) -> Vec<VersionedPackage> {
        match self {
            Self::None => vec![],
            Self::Cargo(cargo) => cargo.packages(),
            Self::PackageJson(package_json) => package_json.packages(),
        }
    }

    pub fn update_version(&self, version: &str) -> Result<()> {
        match self {
            Self::None => Ok(()),
            Self::Cargo(cargo) => cargo.update_version(version),
            Self::PackageJson(package_json) => package_json.update_version(version),
        }
    }
}
