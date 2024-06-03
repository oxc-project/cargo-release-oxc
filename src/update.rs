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
use git_cmd::Repo as GitCommand;
use semver::Version;
use toml_edit::{DocumentMut, Formatted, Value};

const CHANGELOG_NAME: &str = "CHANGELOG.md";

#[derive(Debug, Clone, Bpaf)]
pub struct Options {
    #[bpaf(long, argument::<String>("NAME"))]
    name: String,

    #[bpaf(positional("PATH"), fallback(PathBuf::from(".")))]
    path: PathBuf,
}

pub struct Update {
    metadata: Metadata,
    repo: Repository,
    git_command: GitCommand,
    config: Config,
    tags: Vec<GitTag>,
    current_version: Version,
    /// Name used to prefix a tag, e.g. `crates_v1.0.0`
    name: String,
}

#[derive(Debug, Clone)]
struct GitTag {
    version: Version,
    sha: String,
}

impl GitTag {
    /// # Errors
    fn new((sha, tag): (String, String)) -> Result<Self> {
        let version = if tag.contains('v') {
            tag.split_once('v')
                .with_context(|| format!("tag {tag} does not have a `v`"))?
                .1
                .to_string()
        } else {
            tag
        };
        let version = Version::parse(&version)
            .with_context(|| format!("version {version} should be semver"))?;
        Ok(Self { version, sha })
    }
}

impl Update {
    /// # Errors
    pub fn new(options: Options) -> Result<Self> {
        let metadata = MetadataCommand::new().current_dir(&options.path).no_deps().exec()?;
        let repo = Repository::init(metadata.workspace_root.as_std_path().to_owned())?;
        let git_command = GitCommand::new(&metadata.workspace_root)?;
        let config = Config::parse(&metadata.workspace_root.as_std_path().join(DEFAULT_CONFIG))?;
        let tag_pattern = &config.git.tag_pattern;
        let tags = repo
            .tags(tag_pattern, config.git.topo_order.unwrap_or(false))?
            .into_iter()
            .map(GitTag::new)
            .collect::<Result<Vec<_>>>()?;
        let current_tag = tags
            .last()
            .ok_or_else(|| anyhow::anyhow!("Tags should not be empty for {tag_pattern:?}"))?;
        let current_version = current_tag.version.clone();
        Ok(Self { metadata, repo, git_command, config, tags, current_version, name: options.name })
    }

    /// # Errors
    pub fn run(self) -> Result<()> {
        self.git_command.is_clean()?;

        let packages = self.get_packages();

        let next_version = self.calculate_next_version(&packages)?;

        for package in &packages {
            self.generate_changelog_for_package(package, &next_version)?;
        }

        self.update_cargo_toml_version_for_workspace(&packages, &next_version)?;
        for package in &packages {
            Self::update_cargo_toml_version_for_package(
                package.manifest_path.as_std_path(),
                &next_version,
            )?;
        }

        println!("{next_version}");

        Ok(())
    }

    fn get_packages(&self) -> Vec<&Package> {
        // `publish.is_none()` means `publish = true`.
        self.metadata.workspace_packages().into_iter().filter(|p| p.publish.is_none()).collect()
    }

    fn calculate_next_version(&self, packages: &[&Package]) -> Result<String> {
        let commits_range = format!("{}_v{}..HEAD", &self.name, self.current_version);
        let include_paths = packages
            .iter()
            .map(|package| -> Result<glob::Pattern> { self.get_include_pattern(package) })
            .collect::<Result<Vec<_>>>()?;
        let commits = self
            .repo
            .commits(Some(commits_range), Some(include_paths), None)?
            .iter()
            .map(Commit::from)
            .collect::<Vec<_>>();
        let previous = Release {
            version: Some(self.current_version.to_string()),
            commits: vec![],
            commit_id: None,
            timestamp: 0,
            previous: None,
        };
        let release = Release {
            version: None,
            commits,
            commit_id: None,
            timestamp: 0,
            previous: Some(Box::new(previous)),
        };
        let mut changelog = Changelog::new(vec![release], &self.config)?;
        let next_version =
            changelog.bump_version().context("bump failed")?.context("bump failed")?;
        Ok(next_version)
    }

    fn generate_changelog_for_package(&self, package: &Package, next_version: &str) -> Result<()> {
        let package_path = package.manifest_path.as_std_path();
        let package_path = package_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Failed to get {package_path:?} parent"))?;
        let commits_range = format!("{}_v{}..HEAD", &self.name, self.current_version);
        let commits = self.get_commits_for_package(package, commits_range)?;
        let release = self.get_release(commits, next_version, None)?;
        let changelog = Changelog::new(vec![release], &self.config)?;
        Self::save_changelog(package_path, &changelog)?;
        Ok(())
    }

    fn get_commits_for_package(
        &self,
        package: &Package,
        commits_range: String,
    ) -> Result<Vec<Commit>> {
        let include_path = self.get_include_pattern(package)?;
        let commits = self
            .repo
            .commits(Some(commits_range), Some(vec![include_path]), None)?
            .iter()
            .map(Commit::from)
            .collect::<Vec<_>>();
        Ok(commits)
    }

    #[allow(clippy::unwrap_used)]
    fn package_dir(package: &Package) -> PathBuf {
        package.manifest_path.as_std_path().parent().unwrap().to_path_buf()
    }

    fn get_include_pattern(&self, package: &Package) -> Result<glob::Pattern> {
        let path = Self::package_dir(package);
        let include_path =
            path.strip_prefix(self.metadata.workspace_root.as_std_path())?.to_string_lossy();
        glob::Pattern::new(&format!("{include_path}/**")).context("pattern failed")
    }

    #[allow(clippy::cast_possible_wrap)]
    fn get_release<'a>(
        &self,
        commits: Vec<Commit<'a>>,
        next_version: &str,
        sha: Option<&str>,
    ) -> Result<Release<'a>> {
        let timestamp = match sha {
            None => SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64,
            Some(sha) => self
                .repo
                .find_commit(sha.to_string())
                .ok_or_else(|| anyhow::anyhow!("Cannot find commit {sha}"))?
                .time()
                .seconds(),
        };
        Ok(Release {
            version: Some(next_version.to_string()),
            commits,
            commit_id: None,
            timestamp,
            previous: None,
        })
    }

    fn save_changelog(package_path: &Path, changelog: &Changelog) -> Result<()> {
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

    fn update_cargo_toml_version_for_workspace(
        &self,
        packages: &[&Package],
        next_version: &str,
    ) -> Result<()> {
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
                    *version = Value::String(Formatted::new(next_version.to_string()));
                }
            }
        })
    }

    fn update_cargo_toml_version_for_package(
        manifest_path: &Path,
        next_version: &str,
    ) -> Result<()> {
        Self::update_toml(manifest_path, |manifest| {
            let Some(version) = manifest
                .get_mut("package")
                .and_then(|item| item.as_table_mut())
                .and_then(|table| table.get_mut("version"))
                .and_then(|item| item.as_value_mut())
            else {
                return;
            };
            *version = Value::String(Formatted::new(next_version.to_string()));
        })
    }

    /// # Errors
    pub fn regenerate_changelogs(&self) -> Result<()> {
        for package in self.get_packages() {
            let package_dir = Self::package_dir(package);
            let mut releases = vec![];
            for pair in self.tags.windows(2) {
                let from = &pair[0];
                let to = &pair[1];
                let commits_range = format!("{}..{}", from.sha, to.sha);
                let commits = self.get_commits_for_package(package, commits_range)?;
                let release = self.get_release(commits, &to.version.to_string(), Some(&to.sha))?;
                releases.push(release);
            }
            let changelog = Changelog::new(releases, &self.config)?;
            let changelog_path = package_dir.join(CHANGELOG_NAME);
            let mut out = File::create(&changelog_path)?;
            changelog.generate(&mut out)?;
        }
        Ok(())
    }
}
