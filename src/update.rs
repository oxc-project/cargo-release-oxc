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

use crate::cargo_command::CargoCommand;

const CHANGELOG_NAME: &str = "CHANGELOG.md";
const TAG_PREFIX: &str = "crates_v";

#[derive(Debug, Clone, Bpaf)]
pub struct Options {
    #[bpaf(external(bump))]
    bump: Bump,

    #[bpaf(positional("PATH"), fallback(PathBuf::from(".")))]
    path: PathBuf,
}

#[derive(Debug, Clone, Bpaf)]
enum Bump {
    Major,
    Minor,
    Patch,
}

pub struct Update {
    metadata: Metadata,
    cargo: CargoCommand,
    repo: Repository,
    git_command: GitCommand,
    config: Config,
    tags: Vec<GitTag>,
    current_version: Version,
    next_version: Version,
}

#[derive(Debug, Clone)]
struct GitTag {
    version: Version,
    sha: String,
}

impl GitTag {
    /// # Errors
    fn new((sha, tag): (String, String)) -> Result<Self> {
        let version = tag
            .strip_prefix(TAG_PREFIX)
            .ok_or_else(|| anyhow::anyhow!("Tag {tag} should start with prefix {TAG_PREFIX}"))?;
        let version =
            Version::parse(version).with_context(|| format!("{version} should be semver"))?;
        Ok(Self { version, sha })
    }
}

impl Update {
    /// # Errors
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(options: Options) -> Result<Self> {
        let metadata = MetadataCommand::new().current_dir(&options.path).no_deps().exec()?;
        let cargo = CargoCommand::new(metadata.workspace_root.clone().into_std_path_buf());
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
        let next_version = Self::next_version(&current_tag.version, &options.bump);
        let current_version = current_tag.version.clone();
        Ok(Self { metadata, cargo, repo, git_command, config, tags, current_version, next_version })
    }

    /// # Errors
    pub fn run(self) -> Result<()> {
        self.git_command.is_clean()?;

        let packages = self.get_packages();

        self.update_cargo_toml_version_for_workspace(&packages)?;
        for package in &packages {
            self.update_cargo_toml_version_for_package(package.manifest_path.as_std_path())?;
        }
        self.cargo.check()?;

        for package in &packages {
            self.generate_changelog_for_package(package)?;
        }
        Ok(())
    }

    fn get_packages(&self) -> Vec<&Package> {
        // `publish.is_none()` means `publish = true`.
        self.metadata.workspace_packages().into_iter().filter(|p| p.publish.is_none()).collect()
    }

    fn next_version(version: &Version, bump: &Bump) -> Version {
        let mut version = version.clone();
        match bump {
            Bump::Patch => {
                version.patch += 1;
            }
            Bump::Minor => {
                version.minor += 1;
                version.patch = 0;
            }
            Bump::Major => {
                version.major += 1;
                version.minor += 0;
                version.patch += 0;
            }
        }
        version
    }

    /// # Errors
    pub fn regenerate_changelogs(&self) -> Result<()> {
        for package in self.get_packages() {
            let package_path = package.manifest_path.as_std_path();
            let package_path = package_path
                .parent()
                .ok_or_else(|| anyhow::anyhow!("Failed to get {package_path:?} parent"))?;
            let mut releases = vec![];
            for pair in self.tags.windows(2) {
                let from = &pair[0];
                let to = &pair[1];
                let commits_range = format!("{}..{}", from.sha, to.sha);
                let commits = self.get_commits_for_package(package_path, commits_range)?;
                let release = self.get_release(commits, &to.version, Some(&to.sha))?;
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
        let package_path = package.manifest_path.as_std_path();
        let package_path = package_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Failed to get {package_path:?} parent"))?;
        let commits_range = format!("{TAG_PREFIX}{}..HEAD", self.current_version);
        let commits = self.get_commits_for_package(package_path, commits_range)?;
        let release = self.get_release(commits, &self.next_version, None)?;
        let changelog = Changelog::new(vec![release], &self.config)?;
        Self::save_changelog(package_path, &changelog)?;
        Ok(())
    }

    fn get_commits_for_package(
        &self,
        package_path: &Path,
        commits_range: String,
    ) -> Result<Vec<Commit>> {
        let include_path = package_path
            .strip_prefix(self.metadata.workspace_root.as_std_path())?
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

    #[allow(clippy::cast_possible_wrap)]
    fn get_release<'a>(
        &self,
        commits: Vec<Commit<'a>>,
        version: &Version,
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
            version: Some(version.to_string()),
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
                    *version = Value::String(Formatted::new(self.next_version.to_string()));
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
            *version = Value::String(Formatted::new(self.next_version.to_string()));
        })
    }
}
