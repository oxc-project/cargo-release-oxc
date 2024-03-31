use std::process::ExitCode;

use bpaf::Bpaf;

#[derive(Debug, Clone, Bpaf)]
pub struct PublishOptions {}

#[allow(unused)]
pub struct Publish {
    options: PublishOptions,
}

impl Publish {
    pub fn new(options: PublishOptions) -> Self {
        Self { options }
    }

    #[must_use]
    pub fn run(self) -> ExitCode {
        ExitCode::SUCCESS
    }
}
