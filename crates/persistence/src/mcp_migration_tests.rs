use diesel::connection::SimpleConnection as _;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Text};
use diesel::sqlite::SqliteConnection;

const UP_SQL: &str = include_str!("../migrations/2026-09-01-000000_remove_mcp_tables/up.sql");
const DOWN_SQL: &str = include_str!("../migrations/2026-09-01-000000_remove_mcp_tables/down.sql");

#[derive(QueryableByName)]
struct CountRow {
    #[diesel(sql_type = BigInt)]
    count: i64,
}

#[derive(QueryableByName)]
struct ColumnRow {
    #[diesel(sql_type = Text)]
    name: String,
}

fn database_before_removal() -> SqliteConnection {
    let mut connection = SqliteConnection::establish(":memory:").unwrap();
    connection
        .batch_execute(
            "CREATE TABLE pane_leaves (\
                 pane_node_id INTEGER NOT NULL, \
                 kind TEXT NOT NULL, \
                 UNIQUE(pane_node_id, kind)\
             );",
        )
        .unwrap();
    connection.batch_execute(DOWN_SQL).unwrap();
    connection
}

fn table_exists(connection: &mut SqliteConnection, table: &str) -> bool {
    diesel::sql_query(
        "SELECT COUNT(*) AS count FROM sqlite_master WHERE type = 'table' AND name = ?",
    )
    .bind::<Text, _>(table)
    .get_result::<CountRow>(connection)
    .unwrap()
    .count
        == 1
}

fn table_columns(connection: &mut SqliteConnection, table: &str) -> Vec<String> {
    diesel::sql_query(format!("PRAGMA table_info('{table}')"))
        .load::<ColumnRow>(connection)
        .unwrap()
        .into_iter()
        .map(|row| row.name)
        .collect()
}

#[test]
fn removal_migration_drops_all_four_tables() {
    let mut connection = database_before_removal();
    connection
        .batch_execute(
            "INSERT INTO pane_leaves (pane_node_id, kind) VALUES (1, 'mcp_server');\
             INSERT INTO pane_leaves (pane_node_id, kind) VALUES (2, 'terminal');",
        )
        .unwrap();

    connection.batch_execute(UP_SQL).unwrap();

    assert!(!table_exists(&mut connection, "active_mcp_servers"));
    assert!(!table_exists(&mut connection, "mcp_environment_variables"));
    assert!(!table_exists(&mut connection, "mcp_server_installations"));
    assert!(!table_exists(&mut connection, "mcp_server_panes"));
    let remaining_kinds =
        diesel::sql_query("SELECT kind AS name FROM pane_leaves ORDER BY pane_node_id")
            .load::<ColumnRow>(&mut connection)
            .unwrap()
            .into_iter()
            .map(|row| row.name)
            .collect::<Vec<_>>();
    assert_eq!(remaining_kinds, vec!["terminal"]);
}

#[test]
fn removal_migration_down_restores_pre_removal_schema() {
    let mut connection = database_before_removal();
    connection.batch_execute(UP_SQL).unwrap();

    connection.batch_execute(DOWN_SQL).unwrap();

    assert_eq!(
        table_columns(&mut connection, "active_mcp_servers"),
        vec!["id", "mcp_server_uuid"]
    );
    assert_eq!(
        table_columns(&mut connection, "mcp_environment_variables"),
        vec!["mcp_server_uuid", "environment_variables"]
    );
    assert_eq!(
        table_columns(&mut connection, "mcp_server_installations"),
        vec![
            "id",
            "templatable_mcp_server",
            "template_version_ts",
            "variable_values",
            "restore_running",
            "last_modified_at",
        ]
    );
    assert_eq!(
        table_columns(&mut connection, "mcp_server_panes"),
        vec!["id", "kind"]
    );
}
