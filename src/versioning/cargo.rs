use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{Context, Result};
use cargo_metadata::MetadataCommand;
use toml_edit::{DocumentMut, Formatted, Item, Value};

use crate::config::VersionedPackage;

#[derive(Debug)]
pub struct CargoToml {
    path: PathBuf,

    is_workspace: bool,

    packages: Vec<VersionedPackage>,
}

impl CargoToml {
    pub fn new(path: &Path) -> Result<Self> {
        let dir = path.parent().unwrap();

        let toml = DocumentMut::from_str(&fs::read_to_string(path)?)?;
        let is_workspace = toml.contains_key("workspace");

        let packages = if is_workspace {
            let metadata = MetadataCommand::new().current_dir(dir).no_deps().exec()?;
            metadata
                .workspace_packages()
                .into_iter()
                // `publish.is_none()` means `publish = true`.
                .filter(|p| p.publish.is_none())
                .map(|p| VersionedPackage {
                    name: p.name.to_string(),
                    dir: p.manifest_path.parent().unwrap().as_std_path().to_path_buf(),
                    path: p.manifest_path.as_std_path().to_path_buf(),
                })
                .collect::<Vec<_>>()
        } else {
            vec![VersionedPackage {
                name: toml
                    .get("package")
                    .and_then(|item| item.as_table())
                    .and_then(|table| table.get("name"))
                    .and_then(|value| value.as_str())
                    .context("expect package name")?
                    .to_string(),
                dir: path.parent().unwrap().to_path_buf(),
                path: path.to_path_buf(),
            }]
        };

        Ok(Self { path: path.to_path_buf(), is_workspace, packages })
    }

    pub fn packages(&self) -> Vec<VersionedPackage> {
        self.packages.clone()
    }

    pub fn update_version(&self, version: &str) -> Result<()> {
        if self.is_workspace {
            let mut workspace_toml = CargoTomlFile::new(&self.path)?;
            // Bump `[workspace.package].version` for members that inherit it via
            // `version.workspace = true` (no-op if the workspace doesn't set one).
            workspace_toml.set_workspace_package_version(version);
            for package in &self.packages {
                workspace_toml.set_workspace_dependency_version(&package.name, version)?;
            }
            workspace_toml.save()?;
        }
        for package in &self.packages {
            let mut package_toml = CargoTomlFile::new(&package.path)?;
            package_toml.set_package_version(version)?;
            package_toml.save()?;
        }
        Ok(())
    }
}

struct CargoTomlFile {
    path: PathBuf,
    toml: DocumentMut,
}

impl CargoTomlFile {
    fn new(path: &Path) -> Result<Self> {
        let toml = DocumentMut::from_str(&fs::read_to_string(path)?)?;
        Ok(Self { path: path.to_path_buf(), toml })
    }

    fn save(self) -> Result<()> {
        let serialized = self.toml.to_string();
        fs::write(self.path, serialized)?;
        Ok(())
    }

    fn set_workspace_dependency_version(&mut self, crate_name: &str, version: &str) -> Result<()> {
        let Some(table) = self
            .toml
            .get_mut("workspace")
            .and_then(|item| item.as_table_mut())
            .and_then(|table| table.get_mut("dependencies"))
            .and_then(|item| item.as_table_mut())
        else {
            anyhow::bail!("`workspace.dependencies` field not found: {}", self.path.display());
        };
        // Match the dependency by its key, or by its `package` field for a renamed
        // dependency, e.g. `alias = { package = "crate_name", version = ".." }`.
        let Some(key) = table
            .iter()
            .find(|(key, item)| {
                *key == crate_name
                    || item
                        .as_inline_table()
                        .and_then(|t| t.get("package"))
                        .and_then(toml_edit::Value::as_str)
                        == Some(crate_name)
            })
            .map(|(key, _)| key.to_string())
        else {
            anyhow::bail!("dependency `{}` not found: {}", crate_name, self.path.display());
        };
        let Some(version_field) = table
            .get_mut(&key)
            .and_then(|item| item.as_inline_table_mut())
            .and_then(|item| item.get_mut("version"))
        else {
            anyhow::bail!("dependency `{}` has no `version`: {}", crate_name, self.path.display());
        };
        *version_field = Value::String(Formatted::new(version.to_string()));
        Ok(())
    }

    /// Set `[workspace.package].version` if present; no-op otherwise.
    fn set_workspace_package_version(&mut self, version: &str) {
        if let Some(version_field) = self
            .toml
            .get_mut("workspace")
            .and_then(|item| item.as_table_mut())
            .and_then(|table| table.get_mut("package"))
            .and_then(|item| item.as_table_mut())
            .and_then(|table| table.get_mut("version"))
            .and_then(|item| item.as_value_mut())
        {
            *version_field = Value::String(Formatted::new(version.to_string()));
        }
    }

    fn set_package_version(&mut self, version: &str) -> Result<()> {
        let Some(version_item) = self
            .toml
            .get_mut("package")
            .and_then(|item| item.as_table_mut())
            .and_then(|table| table.get_mut("version"))
        else {
            anyhow::bail!("No `package.version` field found: {}", self.path.display());
        };
        // `version.workspace = true` is inherited and bumped on the workspace instead.
        if is_workspace_inherited(version_item) {
            return Ok(());
        }
        let Some(version_field) = version_item.as_value_mut() else {
            anyhow::bail!("`package.version` is not a string: {}", self.path.display());
        };
        *version_field = Value::String(Formatted::new(version.to_string()));
        Ok(())
    }
}

/// Whether a manifest field is inherited from the workspace (`<field>.workspace = true`).
fn is_workspace_inherited(item: &Item) -> bool {
    item.as_table_like().and_then(|t| t.get("workspace")).and_then(Item::as_bool) == Some(true)
}
