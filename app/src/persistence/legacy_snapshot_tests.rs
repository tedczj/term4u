use diesel::connection::SimpleConnection;
use diesel_migrations::MigrationHarness;

use super::*;
use crate::persistence::local_snapshot;

fn database(bytes: &[u8]) -> (tempfile::TempDir, SqliteConnection) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("legacy.sqlite");
    std::fs::write(&path, bytes).unwrap();
    let mut connection = SqliteConnection::establish(path.to_str().unwrap()).unwrap();
    connection
        .run_pending_migrations(persistence::MIGRATIONS)
        .unwrap();
    (directory, connection)
}

#[test]
fn legacy_terminal_tabs_and_active_tab_are_restored_without_rewriting_old_rows() {
    let (_directory, mut connection) = database(include_bytes!(
        "../../../crates/persistence/fixtures/legacy/three_tabs.sqlite"
    ));
    let state = local_snapshot::load(&mut connection).unwrap().unwrap();
    assert_eq!(state.windows.len(), 1);
    assert_eq!(state.windows[0].tabs.len(), 3);
    assert_eq!(state.windows[0].active_tab_index, 2);
    assert_eq!(
        schema::tabs::table
            .count()
            .get_result::<i64>(&mut connection)
            .unwrap(),
        3
    );
    let snapshots = diesel::sql_query("SELECT count(*) AS count FROM local_app_snapshots")
        .get_result::<RowCount>(&mut connection)
        .unwrap();
    assert_eq!(snapshots.count, 0);
}

#[derive(QueryableByName)]
struct RowCount {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

#[test]
fn damaged_legacy_pane_does_not_discard_other_tabs_or_shift_active_tab() {
    let (_directory, mut connection) = database(include_bytes!(
        "../../../crates/persistence/fixtures/legacy/three_tabs.sqlite"
    ));
    connection
        .batch_execute("DELETE FROM terminal_panes WHERE id = 1;")
        .unwrap();
    let state = local_snapshot::load(&mut connection).unwrap().unwrap();
    assert_eq!(state.windows[0].tabs.len(), 2);
    assert_eq!(state.windows[0].active_tab_index, 1);
    assert_eq!(
        schema::tabs::table
            .count()
            .get_result::<i64>(&mut connection)
            .unwrap(),
        3
    );
}

#[test]
fn legacy_notebook_pane_uses_its_local_body_id() {
    let (_directory, mut connection) = database(include_bytes!(
        "../../../crates/persistence/fixtures/legacy/restored_notebooks.sqlite"
    ));
    let state = local_snapshot::load(&mut connection).unwrap().unwrap();
    let PaneNodeSnapshot::Branch(root) = &state.windows[0].tabs[0].root else {
        panic!("fixture contains a split pane");
    };
    assert_eq!(root.children.len(), 2);
    let PaneNodeSnapshot::Leaf(leaf) = &root.children[0].1 else {
        panic!("first restored pane should be the notebook");
    };
    assert_eq!(
        leaf.contents,
        LeafContents::Notebook(NotebookPaneSnapshot::LocalNotebook {
            notebook_id: Some(NotebookId::from_legacy_id(12)),
        })
    );
}

#[test]
fn legacy_workflow_pane_loads_the_local_workflow_body() {
    let (_directory, mut connection) = database(include_bytes!(
        "../../../crates/persistence/fixtures/legacy/restored_workflows.sqlite"
    ));
    let state = local_snapshot::load(&mut connection).unwrap().unwrap();
    let PaneNodeSnapshot::Branch(root) = &state.windows[0].tabs[0].root else {
        panic!("fixture contains a split pane");
    };
    assert_eq!(root.children.len(), 1);
    let PaneNodeSnapshot::Leaf(leaf) = &root.children[0].1 else {
        panic!("restored pane should be the workflow");
    };
    let LeafContents::Workflow(WorkflowPaneSnapshot::LocalWorkflow {
        workflow_id,
        workflow,
    }) = &leaf.contents
    else {
        panic!("workflow body must be available locally");
    };
    assert_eq!(workflow_id, &WorkflowId::from_legacy_id(136));
    assert_eq!(workflow.name, "My Workflow");
    assert_eq!(workflow.command, "echo hello");
    assert!(workflow.arguments.is_empty());
}

#[test]
fn legacy_terminal_output_bytes_survive_import_and_new_snapshot() {
    let (_directory, mut connection) = database(include_bytes!(
        "../../../crates/persistence/fixtures/legacy/restored_blocks.sqlite"
    ));
    let mut original_outputs = schema::blocks::table
        .select(schema::blocks::stylized_output)
        .load::<Vec<u8>>(&mut connection)
        .unwrap();
    let state = local_snapshot::load(&mut connection).unwrap().unwrap();
    let mut outputs: Vec<_> = state
        .block_lists
        .values()
        .flatten()
        .map(|item| {
            let crate::terminal::model::SerializedBlockListItem::Command { block } = item;
            block.stylized_output.clone()
        })
        .collect();
    assert_eq!(outputs.len(), 5);
    original_outputs.sort();
    outputs.sort();
    assert_eq!(outputs, original_outputs);
    local_snapshot::save(&mut connection, state.clone()).unwrap();
    assert_eq!(local_snapshot::load(&mut connection).unwrap(), Some(state));
    assert_eq!(
        schema::blocks::table
            .count()
            .get_result::<i64>(&mut connection)
            .unwrap(),
        5
    );
}

#[test]
fn newer_snapshot_wins_over_legacy_window_rows() {
    let (_directory, mut connection) = database(include_bytes!(
        "../../../crates/persistence/fixtures/legacy/three_tabs.sqlite"
    ));
    let mut expected = local_snapshot::load(&mut connection).unwrap().unwrap();
    expected.windows[0].tabs[0].custom_title = Some("Edited locally".to_owned());
    local_snapshot::save(&mut connection, expected.clone()).unwrap();
    assert_eq!(
        local_snapshot::load(&mut connection).unwrap(),
        Some(expected)
    );
    assert_eq!(
        schema::tabs::table
            .count()
            .get_result::<i64>(&mut connection)
            .unwrap(),
        3
    );
}

#[test]
fn malformed_new_snapshot_does_not_fall_back_to_stale_legacy_windows() {
    let (_directory, mut connection) = database(include_bytes!(
        "../../../crates/persistence/fixtures/legacy/three_tabs.sqlite"
    ));
    connection
        .batch_execute("INSERT INTO local_app_snapshots (id, snapshot) VALUES (1, '{invalid');")
        .unwrap();
    assert!(local_snapshot::load(&mut connection).unwrap().is_none());
}
