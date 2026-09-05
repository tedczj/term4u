use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkflowId(String);

impl WorkflowId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for WorkflowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A command workflow loaded from local user, project, or bundled configuration.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct Workflow {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub arguments: Vec<Argument>,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub author_url: Option<String>,
    #[serde(default)]
    pub shells: Vec<warp_workflows::Shell>,
    #[serde(default, deserialize_with = "deserialize_optional_legacy_id")]
    pub environment_variables: Option<String>,
}

impl Workflow {
    pub fn new(name: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            command: command.into(),
            tags: Vec::new(),
            description: None,
            arguments: Vec::new(),
            source_url: None,
            author: None,
            author_url: None,
            shells: Vec::new(),
            environment_variables: None,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn content(&self) -> &str {
        &self.command
    }

    pub fn command(&self) -> Option<&str> {
        Some(&self.command)
    }

    pub fn description(&self) -> Option<&String> {
        self.description.as_ref()
    }

    pub fn arguments(&self) -> &Vec<Argument> {
        &self.arguments
    }

    pub fn tags(&self) -> Option<&Vec<String>> {
        Some(&self.tags)
    }

    pub fn source_url(&self) -> Option<&String> {
        self.source_url.as_ref()
    }

    pub fn author_name(&self) -> Option<&String> {
        self.author.as_ref()
    }

    pub fn shells(&self) -> Option<&Vec<warp_workflows::Shell>> {
        Some(&self.shells)
    }

    pub fn is_command_workflow(&self) -> bool {
        true
    }

    pub fn name_starts_with_char_ignore_case(&self, character: char) -> bool {
        self.name
            .chars()
            .next()
            .is_some_and(|first| first.eq_ignore_ascii_case(&character))
    }

    pub fn with_arguments(mut self, arguments: Vec<Argument>) -> Self {
        self.arguments = arguments;
        self
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn set_name(&mut self, name: &str) {
        name.clone_into(&mut self.name);
    }
}

impl From<warp_workflows::Workflow> for Workflow {
    fn from(workflow: warp_workflows::Workflow) -> Self {
        Self {
            name: workflow.name,
            command: workflow.command,
            description: workflow.description,
            arguments: workflow.arguments.into_iter().map(Argument::from).collect(),
            tags: workflow.tags,
            source_url: workflow.source_url,
            author: workflow.author,
            author_url: workflow.author_url,
            shells: workflow.shells,
            environment_variables: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Hash, Default)]
pub struct Argument {
    pub name: String,
    #[serde(flatten, deserialize_with = "deserialize_arg_type")]
    pub arg_type: ArgumentType,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub default_value: Option<String>,
}

impl From<warp_workflows::Argument> for Argument {
    fn from(argument: warp_workflows::Argument) -> Self {
        Self {
            name: argument.name,
            arg_type: ArgumentType::Text,
            description: argument.description,
            default_value: argument.default_value,
        }
    }
}

impl Argument {
    pub fn new(name: impl Into<String>, arg_type: ArgumentType) -> Self {
        Self {
            name: name.into(),
            arg_type,
            description: None,
            default_value: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.default_value = Some(default.into());
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &Option<String> {
        &self.description
    }

    pub fn arg_type(&self) -> &ArgumentType {
        &self.arg_type
    }

    pub fn default_value(&self) -> &Option<String> {
        &self.default_value
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Hash, Default)]
#[serde(tag = "arg_type")]
pub enum ArgumentType {
    #[default]
    Text,
    Enum { enum_id: String },
}

fn deserialize_arg_type<'de, D>(deserializer: D) -> Result<ArgumentType, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value.get("arg_type").and_then(Value::as_str) {
        Some("Enum") => value
            .get("enum_id")
            .map(stable_legacy_id)
            .map(|enum_id| ArgumentType::Enum { enum_id })
            .ok_or_else(|| serde::de::Error::missing_field("enum_id")),
        Some("Text") | None => Ok(ArgumentType::Text),
        Some(_) => Ok(ArgumentType::Text),
    }
}

fn deserialize_optional_legacy_id<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<Value>::deserialize(deserializer).map(|value| value.map(|value| stable_legacy_id(&value)))
}

fn stable_legacy_id(value: &Value) -> String {
    value
        .as_str()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| value.to_string())
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
