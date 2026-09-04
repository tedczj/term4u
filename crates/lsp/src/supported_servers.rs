#[cfg(not(target_arch = "wasm32"))]
use command::r#async::Command;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[cfg(not(target_arch = "wasm32"))]
use crate::CommandBuilder;
use crate::servers::clangd::ClangdCandidate;
use crate::servers::go::GoPlsCandidate;
use crate::servers::pyright::PyrightCandidate;
use crate::servers::rust::RustAnalyzerCandidate;
use crate::servers::typescript_language_server::TypeScriptLanguageServerCandidate;
use crate::{LanguageId, LanguageServerCandidate};

/// Represents the language servers supported through local PATH discovery.
///
/// This is persisted in SQLite, so existing variants must not be renamed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter)]
pub enum LSPServerType {
    RustAnalyzer,
    GoPls,
    Pyright,
    TypeScriptLanguageServer,
    Clangd,
}

impl LSPServerType {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn create_command(&self, executor: &CommandBuilder) -> Command {
        let mut command = executor.command(self.binary_name());
        command.args(self.args());
        command
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn is_working_on_path(&self, executor: &CommandBuilder) -> bool {
        self.candidate().is_installed_on_path(executor).await
    }

    pub fn binary_name(&self) -> &'static str {
        match self {
            LSPServerType::RustAnalyzer => "rust-analyzer",
            LSPServerType::GoPls => "gopls",
            LSPServerType::Pyright => "pyright-langserver",
            LSPServerType::TypeScriptLanguageServer => "typescript-language-server",
            LSPServerType::Clangd => "clangd",
        }
    }

    pub fn manual_install_message(&self) -> String {
        format!(
            "{} is not installed. Install it manually and make sure it is available on PATH.",
            self.binary_name()
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn args(&self) -> &'static [&'static str] {
        match self {
            LSPServerType::RustAnalyzer | LSPServerType::GoPls | LSPServerType::Clangd => &[],
            LSPServerType::Pyright | LSPServerType::TypeScriptLanguageServer => &["--stdio"],
        }
    }

    pub fn languages(&self) -> Vec<LanguageId> {
        match self {
            LSPServerType::RustAnalyzer => vec![LanguageId::Rust],
            LSPServerType::GoPls => vec![LanguageId::Go],
            LSPServerType::Pyright => vec![LanguageId::Python],
            LSPServerType::TypeScriptLanguageServer => vec![
                LanguageId::TypeScript,
                LanguageId::TypeScriptReact,
                LanguageId::JavaScript,
                LanguageId::JavaScriptReact,
            ],
            LSPServerType::Clangd => vec![LanguageId::C, LanguageId::Cpp],
        }
    }

    pub fn language_name(&self) -> String {
        match self {
            LSPServerType::TypeScriptLanguageServer => "TypeScript/JavaScript".to_string(),
            LSPServerType::RustAnalyzer
            | LSPServerType::GoPls
            | LSPServerType::Pyright
            | LSPServerType::Clangd => self
                .languages()
                .iter()
                .map(|language| {
                    let id = language.lsp_language_identifier();
                    let mut chars = id.chars();
                    chars.next().map_or_else(String::new, |first| {
                        first.to_uppercase().collect::<String>() + chars.as_str()
                    })
                })
                .join("/"),
        }
    }

    pub fn candidate(&self) -> Box<dyn LanguageServerCandidate> {
        match self {
            LSPServerType::RustAnalyzer => Box::new(RustAnalyzerCandidate),
            LSPServerType::GoPls => Box::new(GoPlsCandidate),
            LSPServerType::Pyright => Box::new(PyrightCandidate),
            LSPServerType::TypeScriptLanguageServer => Box::new(TypeScriptLanguageServerCandidate),
            LSPServerType::Clangd => Box::new(ClangdCandidate),
        }
    }

    pub fn all() -> impl Iterator<Item = LSPServerType> {
        LSPServerType::iter()
    }
}

#[cfg(test)]
#[path = "supported_servers_tests.rs"]
mod tests;
