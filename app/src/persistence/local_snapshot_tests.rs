use diesel_migrations::MigrationHarness;

use super::*;
use crate::app_state::{
    LeafContents, LeafSnapshot, PaneNodeSnapshot, TabSnapshot, TerminalPaneSnapshot,
};
use crate::terminal::model::block::SerializedBlock;

fn database() -> SqliteConnection {
    let mut connection = SqliteConnection::establish(":memory:").unwrap();
    connection
        .run_pending_migrations(persistence::MIGRATIONS)
        .unwrap();
    connection
}

fn snapshot() -> AppState {
    let uuid = vec![0, 127, 128, 255];
    let block =
        SerializedBlock::new_for_test("echo 中文".as_bytes().to_vec(), vec![0, 27, 128, 255]);
    AppState {
        windows: vec![WindowSnapshot {
            tabs: vec![TabSnapshot {
                custom_title: Some("本地终端".to_owned()),
                root: PaneNodeSnapshot::Leaf(Box::new(LeafSnapshot {
                    is_focused: true,
                    custom_vertical_tabs_title: None,
                    contents: LeafContents::Terminal(TerminalPaneSnapshot {
                        uuid: uuid.clone(),
                        cwd: Some("/tmp".to_owned()),
                        shell_launch_data: None,
                        is_active: true,
                    }),
                })),
                default_directory_color: None,
                selected_color: Default::default(),
                left_panel: None,
                group_id: None,
                pinned: false,
            }],
            active_tab_index: 0,
            bounds: Some(pathfinder_geometry::rect::RectF::new(
                pathfinder_geometry::vector::vec2f(30., 40.),
                pathfinder_geometry::vector::vec2f(800., 600.),
            )),
            fullscreen_state: warpui::platform::FullscreenState::Maximized,
            quake_mode: false,
            universal_search_width: None,
            voltron_width: None,
            left_panel_open: false,
            vertical_tabs_panel_open: false,
            left_panel_width: None,
            tab_groups: Vec::new(),
        }],
        active_window_index: Some(0),
        block_lists: Arc::new(HashMap::from([(PaneUuid(uuid), vec![block.into()])])),
    }
}

#[test]
fn snapshot_round_trip_preserves_layout_and_arbitrary_terminal_bytes() {
    let mut connection = database();
    assert!(load(&mut connection).unwrap().is_none());
    let expected = snapshot();
    save(&mut connection, expected.clone()).unwrap();
    let restored = load(&mut connection).unwrap().unwrap();
    assert_eq!(restored.windows, expected.windows);
    assert_eq!(restored.active_window_index, expected.active_window_index);
    assert_eq!(restored.block_lists, expected.block_lists);
}

#[test]
fn malformed_snapshot_is_skipped_and_cannot_be_overwritten() {
    let mut connection = database();
    let original = "{invalid snapshot";
    diesel::sql_query("INSERT INTO local_app_snapshots (id, snapshot) VALUES (1, ?)")
        .bind::<Text, _>(original)
        .execute(&mut connection)
        .unwrap();
    assert!(load(&mut connection).unwrap().is_none());
    assert!(save(&mut connection, snapshot()).is_err());
    let row = diesel::sql_query("SELECT snapshot FROM local_app_snapshots WHERE id = 1")
        .get_result::<SnapshotRow>(&mut connection)
        .unwrap();
    assert_eq!(row.snapshot, original);
}
