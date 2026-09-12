use pathfinder_geometry::vector::vec2f;
use warpui::platform::WindowStyle;
use warpui::{App, Presenter, WindowInvalidation};

use super::*;
use crate::server::telemetry::PaletteSource;
use crate::test_util::terminal::initialize_app_for_pane_group;

fn initialize_workspace(app: &mut App) -> ViewHandle<Workspace> {
    initialize_app_for_pane_group(app);
    app.add_singleton_model(crate::appearance::AppearanceManager::new);
    app.update(super::super::register_bindings);
    app.update(crate::undo_close::init);
    app.add_singleton_model(|_| SettingsPaneManager::new());
    app.add_singleton_model(|_| crate::search::command_palette::SelectedItems::new());
    app.add_singleton_model(|_| crate::code::opened_files::OpenedFilesModel::new());
    app.update(lsp::init);
    app.add_singleton_model(|_| crate::terminal::local_shell::LocalShellState::NotLoaded);
    app.add_singleton_model(warp_files::FileModel::new);
    app.add_singleton_model(crate::code::global_buffer_model::GlobalBufferModel::new);
    app.add_singleton_model(|_| crate::code::editor_management::CodeManager::default());
    app.add_singleton_model(|_| crate::code_review::GlobalCodeReviewModel);
    let resources = GlobalResourceHandles::mock(app);
    app.add_window(WindowStyle::NotStealFocus, |ctx| {
        Workspace::new_for_test(resources, ctx)
    })
    .1
}

#[test]
fn restoring_tabs_keeps_each_left_panel_mode_and_width() {
    App::test((), |mut app| async move {
        let workspace = initialize_workspace(&mut app);
        let (resources, mut snapshot) = workspace.update(&mut app, |workspace, ctx| {
            workspace.add_terminal_tab(false, ctx);
            (
                workspace.resources.clone(),
                workspace.snapshot(ctx.window_id(), false, ctx),
            )
        });
        snapshot.left_panel_open = true;
        snapshot.tabs[0].left_panel = Some(LeftPanelSnapshot {
            left_panel_displayed_tab: crate::app_state::LeftPanelDisplayedTab::GlobalSearch,
            pane_group_id: "legacy-first-panel".into(),
            width: 342,
        });
        snapshot.tabs[1].left_panel = Some(LeftPanelSnapshot {
            left_panel_displayed_tab: crate::app_state::LeftPanelDisplayedTab::FileTree,
            pane_group_id: "legacy-second-panel".into(),
            width: 310,
        });
        let expected = snapshot
            .tabs
            .iter()
            .map(|tab| tab.left_panel.clone())
            .collect::<Vec<_>>();
        let restored = app
            .add_window(WindowStyle::NotStealFocus, |ctx| {
                Workspace::new(
                    resources,
                    NewWorkspaceSource::Restored {
                        window_snapshot: snapshot,
                        block_lists: Arc::new(HashMap::new()),
                    },
                    ctx,
                )
            })
            .1;
        restored.update(&mut app, |workspace, ctx| {
            workspace.activate_tab(0, ctx);
            assert_eq!(
                workspace.left_panel_view(),
                LeftPanelTargetView::GlobalSearch
            );
            workspace.activate_tab(1, ctx);
            assert_eq!(
                workspace.left_panel_view(),
                LeftPanelTargetView::ProjectExplorer
            );
            let snapshot = workspace.snapshot(ctx.window_id(), false, ctx);
            assert_eq!(
                snapshot
                    .tabs
                    .iter()
                    .map(|tab| tab.left_panel.clone())
                    .collect::<Vec<_>>(),
                expected
            );
        });
    });
}

#[test]
fn toast_events_reach_only_the_target_window_and_can_be_dismissed() {
    App::test((), |mut app| async move {
        let workspace = initialize_workspace(&mut app);
        let resources = workspace.read(&app, |workspace, _| workspace.resources.clone());
        let other = app
            .add_window(WindowStyle::NotStealFocus, |ctx| {
                Workspace::new_for_test(resources, ctx)
            })
            .1;
        let window_id = workspace.read(&app, |_, ctx| workspace.window_id(ctx));
        app.update(|ctx| {
            ToastStack::handle(ctx).update(ctx, |stack, ctx| {
                stack.add_persistent_toast(
                    crate::view_components::DismissibleToast::error(
                        "Install the language server manually".into(),
                    )
                    .with_object_id("missing-server".into()),
                    window_id,
                    ctx,
                );
            });
        });
        workspace.read(&app, |workspace, ctx| {
            assert!(workspace.toasts.as_ref(ctx).has_toasts())
        });
        other.read(&app, |workspace, ctx| {
            assert!(!workspace.toasts.as_ref(ctx).has_toasts())
        });
        app.update(|ctx| {
            ToastStack::handle(ctx).update(ctx, |stack, ctx| {
                stack.remove_toast_by_identifier("missing-server".into(), window_id, ctx)
            });
        });
        workspace.read(&app, |workspace, ctx| {
            assert!(!workspace.toasts.as_ref(ctx).has_toasts())
        });
    });
}

#[test]
fn exiting_split_terminal_preserves_sibling_and_tab() {
    App::test((), |mut app| async move {
        let workspace = initialize_workspace(&mut app);
        let group = workspace.read(&app, |workspace, _| {
            workspace.active_tab_pane_group().clone()
        });
        let first = group.read(&app, |group, ctx| group.active_session_view(ctx).unwrap());
        first.update(&mut app, |view, ctx| {
            view.handle_action(
                &crate::terminal::view::TerminalAction::SplitRight(None),
                ctx,
            )
        });
        let second = group.read(&app, |group, ctx| {
            assert_eq!(group.visible_pane_ids().len(), 2);
            group.active_session_view(ctx).unwrap()
        });
        assert_ne!(first.id(), second.id());
        second.update(&mut app, |_, ctx| {
            ctx.emit(crate::terminal::view::Event::Exited)
        });
        workspace.read(&app, |workspace, ctx| {
            assert_eq!(workspace.active_tab_pane_group().id(), group.id());
            assert_eq!(group.as_ref(ctx).visible_pane_ids().len(), 1);
            assert!(group.as_ref(ctx).contains_terminal_view(first.id(), ctx));
        });
    });
}

#[test]
fn closed_tab_reopens_in_original_position() {
    App::test((), |mut app| async move {
        let workspace = initialize_workspace(&mut app);
        let original = workspace.update(&mut app, |workspace, ctx| {
            workspace.add_terminal_tab(false, ctx);
            let ids = workspace
                .tab_views()
                .map(|view| view.id())
                .collect::<Vec<_>>();
            workspace.handle_action(&WorkspaceAction::CloseActiveTab, ctx);
            ids
        });
        workspace.update(&mut app, |workspace, ctx| {
            workspace.handle_action(&WorkspaceAction::ReopenClosedSession, ctx)
        });
        workspace.read(&app, |workspace, _| {
            assert_eq!(
                workspace
                    .tab_views()
                    .map(|view| view.id())
                    .collect::<Vec<_>>(),
                original
            );
            assert_eq!(workspace.active_tab_index(), 1);
        });
    });
}

#[test]
fn selecting_workflow_populates_terminal_input_without_executing() {
    App::test((), |mut app| async move {
        let workspace = initialize_workspace(&mut app);
        let search = workspace.update(&mut app, |workspace, ctx| {
            workspace.handle_action(
                &WorkspaceAction::ShowCommandSearch(CommandSearchOptions::default()),
                ctx,
            );
            workspace.command_search.clone().unwrap()
        });
        search.update(&mut app, |_, ctx| {
            ctx.emit(CommandSearchEvent::ItemSelected {
                query: "local".into(),
                payload: Box::new(CommandSearchItemAction::AcceptWorkflow(
                    AcceptedWorkflow::Local {
                        workflow: Box::new(crate::workflows::WorkflowType::Local(
                            crate::workflows::model::Workflow::new("Local", "printf {{message}}")
                                .with_arguments(vec![
                                    crate::workflows::Argument::new(
                                        "message",
                                        crate::workflows::ArgumentType::Text,
                                    )
                                    .with_default("R1R2_WORKFLOW"),
                                ]),
                        )),
                        source: crate::workflows::WorkflowSource::Local,
                    },
                )),
            })
        });
        workspace.read(&app, |workspace, ctx| {
            let terminal = workspace
                .active_tab_pane_group()
                .as_ref(ctx)
                .active_session_view(ctx)
                .unwrap();
            assert_eq!(
                terminal.as_ref(ctx).input().as_ref(ctx).buffer_text(ctx),
                "printf R1R2_WORKFLOW"
            );
        });
    });
}

#[test]
fn workflow_run_waits_for_missing_argument() {
    App::test((), |mut app| async move {
        let workspace = initialize_workspace(&mut app);
        let workflow =
            Workflow::new("Required argument", "printf {{message}}").with_arguments(vec![
                crate::workflows::Argument::new("message", crate::workflows::ArgumentType::Text),
            ]);
        workspace.update(&mut app, |workspace, ctx| {
            workspace.use_workflow(&workflow, None, true, ctx)
        });
        workspace.read(&app, |workspace, ctx| {
            let terminal = workspace
                .active_tab_pane_group()
                .as_ref(ctx)
                .active_session_view(ctx)
                .unwrap();
            let editor = terminal
                .as_ref(ctx)
                .input()
                .as_ref(ctx)
                .editor()
                .as_ref(ctx);
            assert_eq!(editor.buffer_text(ctx), "printf message");
            assert_eq!(editor.selected_text(ctx), "message");
        });
    });
}

#[test]
fn code_review_snapshot_restores_repository_and_terminal_link() {
    App::test((), |mut app| async move {
        let workspace = initialize_workspace(&mut app);
        let directory = tempfile::tempdir().unwrap();
        let snapshot = crate::app_state::CodeReviewPaneSnapshot::Local {
            repo_path: directory.path().to_owned(),
            terminal_uuid: vec![7; 16],
        };
        workspace.update(&mut app, |workspace, ctx| {
            workspace.add_tab_with_pane_layout(
                PanesLayout::Snapshot(Box::new(crate::app_state::PaneNodeSnapshot::Leaf(
                    Box::new(crate::app_state::LeafSnapshot {
                        is_focused: true,
                        custom_vertical_tabs_title: None,
                        contents: crate::app_state::LeafContents::CodeReview(snapshot.clone()),
                    }),
                ))),
                Arc::new(HashMap::new()),
                None,
                ctx,
            )
        });
        workspace.read(&app, |workspace, ctx| {
            let restored = workspace.active_tab_pane_group().as_ref(ctx).snapshot(ctx);
            let crate::app_state::PaneNodeSnapshot::Leaf(leaf) = restored else {
                panic!("expected one restored review pane")
            };
            assert_eq!(
                leaf.contents,
                crate::app_state::LeafContents::CodeReview(snapshot)
            );
        });
    });
}

#[test]
fn project_explorer_result_opens_local_file() {
    open_file_from_panel(LeftPanelTargetView::ProjectExplorer);
}

#[test]
fn global_search_result_opens_local_file() {
    open_file_from_panel(LeftPanelTargetView::GlobalSearch);
}

fn open_file_from_panel(panel: LeftPanelTargetView) {
    App::test((), |mut app| async move {
        let workspace = initialize_workspace(&mut app);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("example.rs");
        std::fs::write(&path, "fn main() {}\n").unwrap();
        workspace.update(&mut app, |workspace, ctx| {
            let action = match panel {
                LeftPanelTargetView::ProjectExplorer => WorkspaceAction::OpenProjectExplorer,
                LeftPanelTargetView::GlobalSearch => WorkspaceAction::OpenGlobalSearch,
            };
            workspace.handle_action(&action, ctx);
            assert!(workspace.left_panel_open);
            assert_eq!(workspace.left_panel_view(), panel);
            let group_id = workspace.active_tab_pane_group().id();
            match panel {
                LeftPanelTargetView::ProjectExplorer => {
                    let tree = workspace
                        .working_directories
                        .as_ref(ctx)
                        .get_file_tree_view(group_id)
                        .unwrap();
                    tree.update(ctx, |_, ctx| {
                        ctx.emit(FileTreeEvent::OpenFile {
                            path: LocalOrRemotePath::Local(path.clone()),
                            target: FileTarget::CodeEditor(EditorLayout::NewTab),
                            line_col: None,
                        })
                    });
                }
                LeftPanelTargetView::GlobalSearch => {
                    workspace.global_search_views[&group_id].update(ctx, |_, ctx| {
                        ctx.emit(GlobalSearchViewEvent::OpenMatch {
                            location: LocalOrRemotePath::Local(path.clone()),
                            line_number: 1,
                            column_num: Some(1),
                        })
                    });
                }
            }
        });
        let code = workspace.read(&app, |workspace, ctx| {
            assert_eq!(workspace.tab_count(), 2);
            workspace
                .active_tab_pane_group()
                .as_ref(ctx)
                .code_panes(ctx)
                .next()
                .unwrap()
                .1
        });
        let loaded = code.update(&mut app, |code, ctx| {
            let id = code.active_file_id_for_test(ctx).unwrap();
            let future = warp_files::FileModel::as_ref(ctx)
                .get_future_handle(id)
                .unwrap();
            ctx.await_spawned_future(future.future_id())
        });
        loaded.await;
        code.read(&app, |code, ctx| {
            let id = code.active_file_id_for_test(ctx).unwrap();
            assert_eq!(
                warp_files::FileModel::as_ref(ctx).file_path(id),
                Some(path.canonicalize().unwrap())
            );
        });
        workspace.read(&app, |workspace, ctx| {
            let directory = ActiveSession::as_ref(ctx)
                .path_if_local(workspace.window_id(ctx))
                .unwrap();
            assert_eq!(directory, path.parent().unwrap().canonicalize().unwrap());
        });
    });
}

#[test]
fn command_palette_opens_with_local_bindings_and_closes() {
    App::test((), |mut app| async move {
        let workspace = initialize_workspace(&mut app);
        app.update(|ctx| {
            let mut presenter = Presenter::new(workspace.window_id(ctx));
            presenter.invalidate(
                WindowInvalidation {
                    updated: [workspace.id()].into_iter().collect(),
                    ..Default::default()
                },
                ctx,
            );
            presenter.build_scene(vec2f(1024., 768.), 1., None, ctx);
        });
        workspace.update(&mut app, |workspace, ctx| {
            workspace.handle_action(
                &WorkspaceAction::OpenPalette {
                    mode: PaletteMode::Command,
                    source: PaletteSource::Keybinding,
                    query: Some("New Notebook".into()),
                },
                ctx,
            );
        });
        for mode in [PaletteMode::Files, PaletteMode::Command] {
            workspace.update(&mut app, |workspace, ctx| {
                workspace.handle_action(
                    &WorkspaceAction::OpenPalette {
                        mode,
                        source: PaletteSource::Keybinding,
                        query: None,
                    },
                    ctx,
                );
            });
        }
        let palette = workspace.read(&app, |workspace, _| {
            workspace.command_palette.clone().unwrap()
        });
        palette.read(&app, |palette, ctx| {
            assert_eq!(palette.active_query_filter(ctx), Some(QueryFilter::Actions));
            let binding = ctx.get_binding_by_name("workspace:new_notebook").unwrap();
            assert!(
                palette
                    .data_source_store
                    .as_ref(ctx)
                    .query_result_for_binding_id(binding.id, ctx)
                    .is_some()
            );
        });
        palette.update(&mut app, |palette, ctx| {
            palette.handle_action(&crate::search::command_palette::view::Action::Close, ctx)
        });
        workspace.read(&app, |workspace, _| {
            assert!(workspace.command_palette.is_none())
        });
    });
}
