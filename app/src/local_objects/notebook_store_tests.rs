use super::NotebookStore;

#[test]
fn legacy_notebook_can_be_edited_and_reloaded() {
    let directory = tempfile::tempdir().unwrap();
    let legacy = r#"{"data":"# Legacy\noriginal","unknown":"ignored"}"#;
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
fn malformed_legacy_notebook_does_not_block_valid_notebooks() {
    let directory = tempfile::tempdir().unwrap();
    let valid = r#"{"data":"valid"}"#;
    let store = NotebookStore::load(
        directory.path().to_path_buf(),
        vec![
            (1, Some("broken".to_owned()), Some("{".to_owned())),
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
