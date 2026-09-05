use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NotebookId(String);

impl NotebookId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn from_legacy_id(id: i32) -> Self {
        Self(format!("legacy-{id}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for NotebookId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notebook {
    pub id: NotebookId,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub data: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct LegacySerializedNotebook {
    #[serde(default)]
    pub data: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum NotebookLocation {
    LocalStore,
    LocalFile,
}
