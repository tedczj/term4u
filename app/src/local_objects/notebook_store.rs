use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use warp_core::safe_warn;
use warpui::{Entity, SingletonEntity};

use crate::notebooks::model::{LegacySerializedNotebook, Notebook, NotebookId};

pub struct NotebookStore {
    storage_dir: PathBuf,
    notebooks: HashMap<NotebookId, Notebook>,
}

impl NotebookStore {
    pub fn new(legacy_rows: Vec<(i32, Option<String>, Option<String>)>) -> Self {
        Self::load(warp_core::paths::data_dir().join("notebooks"), legacy_rows)
    }

    fn load(
        storage_dir: PathBuf,
        legacy_rows: Vec<(i32, Option<String>, Option<String>)>,
    ) -> Self {
        let mut notebooks = load_notebook_files(&storage_dir);
        for (id, title, serialized) in legacy_rows {
            let id = NotebookId::from_legacy_id(id);
            if notebooks.contains_key(&id) {
                continue;
            }
            let Some(serialized) = serialized else {
                continue;
            };
            match serde_json::from_str::<LegacySerializedNotebook>(&serialized) {
                Ok(legacy) => {
                    notebooks.insert(
                        id.clone(),
                        Notebook {
                            id,
                            title: title.unwrap_or_default(),
                            data: legacy.data,
                        },
                    );
                }
                Err(error) => safe_warn!(
                    safe: ("Skipping malformed legacy notebook"),
                    full: ("Skipping malformed legacy notebook {id}: {error}")
                ),
            }
        }
        Self {
            storage_dir,
            notebooks,
        }
    }

    pub fn all(&self) -> impl Iterator<Item = &Notebook> {
        self.notebooks.values()
    }

    pub fn get(&self, id: &NotebookId) -> Option<&Notebook> {
        self.notebooks.get(id)
    }

    pub fn create(&mut self, title: String) -> Result<NotebookId> {
        let id = NotebookId::new();
        self.upsert(Notebook {
            id: id.clone(),
            title,
            data: String::new(),
        })?;
        Ok(id)
    }

    pub fn upsert(&mut self, notebook: Notebook) -> Result<()> {
        write_notebook(&self.storage_dir, &notebook)?;
        self.notebooks.insert(notebook.id.clone(), notebook);
        Ok(())
    }
}

impl Entity for NotebookStore {
    type Event = ();
}

impl SingletonEntity for NotebookStore {}

fn load_notebook_files(storage_dir: &Path) -> HashMap<NotebookId, Notebook> {
    let Ok(entries) = fs::read_dir(storage_dir) else {
        return HashMap::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                return None;
            }
            match fs::read_to_string(&path)
                .ok()
                .and_then(|contents| serde_json::from_str::<Notebook>(&contents).ok())
            {
                Some(notebook) => Some((notebook.id.clone(), notebook)),
                None => {
                    safe_warn!(
                        safe: ("Skipping malformed local notebook"),
                        full: ("Skipping malformed local notebook at {}", path.display())
                    );
                    None
                }
            }
        })
        .collect()
}

fn write_notebook(storage_dir: &Path, notebook: &Notebook) -> Result<()> {
    fs::create_dir_all(storage_dir)
        .with_context(|| format!("failed to create notebook directory {}", storage_dir.display()))?;
    let path = storage_dir.join(format!("{}.json", notebook.id));
    let temporary = storage_dir.join(format!(".{}.tmp", notebook.id));
    let serialized = serde_json::to_vec_pretty(notebook)?;
    fs::write(&temporary, serialized)
        .with_context(|| format!("failed to write notebook {}", notebook.id))?;
    fs::rename(&temporary, &path)
        .with_context(|| format!("failed to commit notebook {}", notebook.id))?;
    Ok(())
}

#[cfg(test)]
#[path = "notebook_store_tests.rs"]
mod tests;
