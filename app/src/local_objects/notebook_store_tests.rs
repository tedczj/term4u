use diesel::prelude::*;
use diesel_migrations::MigrationHarness;

use super::NotebookStore;

#[test]
fn historical_sqlite_notebook_body_survives_edit_and_reload() {
    let directory = tempfile::tempdir().unwrap();
    let database_path = directory.path().join("legacy.sqlite");
    std::fs::write(
        &database_path,
        include_bytes!("../../../crates/persistence/fixtures/legacy/restored_notebooks.sqlite"),
    )
    .unwrap();
    let mut connection = SqliteConnection::establish(database_path.to_str().unwrap()).unwrap();
    connection
        .run_pending_migrations(persistence::MIGRATIONS)
        .unwrap();
    let rows = persistence::schema::notebooks::table
        .select((
            persistence::schema::notebooks::id,
            persistence::schema::notebooks::title,
            persistence::schema::notebooks::data,
        ))
        .load(&mut connection)
        .unwrap();
    let storage = directory.path().join("notebooks");
    let mut store = NotebookStore::load(storage.clone(), rows);
    let id = crate::notebooks::NotebookId::from_legacy_id(12);
    let mut notebook = store.get(&id).unwrap().clone();
    assert_eq!(notebook.title, "First Notebook");
    assert_eq!(notebook.data, "Notebook 1 content");

    notebook.data = "Edited legacy notebook".to_owned();
    store.upsert(notebook).unwrap();
    let reloaded = NotebookStore::load(
        storage,
        vec![(
            12,
            Some("First Notebook".to_owned()),
            Some("Notebook 1 content".to_owned()),
        )],
    );
    assert_eq!(reloaded.get(&id).unwrap().data, "Edited legacy notebook");
    assert_eq!(
        persistence::schema::notebooks::table
            .find(12)
            .select(persistence::schema::notebooks::data)
            .first::<Option<String>>(&mut connection)
            .unwrap()
            .as_deref(),
        Some("Notebook 1 content")
    );
}

#[test]
fn legacy_notebook_can_be_edited_and_reloaded() {
    let directory = tempfile::tempdir().unwrap();
    let legacy = "# Legacy\noriginal";
    let mut store = NotebookStore::load(
        directory.path().to_path_buf(),
        vec![(7, Some("Legacy title".to_owned()), Some(legacy.to_owned()))],
    );
    let id = crate::notebooks::model::NotebookId::from_legacy_id(7);
    let mut notebook = store.get(&id).unwrap().clone();
    assert_eq!(notebook.data, "# Legacy\noriginal");

    notebook.data = "# Legacy\nedited".to_owned();
    store.upsert(notebook).unwrap();
    let reloaded = NotebookStore::load(directory.path().to_path_buf(), Vec::new());

    assert_eq!(reloaded.get(&id).unwrap().data, "# Legacy\nedited");
}

#[test]
fn incomplete_legacy_notebook_does_not_block_valid_notebooks() {
    let directory = tempfile::tempdir().unwrap();
    let valid = "valid";
    let store = NotebookStore::load(
        directory.path().to_path_buf(),
        vec![
            (1, Some("incomplete".to_owned()), None),
            (2, Some("valid".to_owned()), Some(valid.to_owned())),
        ],
    );

    assert_eq!(store.all().count(), 1);
    assert_eq!(
        store
            .get(&crate::notebooks::model::NotebookId::from_legacy_id(2))
            .unwrap()
            .data,
        "valid"
    );
}

#[test]
fn legacy_json_body_is_preserved_verbatim() {
    let directory = tempfile::tempdir().unwrap();
    let body = r#"{"data":"example","future":true}"#;
    let store = NotebookStore::load(
        directory.path().to_path_buf(),
        vec![(1, None, Some(body.to_owned()))],
    );
    let notebook = store
        .get(&crate::notebooks::NotebookId::from_legacy_id(1))
        .unwrap();
    assert_eq!(notebook.data, body);
    assert_eq!(notebook.title, "");
}

#[test]
fn editing_local_notebook_keeps_unknown_json_fields() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("sample.json"),
        r#"{"id":"sample","data":"before","future":{"enabled":true}}"#,
    )
    .unwrap();
    let mut store = NotebookStore::load(directory.path().to_path_buf(), Vec::new());
    let mut notebook = store.all().next().unwrap().clone();
    let id = notebook.id.clone();
    assert_eq!(notebook.title, "");
    notebook.data = "after".to_owned();
    store.upsert(notebook).unwrap();
    let reloaded = NotebookStore::load(directory.path().to_path_buf(), Vec::new());
    let notebook = reloaded.get(&id).unwrap();
    assert_eq!(notebook.data, "after");
    assert_eq!(
        notebook.extra["future"],
        serde_json::json!({"enabled": true})
    );
}
