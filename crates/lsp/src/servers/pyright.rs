use std::path::Path;

use async_trait::async_trait;

use crate::CommandBuilder;
use crate::language_server_candidate::LanguageServerCandidate;

pub struct PyrightCandidate;

#[async_trait]
impl LanguageServerCandidate for PyrightCandidate {
    async fn should_suggest_for_repo(&self, path: &Path, _executor: &CommandBuilder) -> bool {
        path.join("pyproject.toml").exists()
            || path.join("setup.py").exists()
            || path.join("requirements.txt").exists()
            || path.join("Pipfile").exists()
    }

    async fn is_installed_on_path(&self, executor: &CommandBuilder) -> bool {
        executor
            .command("pyright-langserver")
            .arg("--version")
            .output()
            .await
            .is_ok_and(|output| output.status.success())
    }
}
