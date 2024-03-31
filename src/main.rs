use std::process::ExitCode;

use release_oxc::{release_options, Releaser};

fn main() -> ExitCode {
    let options = release_options().fallback_to_usage().run();
    Releaser::new(options).run()
}
