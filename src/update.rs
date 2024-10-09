use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use git_cliff_core::{
    changelog::Changelog, commit::Commit, config::Config, release::Release, repo::Repository,
    DEFAULT_CONFIG,
};

use crate::{
    config::{ReleaseConfig, ReleaseSet, VersionedPackage},
    Options,
};

const CHANGELOG_NAME: &str = "CHANGELOG.md";

#[derive(Debug, Clone)]
struct GitTag {
    version: String,
    sha: String,
}

impl GitTag {
    fn new(sha: String, tag: String) -> Result<Self> {
        let version = if tag.contains('v') {
            tag.split_once('v')
                .with_context(|| format!("tag {tag} does not have a `v`"))?
                .1
                .to_string()
        } else {
            tag
        };
        Ok(Self { version, sha })
    }
}

pub struct Update {
    cwd: PathBuf,
    release_set: ReleaseSet,
    git_cliff_repo: Repository,
    git_cliff_config: Config,
    tags: Vec<GitTag>,
    current_version: String,
}

impl Update {
    pub fn new(options: Options) -> Result<Self> {
        let cwd = options.path;

        super::check_git_clean(&cwd)?;

        let release_set = ReleaseConfig::new(&cwd)?.get_release(&options.release)?;

        let git_cliff_repo = Repository::init(cwd.clone())?;
        let git_cliff_config = Config::parse(&cwd.join(DEFAULT_CONFIG))?;
        let tag_pattern = regex::Regex::new(&format!("^{}_v[0-9]*", options.release))
            .context("failed to make regex")?;
        let tags = git_cliff_repo
            .tags(
                &Some(tag_pattern.clone()),
                /* topo_order */ false,
                /* include only the tags that belong to the current branch. */ false,
            )?
            .into_iter()
            .map(|(sha, tag)| GitTag::new(sha, tag.name))
            .collect::<Result<Vec<_>>>()?;
        let current_tag = tags
            .last()
            .ok_or_else(|| anyhow::anyhow!("Tags should not be empty for {tag_pattern:?}"))?;
        let current_version = current_tag.version.clone();
        Ok(Self { cwd, release_set, git_cliff_repo, git_cliff_config, tags, current_version })
    }

    pub fn run(&self) -> Result<()> {
        let next_version = self.changelog_for_release()?;
        for package in self.release_set.versioned_packages() {
            self.generate_changelog_for_package(&package, &next_version)?;
        }
        self.release_set.update_version(&next_version)?;
        Ok(())
    }

    pub fn changelog_for_release(&self) -> Result<String> {
        let next_version = self.calculate_next_version()?;
        self.print_changelog_for_release(&next_version)?;
        fs::write("./target/OXC_VERSION", &next_version)?;
        Ok(next_version)
    }

    fn calculate_next_version(&self) -> Result<String> {
        let mut commits = self
            .get_commits_for_release()?
            .into_iter()
            .filter_map(|c| c.into_conventional().ok())
            .collect::<Vec<_>>();
        // Only matching scopes can participate in braking change detection.
        if let Some(scopes) = &self.release_set.scopes_for_breaking_change {
            commits = commits
                .into_iter()
                .filter(|commit| {
                    let Some(commit) = commit.conv.as_ref() else { return false };
                    let Some(scope) = commit.scope() else { return false };
                    scopes.iter().any(|s| scope.contains(s))
                })
                .collect::<Vec<_>>();
        }

        let previous =
            Release { version: Some(self.current_version.to_string()), ..Release::default() };
        let release = Release { commits, previous: Some(Box::new(previous)), ..Release::default() };
        let mut changelog = Changelog::new(vec![release], &self.git_cliff_config)?;
        let next_version =
            changelog.bump_version().context("bump failed")?.context("bump failed")?;
        Ok(next_version)
    }

    fn get_commits_for_package(
        &self,
        package: &VersionedPackage,
        commits_range: &str,
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
    fn get_git_cliff_release<'a>(
        &self,
        commits: Vec<Commit<'a>>,
        next_version: &str,
        sha: Option<&str>,
    ) -> Result<Release<'a>> {
        let timestamp = match sha {
            None => SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64,
            Some(sha) => self
                .git_cliff_repo
                .find_commit(sha)
                .ok_or_else(|| anyhow::anyhow!("Cannot find commit {sha}"))?
                .time()
                .seconds(),
        };
        Ok(Release {
            version: Some(next_version.to_string()),
            commits,
            timestamp,
            ..Release::default()
        })
    }

    fn save_changelog(package_path: &Path, changelog: &Changelog) -> Result<()> {
        let changelog_path = package_path.join(CHANGELOG_NAME);
        let prev_changelog_string = fs::read_to_string(&changelog_path).unwrap_or_default();
        let mut out = File::create(&changelog_path)?;
        changelog.prepend(prev_changelog_string, &mut out)?;
        Ok(())
    }

    fn generate_changelog_for_package(
        &self,
        package: &VersionedPackage,
        next_version: &str,
    ) -> Result<()> {
        let commits_range = self.release_set.commits_range(&self.current_version);
        let commits = self.get_commits_for_package(package, &commits_range)?;
        let release = self.get_git_cliff_release(commits, next_version, None)?;
        let changelog = Changelog::new(vec![release], &self.git_cliff_config)?;
        Self::save_changelog(&package.dir, &changelog)?;
        Ok(())
    }

    fn get_commits_for_release(&self) -> Result<Vec<Commit<'_>>> {
        let release_set = &self.release_set;
        let commits_range = release_set.commits_range(&self.current_version);
        let include_paths = release_set
            .versioned_packages()
            .iter()
            .map(|package| self.get_include_pattern(package))
            .collect::<Result<Vec<_>>>()?;
        let commits = self
            .git_cliff_repo
            .commits(Some(&commits_range), Some(include_paths), None)?
            .iter()
            .map(Commit::from)
            .collect::<Vec<_>>();
        Ok(commits)
    }

    fn print_changelog_for_release(&self, next_version: &str) -> Result<()> {
        let commits = self.get_commits_for_release()?;
        let release = self.get_git_cliff_release(commits, next_version, None)?;
        let mut git_cliff_config = self.git_cliff_config.clone();
        git_cliff_config.changelog.header = None;
        let changelog = Changelog::new(vec![release], &git_cliff_config)?;
        let mut s = vec![];
        changelog.generate(&mut s).context("failed to generate changelog")?;
        fs::write("./target/OXC_CHANGELOG", String::from_utf8(s).unwrap())?;
        Ok(())
    }

    pub fn regenerate_changelogs(&self) -> Result<()> {
        for package in self.release_set.versioned_packages() {
            let mut releases = vec![];
            for pair in self.tags.windows(2) {
                let from = &pair[0];
                let to = &pair[1];
                let commits_range = format!("{}..{}", from.sha, to.sha);
                let commits = self.get_commits_for_package(&package, &commits_range)?;
                let release =
                    self.get_git_cliff_release(commits, &to.version.to_string(), Some(&to.sha))?;
                releases.push(release);
            }
            let changelog = Changelog::new(releases, &self.git_cliff_config)?;
            let changelog_path = package.dir.join(CHANGELOG_NAME);
            let mut out = File::create(&changelog_path)?;
            changelog.generate(&mut out)?;
        }
        Ok(())
    }
}
