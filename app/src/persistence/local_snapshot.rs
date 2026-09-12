use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use diesel::prelude::*;
use diesel::sql_types::Text;
use serde::{Deserialize, Serialize};
use serde_bytes_repr::{ByteFmtDeserializer, ByteFmtSerializer};

use crate::app_state::{AppState, PaneUuid, WindowSnapshot};
use crate::terminal::model::SerializedBlockListItem;

#[derive(Serialize, Deserialize)]
struct LocalSnapshot {
    windows: Vec<WindowSnapshot>,
    active_window_index: Option<usize>,
    block_lists: Vec<(PaneUuid, Vec<SerializedBlockListItem>)>,
}

#[derive(QueryableByName)]
struct SnapshotRow {
    #[diesel(sql_type = Text)]
    snapshot: String,
}

pub(super) fn save(connection: &mut SqliteConnection, state: AppState) -> Result<()> {
    if let Some(previous) =
        diesel::sql_query("SELECT snapshot FROM local_app_snapshots WHERE id = 1")
            .get_result::<SnapshotRow>(connection)
            .optional()?
        && decode(&previous.snapshot).is_err()
    {
        anyhow::bail!("Unreadable local session snapshot was preserved for recovery");
    }
    let snapshot = LocalSnapshot {
        windows: state.windows,
        active_window_index: state.active_window_index,
        block_lists: state
            .block_lists
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    };
    let mut data = Vec::new();
    snapshot.serialize(ByteFmtSerializer::base64(
        &mut serde_json::Serializer::new(&mut data),
        base64::alphabet::STANDARD,
        base64::engine::GeneralPurposeConfig::new(),
    ))?;
    diesel::sql_query("INSERT INTO local_app_snapshots (id, snapshot) VALUES (1, ?) ON CONFLICT(id) DO UPDATE SET snapshot = excluded.snapshot")
        .bind::<Text, _>(String::from_utf8(data)?)
        .execute(connection)?;
    Ok(())
}

pub(super) fn load(connection: &mut SqliteConnection) -> QueryResult<Option<AppState>> {
    let Some(row) = diesel::sql_query("SELECT snapshot FROM local_app_snapshots WHERE id = 1")
        .get_result::<SnapshotRow>(connection)
        .optional()?
    else {
        #[cfg(target_os = "macos")]
        return super::legacy_snapshot::load(connection);
        #[cfg(not(target_os = "macos"))]
        return Ok(None);
    };
    match decode(&row.snapshot) {
        Ok(snapshot) => Ok(Some(AppState {
            windows: snapshot.windows,
            active_window_index: snapshot.active_window_index,
            block_lists: Arc::new(snapshot.block_lists.into_iter().collect::<HashMap<_, _>>()),
        })),
        Err(_) => {
            log::warn!("Skipping unreadable local session snapshot");
            Ok(None)
        }
    }
}

fn decode(json: &str) -> serde_json::Result<LocalSnapshot> {
    let mut deserializer = serde_json::Deserializer::from_str(json);
    LocalSnapshot::deserialize(ByteFmtDeserializer::new_base64(
        &mut deserializer,
        base64::alphabet::STANDARD,
        base64::engine::GeneralPurposeConfig::new(),
    ))
}

#[cfg(test)]
#[path = "local_snapshot_tests.rs"]
mod tests;
