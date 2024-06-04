mod cargo_command;
mod config;
mod publish;
mod update;
mod versioning;

use std::{
    path::Path,
    process::{Command, Stdio},
};

use anyhow::Result;
use bpaf::Bpaf;

pub use self::{
    publish::{options as publish_options, Options as PublishOptions, Publish},
    update::{options as update_options, Options as UpdateOptions, Update},
};

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options("release-oxc"))]
pub enum ReleaseCommand {
    /// Generate CHANGELOG.md and bump versions for all published crates
    #[bpaf(command)]
    Update(#[bpaf(external(update_options))] UpdateOptions),

    /// Regenerate CHANGELOG.md.
    #[bpaf(command)]
    RegenerateChangelogs(#[bpaf(external(update_options))] UpdateOptions),

    #[bpaf(command)]
    Publish(#[bpaf(external(publish_options))] PublishOptions),
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
