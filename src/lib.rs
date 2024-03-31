use std::process::ExitCode;

use bpaf::Bpaf;

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options("release-oxc"))]
pub struct ReleaseOptions {}

pub struct Releaser {
    options: ReleaseOptions,
}

impl Releaser {
    pub fn new(options: ReleaseOptions) -> Self {
        Self { options }
    }

    #[must_use]
    pub fn run(self) -> ExitCode {
        ExitCode::SUCCESS
    }
}
