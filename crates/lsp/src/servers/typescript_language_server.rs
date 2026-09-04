use std::path::Path;

use async_trait::async_trait;

use crate::CommandBuilder;
use crate::language_server_candidate::LanguageServerCandidate;

pub struct TypeScriptLanguageServerCandidate;

#[async_trait]
impl LanguageServerCandidate for TypeScriptLanguageServerCandidate {
    async fn should_suggest_for_repo(&self, path: &Path, _executor: &CommandBuilder) -> bool {
        path.join("package.json").exists()
            || path.join("tsconfig.json").exists()
            || path.join("jsconfig.json").exists()
    }

    async fn is_installed_on_path(&self, executor: &CommandBuilder) -> bool {
        executor
            .command("typescript-language-server")
            .arg("--version")
            .output()
            .await
            .is_ok_and(|output| output.status.success())
    }
}
