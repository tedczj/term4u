use chrono::{Local, TimeZone};
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel_migrations::MigrationHarness;

use super::{delete_blocks, get_all_restored_blocks, save_block};
use crate::app_state::PaneUuid;
use crate::persistence::schema;
use crate::terminal::model::SerializedBlockListItem;
use crate::terminal::model::block::SerializedBlock;

fn test_connection() -> SqliteConnection {
    let mut conn = SqliteConnection::establish(":memory:").unwrap();
    conn.run_pending_migrations(::persistence::MIGRATIONS)
        .unwrap();
    conn.batch_execute(
        "PRAGMA foreign_keys = ON;
         INSERT INTO windows (id, active_tab_index) VALUES (1, 0);
         INSERT INTO tabs (id, window_id) VALUES (1, 1);
         INSERT INTO pane_nodes (id, tab_id, is_leaf) VALUES (1, 1, TRUE), (2, 1, TRUE);
         INSERT INTO pane_leaves (pane_node_id, kind, is_focused)
             VALUES (1, 'terminal', TRUE), (2, 'terminal', FALSE);
         INSERT INTO terminal_panes (id, uuid, is_active)
             VALUES (1, X'01', TRUE), (2, X'02', FALSE);",
    )
    .unwrap();
    conn
}

fn block(id: &str, second: i64) -> SerializedBlock {
    SerializedBlock {
        id: id.to_owned().into(),
        stylized_command: b"printf hello".to_vec(),
        stylized_output: b"hello".to_vec(),
        pwd: Some("/tmp/local-project".to_owned()),
        git_head: Some("abc123".to_owned()),
        git_branch_name: Some("main".to_owned()),
        start_ts: Some(Local.timestamp_opt(1_700_000_000 + second, 0).unwrap()),
        completed_ts: Some(Local.timestamp_opt(1_700_000_001 + second, 0).unwrap()),
        did_execute: true,
        is_local: Some(true),
        ..Default::default()
    }
}

#[test]
fn terminal_blocks_round_trip_in_chronological_order() {
    let mut conn = test_connection();
    let older = block("older", 0);
    let newer = block("newer", 1);
    save_block(&mut conn, vec![1], &newer, true).unwrap();
    save_block(&mut conn, vec![1], &older, true).unwrap();

    let restored = get_all_restored_blocks(&mut conn).unwrap();

    assert_eq!(
        restored[&PaneUuid(vec![1])],
        vec![older.into(), newer.into()]
    );
    assert_eq!(restored[&PaneUuid(vec![2])], vec![]);
}

#[test]
fn terminal_block_limit_retains_newest_commands() {
    let mut conn = test_connection();
    for second in 0..102 {
        save_block(
            &mut conn,
            vec![1],
            &block(&format!("block-{second}"), second),
            true,
        )
        .unwrap();
    }

    let restored = get_all_restored_blocks(&mut conn).unwrap();
    let blocks = &restored[&PaneUuid(vec![1])];

    assert_eq!(blocks.len(), 100);
    assert_eq!(blocks.first(), Some(&block("block-2", 2).into()));
    assert_eq!(blocks.last(), Some(&block("block-101", 101).into()));
}

#[test]
fn deleting_terminal_blocks_is_scoped_to_the_pane() {
    let mut conn = test_connection();
    save_block(&mut conn, vec![1], &block("first-pane", 0), true).unwrap();
    let other = block("second-pane", 0);
    save_block(&mut conn, vec![2], &other, true).unwrap();

    delete_blocks(&mut conn, vec![1]).unwrap();

    let restored = get_all_restored_blocks(&mut conn).unwrap();
    assert_eq!(restored[&PaneUuid(vec![1])], vec![]);
    assert_eq!(restored[&PaneUuid(vec![2])], vec![other.into()]);
}

#[test]
fn terminal_persistence_leaves_legacy_agent_queries_opaque() {
    let mut conn = test_connection();
    conn.batch_execute(
        "INSERT INTO ai_queries (exchange_id, conversation_id, start_ts, input, output_status)
         VALUES ('legacy-exchange', 'unknown-conversation', '2023-01-01 00:00:00',
                 'not valid JSON', 'unknown-status');",
    )
    .unwrap();

    save_block(&mut conn, vec![1], &block("local-command", 0), true).unwrap();
    let restored = get_all_restored_blocks(&mut conn).unwrap();
    delete_blocks(&mut conn, vec![1]).unwrap();

    assert_eq!(restored[&PaneUuid(vec![1])].len(), 1);
    let legacy: Vec<(String, String)> = schema::ai_queries::table
        .select((schema::ai_queries::input, schema::ai_queries::output_status))
        .load(&mut conn)
        .unwrap();
    assert_eq!(
        legacy,
        vec![("not valid JSON".to_owned(), "unknown-status".to_owned())]
    );
}

#[test]
fn restoring_blocks_does_not_rewrite_unknown_legacy_columns() {
    let mut conn = test_connection();
    save_block(&mut conn, vec![1], &block("legacy-command", 0), true).unwrap();
    conn.batch_execute(
        "UPDATE blocks SET ai_metadata = 'opaque legacy metadata',
             agent_view_visibility = '{unknown legacy format';",
    )
    .unwrap();

    let restored = get_all_restored_blocks(&mut conn).unwrap();
    let SerializedBlockListItem::Command { block } = &restored[&PaneUuid(vec![1])][0];

    assert_eq!(block.stylized_output, b"hello");
    assert_eq!(
        block.removed_feature_metadata.as_deref(),
        Some("opaque legacy metadata")
    );
    let legacy: (Option<String>, Option<String>) = schema::blocks::table
        .select((
            schema::blocks::ai_metadata,
            schema::blocks::agent_view_visibility,
        ))
        .first(&mut conn)
        .unwrap();
    assert_eq!(
        legacy,
        (
            Some("opaque legacy metadata".to_owned()),
            Some("{unknown legacy format".to_owned())
        )
    );
}

#[test]
fn old_terminal_fixture_remains_readable_after_migrations() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("warp.sqlite");
    std::fs::write(
        &path,
        include_bytes!("../../../crates/integration/tests/data/restored_blocks.sqlite"),
    )
    .unwrap();
    let mut conn = SqliteConnection::establish(path.to_str().unwrap()).unwrap();
    conn.run_pending_migrations(::persistence::MIGRATIONS)
        .unwrap();
    let before: i64 = schema::blocks::table.count().get_result(&mut conn).unwrap();

    let restored = get_all_restored_blocks(&mut conn).unwrap();

    assert!(
        before > 0,
        "the historical fixture must contain terminal output"
    );
    assert!(restored.values().any(|blocks| !blocks.is_empty()));
    let after: i64 = schema::blocks::table.count().get_result(&mut conn).unwrap();
    assert_eq!(after, before);
}
