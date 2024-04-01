use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use bpaf::Bpaf;
use cargo_metadata::{Metadata, MetadataCommand, Package};
use git_cliff_core::{
    changelog::Changelog, commit::Commit, config::Config, release::Release, repo::Repository,
    DEFAULT_CONFIG,
};
use semver::Version;
use toml_edit::{DocumentMut, Formatted, Value};

const CHANGELOG_NAME: &str = "CHANGELOG.md";

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
    tags: Vec<(String, String)>, // pair = (sha, tag)
}

impl Update {
    pub fn new(options: UpdateOptions) -> Result<Self> {
        let metadata = MetadataCommand::new().current_dir(&options.path).no_deps().exec()?;
        let repo = Repository::init(metadata.workspace_root.clone().into_std_path_buf())?;
        let config = Config::parse(&metadata.workspace_root.as_std_path().join(DEFAULT_CONFIG))?;
        let tags = repo
            .tags(&config.git.tag_pattern, config.git.topo_order.unwrap_or(false))?
            .into_iter()
            .collect::<Vec<_>>();
        Ok(Self { options, metadata, repo, config, tags })
    }

    pub fn run(self) -> Result<()> {
        let packages = self.get_packages();
        for package in &packages {
            self.generate_changelog_for_package(package)?;
        }
        self.update_cargo_toml_version_for_workspace(&packages)?;
        for package in &packages {
            self.update_cargo_toml_version_for_package(package.manifest_path.as_std_path())?;
        }

        Ok(())
    }

    fn get_packages(&self) -> Vec<&Package> {
        // `publish.is_none()` means `publish = true`.
        self.metadata.workspace_packages().into_iter().filter(|p| p.publish.is_none()).collect()
    }

    fn version(&self) -> String {
        self.options.version.to_string()
    }

    /// Regenerate the changelogs. Please change main.rs to use this.
    #[allow(unused)]
    pub fn regenerate_changelogs(&self) -> Result<()> {
        for package in self.get_packages() {
            let package_path = package.manifest_path.as_std_path().parent().unwrap();
            let mut releases = vec![];
            for pair in self.tags.windows(2) {
                // pair = (sha, tag)
                let from = &pair[0];
                let to = &pair[1];
                let commits_range = format!("{}..{}", from.1, to.1);
                let commits = self.get_commits_for_package(package_path, commits_range)?;
                let release = self.get_release(commits, &to.1, Some(&to.0));
                releases.push(release);
            }
            let changelog = Changelog::new(releases, &self.config)?;
            let changelog_path = package_path.join(CHANGELOG_NAME);
            let mut out = File::create(&changelog_path)?;
            changelog.generate(&mut out)?;
        }
        Ok(())
    }

    fn generate_changelog_for_package(&self, package: &Package) -> Result<()> {
        let package_path = package.manifest_path.as_std_path().parent().unwrap();
        let last_tag = self.tags.last().context("Last commit not found")?.0.clone();
        let commits_range = format!("{}..HEAD", last_tag);
        let commits = self.get_commits_for_package(package_path, commits_range)?;
        let tag = format!("v{}", self.version());
        let release = self.get_release(commits, &tag, None);
        let changelog = Changelog::new(vec![release], &self.config)?;
        self.save_changelog(package_path, changelog)?;
        Ok(())
    }

    fn get_commits_for_package(
        &self,
        package_path: &Path,
        commits_range: String,
    ) -> Result<Vec<Commit>> {
        let include_path = package_path
            .strip_prefix(self.metadata.workspace_root.as_std_path())
            .unwrap()
            .to_string_lossy();
        let include_path = glob::Pattern::new(&format!("{include_path}/**"))?;
        let commits = self
            .repo
            .commits(Some(commits_range), Some(vec![include_path]), None)?
            .iter()
            .map(Commit::from)
            .collect::<Vec<_>>();
        Ok(commits)
    }

    fn get_release<'a>(
        &self,
        commits: Vec<Commit<'a>>,
        tag: &str,
        sha: Option<&str>,
    ) -> Release<'a> {
        let tag = tag.trim_start_matches("crates_").to_string();
        let timestamp = sha.map_or_else(
            || SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
            |sha| self.repo.find_commit(sha.to_string()).unwrap().time().seconds(),
        );
        Release { version: Some(tag), commits, commit_id: None, timestamp, previous: None }
    }

    fn save_changelog(&self, package_path: &Path, changelog: Changelog) -> Result<()> {
        let changelog_path = package_path.join(CHANGELOG_NAME);
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
