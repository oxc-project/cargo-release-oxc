use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use bpaf::Bpaf;
use git_cliff_core::{
    changelog::Changelog, commit::Commit, config::Config, release::Release, repo::Repository,
    DEFAULT_CONFIG,
};
use semver::Version;

use crate::config::{ReleaseConfig, ReleaseSet, VersionedPackage};

const CHANGELOG_NAME: &str = "CHANGELOG.md";

#[derive(Debug, Clone, Bpaf)]
pub struct Options {
    #[bpaf(long, argument::<String>("NAME"))]
    release: String,

    #[bpaf(positional("PATH"), fallback(PathBuf::from(".")))]
    path: PathBuf,
}

pub struct Update {
    cwd: PathBuf,
    release_name: String,
    release_config: ReleaseConfig,
    git_cliff_repo: Repository,
    git_cliff_config: Config,
    tags: Vec<GitTag>,
    current_version: Version,
}

#[derive(Debug, Clone)]
struct GitTag {
    version: Version,
    sha: String,
}

impl GitTag {
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

fn check_git_clean(path: &Path) -> Result<()> {
    let git_status = Command::new("git")
        .current_dir(path)
        .stdout(Stdio::null())
        .args(["diff", "--exit-code"])
        .status();
    if !git_status.is_ok_and(|s| s.success()) {
        anyhow::bail!("Uncommitted changes found, please check `git status`.")
    }
    Ok(())
}

impl Update {
    pub fn new(options: Options) -> Result<Self> {
        let cwd = options.path;

        check_git_clean(&cwd)?;

        let config = ReleaseConfig::new(&cwd)?;
        let git_cliff_repo = Repository::init(cwd.clone())?;
        let git_cliff_config = Config::parse(&cwd.join(DEFAULT_CONFIG))?;
        let tag_pattern = &git_cliff_config.git.tag_pattern;
        let tags = git_cliff_repo
            .tags(tag_pattern, git_cliff_config.git.topo_order.unwrap_or(false))?
            .into_iter()
            .map(GitTag::new)
            .collect::<Result<Vec<_>>>()?;
        let current_tag = tags
            .last()
            .ok_or_else(|| anyhow::anyhow!("Tags should not be empty for {tag_pattern:?}"))?;
        let current_version = current_tag.version.clone();
        Ok(Self {
            cwd,
            release_name: options.release,
            release_config: config,
            git_cliff_repo,
            git_cliff_config,
            tags,
            current_version,
        })
    }

    pub fn run(self) -> Result<()> {
        let Some(release_set) =
            self.release_config.release_sets.iter().find(|r| r.name == self.release_name)
        else {
            anyhow::bail!("release {} not found", self.release_name);
        };
        let next_version = self.calculate_next_version(release_set)?;
        for package in release_set.versioned_packages() {
            self.generate_changelog_for_package(&release_set.name, &package, &next_version)?;
        }
        release_set.update_version(&next_version)?;
        println!("{next_version}");
        Ok(())
    }

    fn calculate_next_version(&self, release_set: &ReleaseSet) -> Result<String> {
        let commits_range = format!("{}_v{}..HEAD", &release_set.name, self.current_version);
        let include_paths = release_set
            .versioned_packages()
            .iter()
            .map(|package| self.get_include_pattern(package))
            .collect::<Result<Vec<_>>>()?;
        let commits = self
            .git_cliff_repo
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
        let mut changelog = Changelog::new(vec![release], &self.git_cliff_config)?;
        let next_version =
            changelog.bump_version().context("bump failed")?.context("bump failed")?;
        Ok(next_version)
    }

    fn generate_changelog_for_package(
        &self,
        name: &str,
        package: &VersionedPackage,
        next_version: &str,
    ) -> Result<()> {
        let commits_range = format!("{}_v{}..HEAD", name, self.current_version);
        let commits = self.get_commits_for_package(package, commits_range)?;
        let release = self.get_release(commits, next_version, None)?;
        let changelog = Changelog::new(vec![release], &self.git_cliff_config)?;
        Self::save_changelog(&package.dir, &changelog)?;
        Ok(())
    }

    fn get_commits_for_package(
        &self,
        package: &VersionedPackage,
        commits_range: String,
    ) -> Result<Vec<Commit>> {
        let include_path = self.get_include_pattern(package)?;
        let commits = self
            .git_cliff_repo
            .commits(Some(commits_range), Some(vec![include_path]), None)?
            .iter()
            .map(Commit::from)
            .collect::<Vec<_>>();
        Ok(commits)
    }

    fn get_include_pattern(&self, package: &VersionedPackage) -> Result<glob::Pattern> {
        let path = &package.dir;
        let include_path = path.strip_prefix(&self.cwd)?.to_string_lossy();
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
                .git_cliff_repo
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

    pub fn regenerate_changelogs(&self) -> Result<()> {
        for release_set in &self.release_config.release_sets {
            for package in release_set.versioned_packages() {
                let mut releases = vec![];
                for pair in self.tags.windows(2) {
                    let from = &pair[0];
                    let to = &pair[1];
                    let commits_range = format!("{}..{}", from.sha, to.sha);
                    let commits = self.get_commits_for_package(&package, commits_range)?;
                    let release =
                        self.get_release(commits, &to.version.to_string(), Some(&to.sha))?;
                    releases.push(release);
                }
                let changelog = Changelog::new(releases, &self.git_cliff_config)?;
                let changelog_path = package.dir.join(CHANGELOG_NAME);
                let mut out = File::create(&changelog_path)?;
                changelog.generate(&mut out)?;
            }
        }
        Ok(())
    }
}
