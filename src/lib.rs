mod changelog;
mod publish;

use bpaf::Bpaf;

pub use self::{
    changelog::{changelog_options, Changelog, ChangelogOptions},
    publish::{publish_options, Publish, PublishOptions},
};

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options("release-oxc"))]
pub enum ReleaseCommand {
    #[bpaf(command)]
    Changelog(#[bpaf(external(changelog_options))] ChangelogOptions),

    #[bpaf(command)]
    Publish(#[bpaf(external(publish_options))] PublishOptions),
}
