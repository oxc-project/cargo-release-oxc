mod cargo_command;
mod publish;
mod update;

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
