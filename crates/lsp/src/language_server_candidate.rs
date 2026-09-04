use std::path::Path;

use async_trait::async_trait;

use crate::CommandBuilder;

/// Defines local discovery for a supported language server.
#[async_trait]
pub trait LanguageServerCandidate: Send + Sync {
    /// Heuristic to determine if this server is relevant for the repository at `path`.
    async fn should_suggest_for_repo(&self, path: &Path, executor: &CommandBuilder) -> bool;

    /// Checks whether the server binary is available and working on the system PATH.
    async fn is_installed_on_path(&self, executor: &CommandBuilder) -> bool;

    async fn is_installed(&self, executor: &CommandBuilder) -> bool {
        self.is_installed_on_path(executor).await
    }
}
