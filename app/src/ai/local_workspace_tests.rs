use std::sync::mpsc::sync_channel;

use warpui::App;

use super::*;

#[test]
fn reopening_workspace_keeps_existing_metadata_and_lsp_preferences() {
    App::test((), |mut app| async move {
        let (sender, receiver) = sync_channel(2);
        let path = PathBuf::from("/project");
        let metadata = WorkspaceMetadata {
            path: path.clone(),
            modified_ts: Some(Utc::now()),
            queried_ts: Some(Utc::now()),
            ..Default::default()
        };
        let model = app.add_model(|_| {
            PersistedWorkspace::new(
                vec![metadata.clone()],
                HashMap::from([(
                    path.clone(),
                    HashMap::from([(LSPServerType::RustAnalyzer, EnablementState::Yes)]),
                )]),
                Some(sender),
            )
        });
        model.update(&mut app, |model, ctx| model.user_added_workspace(path, ctx));
        let ModelEvent::UpsertCodebaseIndexMetadata { index_metadata } =
            receiver.try_recv().unwrap()
        else {
            panic!("workspace metadata must reach the writer")
        };
        assert_eq!(index_metadata.modified_ts, metadata.modified_ts);
        assert_eq!(index_metadata.queried_ts, metadata.queried_ts);
        assert!(index_metadata.navigated_ts.is_some());
        model.read(&app, |model, _| {
            assert_eq!(
                model
                    .enabled_lsp_servers(Path::new("/project/main.rs"))
                    .unwrap()
                    .collect::<Vec<_>>(),
                vec![LSPServerType::RustAnalyzer]
            )
        });
    });
}

#[test]
fn restored_workspace_keeps_language_server_preferences() {
    let root = PathBuf::from("/project");
    let model = PersistedWorkspace::new(
        vec![WorkspaceMetadata {
            path: root.clone(),
            ..Default::default()
        }],
        HashMap::from([(
            root,
            HashMap::from([
                (LSPServerType::RustAnalyzer, EnablementState::Yes),
                (LSPServerType::Pyright, EnablementState::No),
            ]),
        )]),
        None,
    );

    assert_eq!(
        model
            .enabled_lsp_servers(Path::new("/project/src/main.rs"))
            .unwrap()
            .collect::<Vec<_>>(),
        vec![LSPServerType::RustAnalyzer]
    );
    assert_eq!(
        model.workspaces().next().unwrap().path,
        Path::new("/project")
    );
}

#[test]
fn changing_language_server_preference_reaches_persistence_writer() {
    let (sender, receiver) = sync_channel(2);
    let mut model = PersistedWorkspace::new(Vec::new(), HashMap::new(), Some(sender));

    model.enable_lsp_server_for_path(Path::new("/project"), LSPServerType::RustAnalyzer);

    let ModelEvent::UpsertCodebaseIndexMetadata { index_metadata } = receiver.try_recv().unwrap()
    else {
        panic!("workspace metadata must be saved before its language server preference");
    };
    assert_eq!(index_metadata.path, Path::new("/project"));
    let ModelEvent::UpsertWorkspaceLanguageServer {
        workspace_path,
        lsp_type,
        enabled,
    } = receiver.try_recv().unwrap()
    else {
        panic!("language server preference must reach the persistence writer");
    };
    assert_eq!(workspace_path, Path::new("/project"));
    assert_eq!(lsp_type, LSPServerType::RustAnalyzer);
    assert_eq!(enabled, EnablementState::Yes);
}
