use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{Context, Result};
use cargo_metadata::MetadataCommand;
use toml_edit::{DocumentMut, Formatted, Value};

use crate::config::VersionedPackage;

#[derive(Debug)]
pub struct CargoWorkspace {
    path: PathBuf,

    pub packages: Vec<VersionedPackage>,
}

impl CargoWorkspace {
    pub fn new(path: &Path) -> Result<Self> {
        let dir = path.parent().context("path should have a parent")?;
        let metadata = MetadataCommand::new().current_dir(dir).no_deps().exec()?;
        let packages = metadata
            .workspace_packages()
            .into_iter()
            // `publish.is_none()` means `publish = true`.
            .filter(|p| p.publish.is_none())
            .map(|p| {
                Ok(VersionedPackage {
                    name: p.name.clone(),
                    dir: p
                        .manifest_path
                        .parent()
                        .context("path should have a parent")?
                        .as_std_path()
                        .to_path_buf(),
                    path: p.manifest_path.as_std_path().to_path_buf(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { path: path.to_path_buf(), packages })
    }

    pub fn update_version(&self, version: &str) -> Result<()> {
        let mut workspace_toml = CargoToml::new(&self.path)?;

        for package in &self.packages {
            workspace_toml.set_workspace_dependency_version(&package.name, version)?;
            let mut package_toml = CargoToml::new(&package.path)?;
            package_toml.set_package_version(version)?;
            package_toml.save()?;
        }

        workspace_toml.save()
    }
}

struct CargoToml {
    path: PathBuf,
    toml: DocumentMut,
}

impl CargoToml {
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
            anyhow::bail!("`workspace.dependencies` field not found: {:?}", self.path);
        };
        let Some(version_field) = table
            .get_mut(crate_name)
            .and_then(|item| item.as_inline_table_mut())
            .and_then(|item| item.get_mut("version"))
        else {
            anyhow::bail!("dependency `{}` not found: {:?}", crate_name, self.path);
        };
        *version_field = Value::String(Formatted::new(version.to_string()));
        Ok(())
    }

    fn set_package_version(&mut self, version: &str) -> Result<()> {
        let Some(version_field) = self
            .toml
            .get_mut("package")
            .and_then(|item| item.as_table_mut())
            .and_then(|table| table.get_mut("version"))
            .and_then(|item| item.as_value_mut())
        else {
            anyhow::bail!("No `package.version` field found: {:?}", self.path);
        };
        *version_field = Value::String(Formatted::new(version.to_string()));
        Ok(())
    }
}
