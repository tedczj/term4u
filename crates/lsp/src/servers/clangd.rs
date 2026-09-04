use std::path::Path;

use async_trait::async_trait;

use crate::CommandBuilder;
use crate::language_server_candidate::LanguageServerCandidate;

pub struct ClangdCandidate;

fn is_c_or_cpp_extension(extension: &str) -> bool {
    matches!(
        extension,
        "c" | "C" | "cc" | "cpp" | "cxx" | "h" | "hh" | "hpp" | "hxx" | "H"
    )
}

#[async_trait]
impl LanguageServerCandidate for ClangdCandidate {
    async fn should_suggest_for_repo(&self, path: &Path, _executor: &CommandBuilder) -> bool {
        let repo_markers = [
            "compile_commands.json",
            "compile_flags.txt",
            ".clangd",
            "CMakeLists.txt",
        ];

        if repo_markers.iter().any(|marker| path.join(marker).exists()) {
            return true;
        }

        std::fs::read_dir(path).is_ok_and(|entries| {
            entries.flatten().any(|entry| {
                let file_path = entry.path();
                file_path.is_file()
                    && file_path
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(is_c_or_cpp_extension)
            })
        })
    }

    async fn is_installed_on_path(&self, executor: &CommandBuilder) -> bool {
        executor
            .command("clangd")
            .arg("--version")
            .output()
            .await
            .is_ok_and(|output| output.status.success())
    }
}
