use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use bpaf::Bpaf;
use git_cliff_core::{
    changelog::Changelog, commit::Commit, config::Config, release::Release, repo::Repository,
    DEFAULT_CONFIG,
};

use crate::config::{ReleaseConfig, ReleaseSet, VersionedPackage};

const CHANGELOG_NAME: &str = "CHANGELOG.md";

#[derive(Debug, Clone, Bpaf)]
pub struct Options {
    #[bpaf(long, argument::<String>("NAME"))]
    release: String,

    #[bpaf(positional("PATH"), fallback_with(crate::current_dir))]
    path: PathBuf,
}

pub struct Update {
    cwd: PathBuf,
    release_set: ReleaseSet,
    git_cliff_repo: Repository,
    git_cliff_config: Config,
    tags: Vec<GitTag>,
    current_version: String,
}

#[derive(Debug, Clone)]
struct GitTag {
    version: String,
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
        Ok(Self { version, sha })
    }
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
            .tags(&Some(tag_pattern.clone()), /* topo_order */ false)?
            .into_iter()
            .map(GitTag::new)
            .collect::<Result<Vec<_>>>()?;
        let current_tag = tags
            .last()
            .ok_or_else(|| anyhow::anyhow!("Tags should not be empty for {tag_pattern:?}"))?;
        let current_version = current_tag.version.clone();
        Ok(Self { cwd, release_set, git_cliff_repo, git_cliff_config, tags, current_version })
    }

    pub fn run(self) -> Result<()> {
        let release_set = &self.release_set;
        let next_version = self.calculate_next_version(release_set)?;
        for package in release_set.versioned_packages() {
            self.generate_changelog_for_package(&release_set, &package, &next_version)?;
        }
        release_set.update_version(&next_version)?;
        self.generate_changelog_for_release(release_set, &next_version)?;
        println!("{next_version}");
        Ok(())
    }

    fn commits_range(&self, release_set: &ReleaseSet) -> String {
        format!("{}_v{}..HEAD", &release_set.name, self.current_version)
    }

    fn calculate_next_version(&self, release_set: &ReleaseSet) -> Result<String> {
        let commits_range = self.commits_range(release_set);
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

    fn generate_changelog_for_package(
        &self,
        release_set: &ReleaseSet,
        package: &VersionedPackage,
        next_version: &str,
    ) -> Result<()> {
        let commits_range = self.commits_range(release_set);
        let commits = self.get_commits_for_package(package, commits_range)?;
        let release = self.get_release(commits, next_version, None)?;
        let changelog = Changelog::new(vec![release], &self.git_cliff_config)?;
        Self::save_changelog(&package.dir, &changelog)?;
        Ok(())
    }

    fn generate_changelog_for_release(
        &self,
        release_set: &ReleaseSet,
        next_version: &str,
    ) -> Result<()> {
        let commits_range = self.commits_range(release_set);
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
        let release = self.get_release(commits, next_version, None)?;
        let changelog = Changelog::new(vec![release], &self.git_cliff_config)?;
        let mut s = vec![];
        changelog.generate(&mut s).context("failed to generate changelog")?;
        println!("{}", String::from_utf8(s).unwrap());
        Ok(())
    }

    pub fn regenerate_changelogs(&self) -> Result<()> {
        for package in self.release_set.versioned_packages() {
            let mut releases = vec![];
            for pair in self.tags.windows(2) {
                let from = &pair[0];
                let to = &pair[1];
                let commits_range = format!("{}..{}", from.sha, to.sha);
                let commits = self.get_commits_for_package(&package, commits_range)?;
                let release = self.get_release(commits, &to.version.to_string(), Some(&to.sha))?;
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
