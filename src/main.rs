use std::process::ExitCode;

use release_oxc::{release_command, Changelog, Publish, ReleaseCommand};

fn main() -> ExitCode {
    let command = release_command().fallback_to_usage().run();
    match command {
        ReleaseCommand::Changelog(options) => Changelog::new(options).run(),
        ReleaseCommand::Publish(options) => Publish::new(options).run(),
    }
}
