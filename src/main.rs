use anyhow::Result;

use cargo_release_oxc::{release_command, Publish, ReleaseCommand, Update};

fn main() -> Result<()> {
    let command = release_command().fallback_to_usage().run();
    match command {
        ReleaseCommand::Update(options) => Update::new(options)?.run(),
        ReleaseCommand::RegenerateChangelogs(options) => {
            Update::new(options)?.regenerate_changelogs()
        }
        ReleaseCommand::Publish(options) => Publish::new(&options)?.run(),
    }
}
