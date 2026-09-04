use std::path::Path;

use anyhow::Context as _;

use crate::ai::agent_tasks::AgentConfigSnapshot;

/// Strict file-based agent configuration. Unknown keys are rejected.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentConfigSnapshotFile {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub environment_id: Option<String>,
    #[serde(default)]
    pub runner_id: Option<String>,
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub base_prompt: Option<String>,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub computer_use_enabled: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct LoadedAgentConfigSnapshotFile {
    pub file: AgentConfigSnapshotFile,
}

#[cfg(not(target_family = "wasm"))]
pub fn load_config_file(path: &Path) -> anyhow::Result<LoadedAgentConfigSnapshotFile> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file '{}'", path.display()))?;

    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);
    let file = match extension.as_deref() {
        Some("json") => parse_json(&contents)
            .with_context(|| format!("Invalid JSON in config file '{}'", path.display()))?,
        Some("yml") | Some("yaml") => parse_yaml(&contents)
            .with_context(|| format!("Invalid YAML in config file '{}'", path.display()))?,
        _ => parse_json(&contents)
            .or_else(|_| parse_yaml(&contents))
            .with_context(|| {
                format!(
                    "Failed to parse config file '{}' as JSON or YAML",
                    path.display()
                )
            })?,
    };

    Ok(LoadedAgentConfigSnapshotFile { file })
}

#[cfg(target_family = "wasm")]
pub fn load_config_file(_path: &Path) -> anyhow::Result<LoadedAgentConfigSnapshotFile> {
    Err(anyhow::anyhow!(
        "Config files are not supported in WASM builds"
    ))
}

fn parse_json(input: &str) -> anyhow::Result<AgentConfigSnapshotFile> {
    serde_json::from_str(input).with_context(supported_keys_context)
}

fn parse_yaml(input: &str) -> anyhow::Result<AgentConfigSnapshotFile> {
    serde_yaml::from_str(input).with_context(supported_keys_context)
}

fn supported_keys_context() -> String {
    "Supported keys: name, environment_id, runner_id, model_id, base_prompt, host, computer_use_enabled"
        .to_owned()
}

/// Merge file configuration with CLI values, with CLI taking precedence.
pub fn merge_with_precedence(
    file: Option<&LoadedAgentConfigSnapshotFile>,
    cli: AgentConfigSnapshot,
) -> AgentConfigSnapshot {
    let default_file = AgentConfigSnapshotFile::default();
    let file = file.map(|loaded| &loaded.file).unwrap_or(&default_file);

    AgentConfigSnapshot {
        name: cli.name.or_else(|| file.name.clone()),
        environment_id: cli.environment_id.or_else(|| file.environment_id.clone()),
        runner_id: cli.runner_id.or_else(|| file.runner_id.clone()),
        model_id: cli.model_id.or_else(|| file.model_id.clone()),
        base_prompt: cli.base_prompt.or_else(|| file.base_prompt.clone()),
        profile_id: None,
        worker_host: cli.worker_host.or_else(|| file.host.clone()),
        skill_spec: cli.skill_spec,
        computer_use_enabled: cli.computer_use_enabled.or(file.computer_use_enabled),
        harness: cli.harness,
        harness_auth_secrets: cli.harness_auth_secrets,
        additional_source_repos: None,
    }
}
