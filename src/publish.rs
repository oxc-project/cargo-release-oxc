use anyhow::Result;
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

    pub fn run(self) -> Result<()> {
        Ok(())
    }
}
