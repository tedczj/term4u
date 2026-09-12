//! Persists local terminal blocks without decoding historical Agent rows.

use std::collections::HashMap;

use diesel::prelude::*;
use diesel::result::Error;
use diesel::sqlite::SqliteConnection;

use super::{model, schema};
use crate::app_state::PaneUuid;
use crate::terminal::model::SerializedBlockListItem;
use crate::terminal::model::block::SerializedBlock;

const MAX_TERMINAL_BLOCKS_TO_PERSIST_PER_SESSION: i64 = 100;

type PersistedBlocks = HashMap<PaneUuid, Vec<SerializedBlockListItem>>;

/// Returns the most recent terminal blocks for each session in chronological order.
pub(super) fn get_all_restored_blocks(
    conn: &mut SqliteConnection,
) -> Result<PersistedBlocks, Error> {
    let terminal_sessions = schema::terminal_panes::table
        .select(model::TerminalSession::as_select())
        .load::<model::TerminalSession>(conn)?;
    let block_lists = model::Block::belonging_to(&terminal_sessions)
        .select(model::Block::as_select())
        .order_by(schema::blocks::columns::id.asc())
        .load::<model::Block>(conn)?
        .grouped_by(&terminal_sessions);

    let mut blocks_by_pane = block_lists
        .into_iter()
        .zip(terminal_sessions)
        .map(|(blocks, pane)| {
            (
                PaneUuid(pane.uuid),
                blocks.into_iter().map(Into::into).collect(),
            )
        })
        .collect::<HashMap<_, Vec<_>>>();

    for blocks in blocks_by_pane.values_mut() {
        blocks.sort_by_key(SerializedBlockListItem::start_ts);
        blocks.drain(
            0..blocks
                .len()
                .saturating_sub(MAX_TERMINAL_BLOCKS_TO_PERSIST_PER_SESSION as usize),
        );
    }

    Ok(blocks_by_pane)
}

pub(super) fn save_block(
    conn: &mut SqliteConnection,
    pane_id: Vec<u8>,
    block: &SerializedBlock,
    is_local_block: bool,
) -> Result<(), Error> {
    use schema::blocks::dsl::*;

    conn.transaction(|conn| {
        let saved_blocks_count = blocks
            .filter(pane_leaf_uuid.eq(pane_id.clone()))
            .filter(id.is_not_null())
            .filter(is_background.ne(true))
            .count()
            .first::<i64>(conn)?;
        let excess = saved_blocks_count - MAX_TERMINAL_BLOCKS_TO_PERSIST_PER_SESSION + 1;
        if excess > 0 {
            let first_kept_id = blocks
                .filter(pane_leaf_uuid.eq(pane_id.clone()))
                .filter(id.is_not_null())
                .filter(is_background.ne(true))
                .select(id)
                .order(id.asc())
                .offset(excess)
                .first::<Option<i32>>(conn)?;
            if let Some(first_kept_id) = first_kept_id {
                diesel::delete(
                    blocks
                        .filter(id.lt(first_kept_id))
                        .filter(pane_leaf_uuid.eq(pane_id.clone())),
                )
                .execute(conn)?;
            }
        }

        diesel::insert_into(blocks)
            .values(new_block(pane_id, block, is_local_block))
            .execute(conn)?;
        Ok(())
    })
}

fn new_block<'a>(
    pane_leaf_uuid: Vec<u8>,
    block: &'a SerializedBlock,
    is_local: bool,
) -> model::NewBlock<'a> {
    model::NewBlock {
        block_id: block.id.as_str(),
        pane_leaf_uuid,
        stylized_command: &block.stylized_command,
        stylized_output: &block.stylized_output,
        pwd: block.pwd.as_ref(),
        git_branch: block.git_head.as_ref(),
        git_branch_name: block.git_branch_name.as_ref(),
        virtual_env: block.virtual_env.as_ref(),
        conda_env: block.conda_env.as_ref(),
        exit_code: block.exit_code.value(),
        did_execute: block.did_execute,
        is_background: block.is_background,
        completed_ts: block.completed_ts.map(|timestamp| timestamp.naive_utc()),
        start_ts: block.start_ts.map(|timestamp| timestamp.naive_utc()),
        ps1: block.ps1.as_ref(),
        rprompt: block.rprompt.as_ref(),
        honor_ps1: block.honor_ps1,
        shell: block.shell_host.as_ref().map(|host| host.shell_type.name()),
        user: block.shell_host.as_ref().map(|host| host.user.as_str()),
        host: block.shell_host.as_ref().map(|host| host.hostname.as_str()),
        prompt_snapshot: block.prompt_snapshot.as_ref(),
        ai_metadata: block.removed_feature_metadata.as_ref(),
        is_local: Some(is_local),
        agent_view_visibility: block
            .removed_feature_visibility
            .as_ref()
            .and_then(|visibility| serde_json::to_string(visibility).ok()),
    }
}

pub(super) fn delete_blocks(conn: &mut SqliteConnection, pane_id: Vec<u8>) -> Result<(), Error> {
    diesel::delete(schema::blocks::table.filter(schema::blocks::pane_leaf_uuid.eq(pane_id)))
        .execute(conn)?;
    Ok(())
}

#[cfg(test)]
#[path = "block_list_tests.rs"]
mod tests;
