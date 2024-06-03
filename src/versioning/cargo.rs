use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::Result;
use toml_edit::{DocumentMut, Formatted, Value};

pub struct CargoToml {
    manifest_path: PathBuf,
    toml: DocumentMut,
}

impl CargoToml {
    pub fn new(manifest_path: &Path) -> Result<Self> {
        let manifest = fs::read_to_string(manifest_path)?;
        let toml = DocumentMut::from_str(&manifest)?;
        Ok(Self { manifest_path: manifest_path.to_path_buf(), toml })
    }

    pub fn save(self) -> Result<()> {
        let serialized = self.toml.to_string();
        fs::write(self.manifest_path, serialized)?;
        Ok(())
    }

    pub fn set_version(&mut self, version: &str) -> Result<()> {
        let Some(version_field) = self
            .toml
            .get_mut("package")
            .and_then(|item| item.as_table_mut())
            .and_then(|table| table.get_mut("version"))
            .and_then(|item| item.as_value_mut())
        else {
            anyhow::bail!("No `package.version` field found: {:?}", self.manifest_path);
        };
        *version_field = Value::String(Formatted::new(version.to_string()));
        Ok(())
    }

    pub fn set_workspace_dependency_version(
        &mut self,
        crate_name: &str,
        version: &str,
    ) -> Result<()> {
        let Some(table) = self
            .toml
            .get_mut("workspace")
            .and_then(|item| item.as_table_mut())
            .and_then(|table| table.get_mut("dependencies"))
            .and_then(|item| item.as_table_mut())
        else {
            anyhow::bail!("`workspace.dependencies` field not found: {:?}", self.manifest_path);
        };
        let Some(version_field) = table
            .get_mut(crate_name)
            .and_then(|item| item.as_inline_table_mut())
            .and_then(|item| item.get_mut("version"))
        else {
            anyhow::bail!("dependency `{}` not found: {:?}", crate_name, self.manifest_path);
        };
        *version_field = Value::String(Formatted::new(version.to_string()));
        Ok(())
    }
}
