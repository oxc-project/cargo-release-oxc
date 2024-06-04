use std::{
    cell::RefCell,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde_json::Value;

use crate::config::VersionedPackage;

type RawJson = serde_json::Map<String, Value>;

#[derive(Debug)]
pub struct PackageJson {
    path: PathBuf,

    raw: RefCell<RawJson>,
}

impl PackageJson {
    pub fn new(path: &Path) -> Result<Self> {
        let content =
            fs::read_to_string(path).with_context(|| format!("failed to read {path:?}"))?;
        let raw: RawJson =
            serde_json::from_str(&content).with_context(|| format!("failed to parse {path:?}"))?;
        Ok(Self { path: path.to_path_buf(), raw: RefCell::new(raw) })
    }

    pub fn packages(&self) -> Vec<VersionedPackage> {
        vec![VersionedPackage {
            name: self.raw.borrow().get("name").unwrap().as_str().unwrap().to_string(),
            dir: self.path.parent().unwrap().to_path_buf(),
            path: self.path.clone(),
        }]
    }

    pub fn update_version(&self, version: &str) -> Result<()> {
        self.raw.borrow_mut().insert("version".to_string(), Value::String(version.to_string()));
        let json = serde_json::to_string_pretty(&self.raw).context("failed to write json")?;
        fs::write(&self.path, json).context("failed to write json")?;
        Ok(())
    }
}
