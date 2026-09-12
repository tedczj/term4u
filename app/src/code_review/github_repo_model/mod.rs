use warpui::{AppContext, Entity, ModelContext, ModelHandle};

#[cfg(feature = "local_fs")]
mod local;
#[cfg(feature = "local_fs")]
pub use local::LocalGitHubRepoModel;

use crate::util::git::PrInfo;

#[cfg_attr(not(feature = "local_fs"), allow(dead_code))]
#[derive(Debug)]
pub enum GitHubRepoEvent {
    /// Emitted when `pr_info` changes value (fetch result differs from
    /// cached, branch change cleared the cache, etc.).
    PrInfoChanged,
    /// Emitted when `repository_info` changes value.
    RepositoryInfoChanged,
}

// ── Unified GitHubRepoModel (local or remote backend) ───────────────────────

/// Unified per-repo GitHub-info model that dispatches to a local or remote
/// backend, mirroring [`crate::code_review::git_repo_model::GitRepoStatusModel`].
///
/// Consumers (prompt chips, code review, agent context) hold a
/// `ModelHandle<GitHubRepoModel>` and subscribe to its [`GitHubRepoEvent`]s
/// without caring whether the repository is local or on an SSH host.
pub enum GitHubRepoModel {
    #[cfg(feature = "local_fs")]
    Local(ModelHandle<LocalGitHubRepoModel>),
}
impl Entity for GitHubRepoModel {
    type Event = GitHubRepoEvent;
}
impl GitHubRepoModel {
    /// Re-emit a sub-model event so subscribers of the unified model observe
    /// the same `GitHubRepoEvent`s regardless of backend.
    pub(crate) fn forward_event(&mut self, event: &GitHubRepoEvent, ctx: &mut ModelContext<Self>) {
        match event {
            GitHubRepoEvent::PrInfoChanged => ctx.emit(GitHubRepoEvent::PrInfoChanged),
            GitHubRepoEvent::RepositoryInfoChanged => {
                ctx.emit(GitHubRepoEvent::RepositoryInfoChanged)
            }
        }
    }

    /// PR info for the current branch.
    pub fn pr_info<'a>(&self, ctx: &'a AppContext) -> Option<&'a PrInfo> {
        match self {
            #[cfg(feature = "local_fs")]
            Self::Local(model) => model.as_ref(ctx).pr_info(),
        }
    }

    /// Whether a `gh pr view` fetch is currently in flight.
    pub fn is_refreshing_pr_info(&self, ctx: &AppContext) -> bool {
        match self {
            #[cfg(feature = "local_fs")]
            Self::Local(model) => model.as_ref(ctx).is_refreshing_pr_info(),
        }
    }

    /// Force a PR info refresh (e.g. after a `gh`/`gt` command completes).
    pub fn refresh_pr_info(&self, ctx: &mut ModelContext<Self>) {
        match self {
            #[cfg(feature = "local_fs")]
            Self::Local(model) => model.update(ctx, |model, ctx| model.refresh_pr_info(ctx)),
        }
    }
}

#[cfg(all(test, feature = "local_fs"))]
impl GitHubRepoModel {}
