use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct EnvVar {
    pub name: String,
    pub value: EnvVarValue,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum EnvVarValue {
    Constant(String),
    Command(EnvVarCommand),
}

impl Default for EnvVarValue {
    fn default() -> Self {
        Self::Constant(String::new())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EnvVarCommand {
    pub name: String,
    pub command: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct EnvVarCollection {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub vars: Vec<EnvVar>,
}
