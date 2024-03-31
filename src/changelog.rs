use std::process::ExitCode;

use bpaf::Bpaf;

#[derive(Debug, Clone, Bpaf)]
pub struct ChangelogOptions {}

#[allow(unused)]
pub struct Changelog {
    options: ChangelogOptions,
}

impl Changelog {
    pub fn new(options: ChangelogOptions) -> Self {
        Self { options }
    }

    #[must_use]
    pub fn run(self) -> ExitCode {
        ExitCode::SUCCESS
    }
}
