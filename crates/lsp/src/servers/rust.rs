use std::path::Path;

use async_trait::async_trait;

use crate::CommandBuilder;
use crate::language_server_candidate::LanguageServerCandidate;

pub struct RustAnalyzerCandidate;

#[async_trait]
impl LanguageServerCandidate for RustAnalyzerCandidate {
    async fn should_suggest_for_repo(&self, path: &Path, _executor: &CommandBuilder) -> bool {
        path.join("Cargo.toml").exists()
    }

    async fn is_installed_on_path(&self, executor: &CommandBuilder) -> bool {
        executor
            .command("rust-analyzer")
            .arg("--help")
            .output()
            .await
            .is_ok_and(|output| output.status.success())
    }
}
