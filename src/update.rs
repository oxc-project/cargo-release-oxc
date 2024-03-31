use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use bpaf::Bpaf;
use cargo_metadata::{Metadata, MetadataCommand, Package};
use git_cliff_core::{
    changelog::Changelog, commit::Commit, config::Config, release::Release, repo::Repository,
    DEFAULT_CONFIG,
};
use semver::Version;
use toml_edit::{DocumentMut, Formatted, Value};

#[derive(Debug, Clone, Bpaf)]
pub struct UpdateOptions {
    #[bpaf(argument::<String>("version"), parse(parse_version))]
    version: Version,

    #[bpaf(positional("PATH"), fallback(PathBuf::from(".")))]
    path: PathBuf,
}

fn parse_version(version: String) -> Result<Version, semver::Error> {
    Version::parse(&version)
}

pub struct Update {
    options: UpdateOptions,
    metadata: Metadata,
    repo: Repository,
    config: Config,
    timestamp: i64,
}

impl Update {
    pub fn new(options: UpdateOptions) -> Result<Self> {
        let metadata = MetadataCommand::new().current_dir(&options.path).no_deps().exec()?;
        let root_path = metadata.workspace_root.clone().into_std_path_buf();
        let repo = Repository::init(root_path)?;
        let config_path = metadata.workspace_root.as_std_path().join(DEFAULT_CONFIG);
        let config = Config::parse(&config_path)?;
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        Ok(Self { options, metadata, repo, config, timestamp })
    }

    pub fn run(self) -> Result<()> {
        // `publish.is_none()` means `publish = true`.
        let packages = self
            .metadata
            .workspace_packages()
            .into_iter()
            .filter(|p| p.publish.is_none())
            .collect::<Vec<_>>();

        for package in &packages {
            self.generate_changelog_for_package(package)?;
        }
        self.update_cargo_toml_version_for_workspace(&packages)?;
        for package in &packages {
            self.update_cargo_toml_version_for_package(package.manifest_path.as_std_path())?;
        }

        Ok(())
    }

    fn version(&self) -> String {
        self.options.version.to_string()
    }

    fn generate_changelog_for_package(&self, package: &Package) -> Result<()> {
        let package_path = package.manifest_path.as_std_path().parent().unwrap();
        let release = Release {
            version: Some(format!("v{}", self.version())),
            commits: self.get_commits_for_package(package_path)?,
            commit_id: None,
            timestamp: self.timestamp,
            previous: None,
        };
        let changelog = Changelog::new(vec![release], &self.config)?;
        self.save_changelog(package_path, changelog)?;
        Ok(())
    }

    fn get_commits_for_package(&self, package_path: &Path) -> Result<Vec<Commit>> {
        let include_path = package_path
            .strip_prefix(self.metadata.workspace_root.as_std_path())
            .unwrap()
            .to_string_lossy();
        let include_path = glob::Pattern::new(&format!("{include_path}/**"))?;
        let commits = self
            .repo
            .commits(Some("1e9c0bc..HEAD".into()), Some(vec![include_path]), None)?
            .iter()
            .map(Commit::from)
            .collect::<Vec<_>>();
        Ok(commits)
    }

    fn save_changelog(&self, package_path: &Path, changelog: Changelog) -> Result<()> {
        let changelog_path = package_path.join("CHANGELOG.md");
        let prev_changelog_string = fs::read_to_string(&changelog_path).unwrap_or_default();
        let mut out = File::create(&changelog_path)?;
        changelog.prepend(prev_changelog_string, &mut out)?;
        Ok(())
    }

    fn update_toml(manifest_path: &Path, cb: impl FnOnce(&mut DocumentMut)) -> Result<()> {
        let manifest = fs::read_to_string(manifest_path)?;
        let mut manifest = DocumentMut::from_str(&manifest)?;
        cb(&mut manifest);
        let serialized = manifest.to_string();
        fs::write(manifest_path, serialized)?;
        Ok(())
    }

    fn update_cargo_toml_version_for_workspace(&self, packages: &[&Package]) -> Result<()> {
        let manifest_path = self.metadata.workspace_root.as_std_path().join("Cargo.toml");
        Self::update_toml(&manifest_path, |manifest| {
            let Some(table) = manifest
                .get_mut("workspace")
                .and_then(|item| item.as_table_mut())
                .and_then(|table| table.get_mut("dependencies"))
                .and_then(|item| item.as_table_mut())
            else {
                return;
            };
            for package in packages {
                if let Some(version) = table
                    .get_mut(&package.name)
                    .and_then(|item| item.as_inline_table_mut())
                    .and_then(|item| item.get_mut("version"))
                {
                    *version = Value::String(Formatted::new(self.version()));
                }
            }
        })
    }

    fn update_cargo_toml_version_for_package(&self, manifest_path: &Path) -> Result<()> {
        Self::update_toml(manifest_path, |manifest| {
            let Some(version) = manifest
                .get_mut("package")
                .and_then(|item| item.as_table_mut())
                .and_then(|table| table.get_mut("version"))
                .and_then(|item| item.as_value_mut())
            else {
                return;
            };
            *version = Value::String(Formatted::new(self.version()));
        })
    }
}
