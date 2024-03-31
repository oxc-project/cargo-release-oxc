use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use bpaf::Bpaf;
use cargo_metadata::{Metadata, MetadataCommand, Package};
use git_cliff_core::{
    changelog::Changelog as GitCliffChangelog, commit::Commit, config::Config, release::Release,
    repo::Repository, DEFAULT_CONFIG,
};

#[derive(Debug, Clone, Bpaf)]
pub struct ChangelogOptions {
    #[bpaf(argument("tag"), guard(validate_tag, TAG_ERROR_MESSAGE))]
    tag: String,

    #[bpaf(positional("PATH"), fallback(PathBuf::from(".")))]
    path: PathBuf,
}

#[allow(clippy::ptr_arg)]
fn validate_tag(tag: &String) -> bool {
    tag.starts_with('v')
}

const TAG_ERROR_MESSAGE: &str = "Tag must starts with v";

#[allow(unused)]
pub struct Changelog {
    options: ChangelogOptions,
    metadata: Metadata,
    repo: Repository,
    config: Config,
    timestamp: i64,
}

impl Changelog {
    pub fn new(options: ChangelogOptions) -> Result<Self> {
        assert!(options.tag.starts_with('v'));
        let metadata = MetadataCommand::new().current_dir(&options.path).no_deps().exec()?;
        let root_path = metadata.workspace_root.clone().into_std_path_buf();
        let repo = Repository::init(root_path)?;
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        let config_path = metadata.workspace_root.as_std_path().join(DEFAULT_CONFIG);
        let config = Config::parse(&config_path)?;
        Ok(Self { options, metadata, repo, timestamp, config })
    }

    pub fn run(self) -> Result<()> {
        // `publish.is_none()` means `publish = true`.
        let packages = self.metadata.packages.iter().filter(|p| p.publish.is_none());
        for package in packages {
            self.generate_changelog_for_package(package)?;
        }
        Ok(())
    }

    fn generate_changelog_for_package(&self, package: &Package) -> Result<()> {
        let package_path = package.manifest_path.as_std_path().parent().unwrap();
        let release = Release {
            version: Some(self.options.tag.clone()),
            commits: self.get_commits_for_package(package_path)?,
            commit_id: None,
            timestamp: self.timestamp,
            previous: None,
        };
        let changelog = GitCliffChangelog::new(vec![release], &self.config)?;
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

    fn save_changelog(&self, package_path: &Path, changelog: GitCliffChangelog) -> Result<()> {
        let changelog_path = package_path.join("CHANGELOG.md");
        let prev_changelog_string = fs::read_to_string(&changelog_path).unwrap_or_default();
        let mut out = File::create(&changelog_path)?;
        changelog.prepend(prev_changelog_string, &mut out)?;
        Ok(())
    }
}
