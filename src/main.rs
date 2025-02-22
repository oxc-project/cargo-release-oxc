use anyhow::Result;

use cargo_release_oxc::{Publish, ReleaseCommand, Update, release_command};

fn main() -> Result<()> {
    let command = release_command().fallback_to_usage().run();
    match command {
        ReleaseCommand::Update(options) => Update::new(options)?.run(),
        ReleaseCommand::Changelog(options) => {
            Update::new(options)?.changelog_for_release().map(|_| ())
        }
        ReleaseCommand::RegenerateChangelogs(options) => {
            Update::new(options)?.regenerate_changelogs()
        }
        ReleaseCommand::Publish(options) => Publish::new(options)?.run(),
    }
}
