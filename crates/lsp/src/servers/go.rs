use std::path::Path;

use async_trait::async_trait;

use crate::CommandBuilder;
use crate::language_server_candidate::LanguageServerCandidate;

pub struct GoPlsCandidate;

#[async_trait]
impl LanguageServerCandidate for GoPlsCandidate {
    async fn should_suggest_for_repo(&self, path: &Path, executor: &CommandBuilder) -> bool {
        if !path.join("go.mod").exists() {
            return false;
        }

        executor
            .command("go")
            .arg("version")
            .output()
            .await
            .is_ok_and(|output| output.status.success())
    }

    async fn is_installed_on_path(&self, executor: &CommandBuilder) -> bool {
        executor
            .command("gopls")
            .arg("version")
            .output()
            .await
            .is_ok_and(|output| output.status.success())
    }
}
