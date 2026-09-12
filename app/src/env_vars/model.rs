use serde::{Deserialize, Serialize};

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
