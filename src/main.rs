use anyhow::Result;

use cargo_release_oxc::{
    Options, Publish, ReleaseCommand, Update, check_git_clean, release_command,
};

fn main() -> Result<()> {
    let command = release_command().fallback_to_usage().run();
    match command {
        ReleaseCommand::Update(options) => update(&options),
        ReleaseCommand::Changelog(options) => changelog(&options),
        ReleaseCommand::RegenerateChangelogs(options) => regenerate_changelogs(&options),
        ReleaseCommand::Publish(options) => publish(&options),
    }
}

fn update(options: &Options) -> Result<()> {
    let cwd = &options.path;
    check_git_clean(cwd)?;
    for release_name in &options.release {
        Update::new(cwd, release_name)?.run()?;
    }
    Ok(())
}

fn changelog(options: &Options) -> Result<()> {
    for release_name in &options.release {
        Update::new(&options.path, release_name)?.changelog_for_release()?;
    }
    Ok(())
}

fn regenerate_changelogs(options: &Options) -> Result<()> {
    for release_name in &options.release {
        Update::new(&options.path, release_name)?.regenerate_changelogs()?;
    }
    Ok(())
}

fn publish(options: &Options) -> Result<()> {
    let cwd = &options.path;
    check_git_clean(cwd)?;
    for release_name in &options.release {
        Publish::new(cwd, release_name, options.dry_run)?.run()?;
    }
    Ok(())
}
