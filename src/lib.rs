mod cargo_command;
mod config;
mod publish;
mod update;
mod versioning;

use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::Result;
use bpaf::Bpaf;

pub use self::{publish::Publish, update::Update};

#[derive(Debug, Clone, Bpaf)]
pub struct Options {
    #[bpaf(long, argument::<String>("NAME"))]
    release: String,

    #[bpaf(positional("PATH"), fallback_with(crate::current_dir))]
    path: PathBuf,
}

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options("release-oxc"))]
pub enum ReleaseCommand {
    /// Generate CHANGELOG.md and bump versions for all published packages.
    #[bpaf(command)]
    Update(#[bpaf(external(options))] Options),

    /// Generate changelog summary.
    #[bpaf(command)]
    Changelog(#[bpaf(external(options))] Options),

    /// Regenerate CHANGELOG.md for all published packages.
    #[bpaf(command)]
    RegenerateChangelogs(#[bpaf(external(options))] Options),

    #[bpaf(command)]
    Publish(#[bpaf(external(options))] Options),
}

fn current_dir() -> Result<PathBuf, String> {
    std::env::current_dir().map_err(|err| format!("{err:?}"))
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
