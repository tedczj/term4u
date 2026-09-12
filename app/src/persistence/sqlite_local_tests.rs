use diesel_migrations::MigrationHarness;

use super::*;

fn database() -> (tempfile::TempDir, SqliteConnection) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("legacy.sqlite");
    std::fs::write(
        &path,
        include_bytes!("../../../crates/persistence/fixtures/legacy/restored_notebooks.sqlite"),
    )
    .unwrap();
    let mut connection = SqliteConnection::establish(path.to_str().unwrap()).unwrap();
    connection
        .run_pending_migrations(::persistence::MIGRATIONS)
        .unwrap();
    (directory, connection)
}

#[test]
fn trashed_legacy_notebooks_are_hidden_without_deleting_their_body() {
    let (_directory, mut connection) = database();
    assert_eq!(
        read_sqlite_data(&mut connection, PersistedDataScope::Full)
            .unwrap()
            .legacy_notebooks
            .len(),
        1
    );
    diesel::update(schema::object_metadata::table)
        .set(schema::object_metadata::trashed_ts.eq(Some(1234)))
        .execute(&mut connection)
        .unwrap();
    assert!(
        read_sqlite_data(&mut connection, PersistedDataScope::Full)
            .unwrap()
            .legacy_notebooks
            .is_empty()
    );
    assert_eq!(
        schema::notebooks::table
            .find(12)
            .select(schema::notebooks::data)
            .first::<Option<String>>(&mut connection)
            .unwrap()
            .as_deref(),
        Some("Notebook 1 content")
    );
}

#[test]
fn notebooks_without_legacy_cloud_metadata_remain_readable() {
    let (_directory, mut connection) = database();
    diesel::insert_into(schema::notebooks::table)
        .values((
            schema::notebooks::id.eq(999),
            schema::notebooks::title.eq("Local"),
            schema::notebooks::data.eq("local body"),
        ))
        .execute(&mut connection)
        .unwrap();
    let data = read_sqlite_data(&mut connection, PersistedDataScope::Full).unwrap();
    let notebook = data
        .legacy_notebooks
        .iter()
        .find(|(id, _, _)| *id == 999)
        .unwrap();
    assert_eq!(notebook.1.as_deref(), Some("Local"));
    assert_eq!(notebook.2.as_deref(), Some("local body"));
}
