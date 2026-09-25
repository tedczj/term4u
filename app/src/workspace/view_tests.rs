use pathfinder_geometry::vector::vec2f;
use warpui::keymap::DescriptionContext;
use warpui::platform::WindowStyle;
use warpui::{App, Presenter, WindowInvalidation};

use super::*;
use crate::server::telemetry::PaletteSource;
use crate::terminal::view::TerminalAction;
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
fn l0_03_vim_palette_action_tracks_the_setting_without_changing_draft() {
    App::test((), |mut app| async move {
        let workspace = initialize_workspace(&mut app);
        let (window, input) = workspace.update(&mut app, |workspace, ctx| {
            workspace.insert_in_input("keep draft", true, ctx);
            (
                ctx.window_id(),
                workspace.terminal_for_input(ctx).as_ref(ctx).input().clone(),
            )
        });
        let vim_descriptions = |app: &App| {
            app.read(|ctx| {
                ctx.key_bindings_for_view(window, workspace.id())
                    .into_iter()
                    .filter_map(|binding| binding.description)
                    .map(|description| {
                        description
                            .materialized(ctx)
                            .in_context(DescriptionContext::Default)
                            .to_owned()
                    })
                    .filter(|description| description.ends_with("Vim Keybindings"))
                    .collect::<Vec<_>>()
            })
        };
        assert_eq!(vim_descriptions(&app), ["Enable Vim Keybindings"]);

        workspace.update(&mut app, |workspace, ctx| {
            workspace.handle_action(&WorkspaceAction::ToggleVimMode, ctx);
        });
        assert_eq!(vim_descriptions(&app), ["Disable Vim Keybindings"]);
        input.read(&app, |input, ctx| {
            assert_eq!(input.buffer_text(ctx), "keep draft");
            assert!(input.editor().as_ref(ctx).vim_mode_enabled(ctx));
        });

        workspace.update(&mut app, |workspace, ctx| {
            workspace.handle_action(&WorkspaceAction::ToggleVimMode, ctx);
        });
        assert_eq!(vim_descriptions(&app), ["Enable Vim Keybindings"]);
        input.read(&app, |input, ctx| {
            assert_eq!(input.buffer_text(ctx), "keep draft");
            assert!(!input.editor().as_ref(ctx).vim_mode_enabled(ctx));
        });
    });
}

#[test]
fn l0_01_editor_focus_selects_its_split_terminal() {
    App::test((), |mut app| async move {
        let workspace = initialize_workspace(&mut app);
        let group = workspace.read(&app, |workspace, _| {
            workspace.active_tab_pane_group().clone()
        });
        let first = group.read(&app, |group, ctx| group.active_session_view(ctx).unwrap());
        first.update(&mut app, |view, ctx| {
            view.handle_action(&TerminalAction::SplitRight(None), ctx);
        });
        let second = group.read(&app, |group, ctx| group.active_session_view(ctx).unwrap());
        assert_ne!(first.id(), second.id());
        let first_editor = first.read(&app, |view, ctx| view.input().as_ref(ctx).editor().clone());
        let second_editor =
            second.read(&app, |view, ctx| view.input().as_ref(ctx).editor().clone());
        let window = workspace.update(&mut app, |_, ctx| ctx.window_id());
        let presenter = app.presenter(window).unwrap();
        let size = app.read(|ctx| ctx.windows().platform_window(window).unwrap().size());
        app.update(|ctx| {
            let mut presenter = presenter.borrow_mut();
            presenter.invalidate(
                WindowInvalidation {
                    updated: [workspace.id(), group.id(), first.id(), second.id()]
                        .into_iter()
                        .collect(),
                    ..Default::default()
                },
                ctx,
            );
            presenter.build_scene(size, 1., None, ctx);
        });

        first.update(&mut app, |view, ctx| {
            view.input()
                .update(ctx, |input, ctx| input.focus_input_box(ctx));
        });
        group.read(&app, |group, ctx| {
            assert!(first_editor.is_focused(ctx));
            assert_eq!(group.active_session_view(ctx).unwrap().id(), first.id());
        });

        second.update(&mut app, |view, ctx| {
            view.input()
                .update(ctx, |input, ctx| input.focus_input_box(ctx));
        });
        group.read(&app, |group, ctx| {
            assert!(second_editor.is_focused(ctx));
            assert_eq!(group.active_session_view(ctx).unwrap().id(), second.id());
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

fn history_result(search: &ViewHandle<CommandSearchView>, app: &mut App) {
    search.update(app, |search, ctx| {
        search.handle_action(
            &crate::search::command_search::view::CommandSearchAction::ResultClicked {
                result_index: 0,
                result_action: Box::new(CommandSearchItemAction::AcceptHistory(
                    crate::search::command_search::searcher::AcceptedHistoryItem {
                        command: "echo chosen".into(),
                        linked_workflow_data: None,
                    },
                )),
            },
            ctx,
        );
    });
}

#[test]
fn l0_01_history_accept_fills_origin_without_execution_and_cancel_keeps_cursor() {
    App::test((), |mut app| async move {
        let workspace = initialize_workspace(&mut app);
        let terminal =
            workspace.update(&mut app, |workspace, ctx| workspace.terminal_for_input(ctx));
        terminal.update(&mut app, |view, ctx| {
            view.input().update(ctx, |input, ctx| {
                input.replace_buffer_content("draft 中文 tail", ctx);
                input.editor().update(ctx, |editor, ctx| {
                    editor.select_ranges_by_byte_offset([6_usize.into()..6_usize.into()], ctx);
                });
            });
        });
        let draft = terminal.read(&app, |view, ctx| {
            view.input()
                .as_ref(ctx)
                .editor()
                .as_ref(ctx)
                .snapshot_model(ctx)
        });
        let (tx, rx) = async_channel::unbounded();
        app.update(|ctx| {
            ctx.subscribe_to_view(&terminal, move |_, event, _| {
                if matches!(event, crate::terminal::view::Event::ExecuteCommand(_)) {
                    tx.try_send(()).unwrap();
                }
            });
        });
        let search = workspace.update(&mut app, |workspace, ctx| {
            workspace.show_command_search(&CommandSearchOptions::default(), ctx);
            workspace.command_search.clone().unwrap()
        });
        search.update(&mut app, |_, ctx| {
            ctx.emit(CommandSearchEvent::Close {
                query: "draft".into(),
                filter: None,
            });
        });
        terminal.read(&app, |view, ctx| {
            assert!(view.input().as_ref(ctx).editor().is_focused(ctx));
            assert_eq!(
                view.input()
                    .as_ref(ctx)
                    .editor()
                    .as_ref(ctx)
                    .snapshot_model(ctx),
                draft
            );
        });
        let search = workspace.update(&mut app, |workspace, ctx| {
            workspace.show_command_search(&CommandSearchOptions::default(), ctx);
            workspace.command_search.clone().unwrap()
        });
        history_result(&search, &mut app);
        terminal.read(&app, |view, ctx| {
            assert_eq!(view.input().as_ref(ctx).buffer_text(ctx), "echo chosen");
        });
        assert!(rx.is_empty());
    });
}

#[test]
fn l0_01_history_result_does_not_follow_a_tab_switch() {
    App::test((), |mut app| async move {
        let workspace = initialize_workspace(&mut app);
        let (origin, search) = workspace.update(&mut app, |workspace, ctx| {
            let terminal = workspace.terminal_for_input(ctx);
            workspace.insert_in_input("first draft", true, ctx);
            workspace.show_command_search(&CommandSearchOptions::default(), ctx);
            (terminal, workspace.command_search.clone().unwrap())
        });
        let other = workspace.update(&mut app, |workspace, ctx| {
            workspace.add_terminal_tab(false, ctx);
            workspace.insert_in_input("second draft", true, ctx);
            workspace.terminal_for_input(ctx)
        });
        history_result(&search, &mut app);
        origin.read(&app, |view, ctx| {
            assert_eq!(view.input().as_ref(ctx).buffer_text(ctx), "first draft")
        });
        other.read(&app, |view, ctx| {
            assert_eq!(view.input().as_ref(ctx).buffer_text(ctx), "second draft")
        });
    });
}

#[test]
fn l0_01_history_result_rejects_a_changed_then_restored_draft() {
    App::test((), |mut app| async move {
        let workspace = initialize_workspace(&mut app);
        let (terminal, search) = workspace.update(&mut app, |workspace, ctx| {
            workspace.insert_in_input("draft", true, ctx);
            let terminal = workspace.terminal_for_input(ctx);
            workspace.show_command_search(&CommandSearchOptions::default(), ctx);
            (terminal, workspace.command_search.clone().unwrap())
        });
        terminal.update(&mut app, |view, ctx| {
            view.input().update(ctx, |input, ctx| {
                input.replace_buffer_content("intervening edit", ctx);
                input.replace_buffer_content("draft", ctx);
            });
        });
        history_result(&search, &mut app);
        terminal.read(&app, |view, ctx| {
            assert_eq!(view.input().as_ref(ctx).buffer_text(ctx), "draft")
        });
    });
}

#[test]
fn l0_01_ctrl_r_is_not_captured_by_history_in_a_native_program() {
    App::test((), |mut app| async move {
        let workspace = initialize_workspace(&mut app);
        let (window, terminal) = workspace.update(&mut app, |workspace, ctx| {
            (ctx.window_id(), workspace.terminal_for_input(ctx))
        });
        app.update(|ctx| {
            ctx.dispatch_custom_action(crate::util::bindings::CustomAction::CommandSearch, window)
        });
        let search = workspace
            .read(&app, |workspace, _| workspace.command_search.clone())
            .expect("the menu action must be available in the editor");
        search.update(&mut app, |search, ctx| {
            search.handle_action(
                &crate::search::command_search::view::CommandSearchAction::Close,
                ctx,
            )
        });
        terminal.update(&mut app, |view, ctx| {
            view.model.lock().process_bytes("\x1b[?1049h");
            ctx.focus_self();
        });
        app.update(|ctx| {
            ctx.dispatch_custom_action(crate::util::bindings::CustomAction::CommandSearch, window)
        });
        let handled = app
            .dispatch_keystroke(
                window,
                &[workspace.id(), terminal.id()],
                &warpui::keymap::Keystroke::parse("ctrl-r").unwrap(),
                false,
            )
            .unwrap();
        assert!(!handled);
        assert!(workspace.read(&app, |workspace, _| workspace.command_search.is_none()));
    });
}

#[test]
fn l0_01_cancelled_search_accepts_text_before_the_next_frame() {
    use std::cell::RefCell;
    use std::rc::Rc;

    App::test((), |mut app| async move {
        let workspace = initialize_workspace(&mut app);
        let (window, terminal, search) = workspace.update(&mut app, |workspace, ctx| {
            workspace.insert_in_input("draft", true, ctx);
            let terminal = workspace.terminal_for_input(ctx);
            workspace.show_command_search(&CommandSearchOptions::default(), ctx);
            (
                ctx.window_id(),
                terminal,
                workspace.command_search.clone().unwrap(),
            )
        });
        let mut presenter = Presenter::new(window);
        let updated = app.read(|ctx| ctx.view_ids_for_window(window).into_iter().collect());
        let presenter = app.update(move |ctx| {
            presenter.invalidate(
                WindowInvalidation {
                    updated,
                    ..Default::default()
                },
                ctx,
            );
            presenter.build_scene(vec2f(800., 600.), 1., None, ctx);
            Rc::new(RefCell::new(presenter))
        });
        search.update(&mut app, |_, ctx| {
            ctx.emit(CommandSearchEvent::Close {
                query: "draft".into(),
                filter: None,
            });
        });
        terminal.read(&app, |view, ctx| {
            assert!(view.input().as_ref(ctx).editor().is_focused(ctx))
        });
        // Retain the painted search frame: text delivery must use current focus, not its snapshot.
        app.update(|ctx| {
            ctx.simulate_window_event(
                warpui::Event::TypedCharacters { chars: "x".into() },
                window,
                presenter,
            );
        });
        terminal.read(&app, |view, ctx| {
            assert_eq!(view.input().as_ref(ctx).buffer_text(ctx), "draftx")
        });
    });
}

#[test]
fn l0_07_notification_click_resolves_live_terminal_and_rejects_stale_data() {
    use warpui::notification::NotificationResponse;

    use crate::notification::{NotificationContext, handle_notification_response};
    App::test((), |mut app| async move {
        let workspace = initialize_workspace(&mut app);
        let terminal = workspace.update(&mut app, |w, ctx| {
            let t = w.terminal_for_input(ctx);
            w.add_terminal_tab(false, ctx);
            t
        });
        let response = |context: NotificationContext| {
            NotificationResponse::new(
                chrono::DateTime::UNIX_EPOCH.naive_utc(),
                Some(serde_json::to_string(&context).unwrap()),
            )
        };
        let stale = response(NotificationContext::TerminalOrigin {
            run_id: uuid::Uuid::nil(),
            terminal_view_id: terminal.id(),
        });
        app.update(|ctx| handle_notification_response(&stale, ctx));
        assert_eq!(workspace.read(&app, |w, _| w.active_tab_index), 1);
        let absent = response(NotificationContext::for_terminal(warpui::EntityId::new()));
        app.update(|ctx| handle_notification_response(&absent, ctx));
        let legacy = workspace.read(&app, |w, ctx| {
            let group = w.tabs[0].pane_group.as_ref(ctx);
            response(NotificationContext::BlockOrigin {
                window_id: workspace.window_id(ctx),
                pane_group_id: w.tabs[0].pane_group.id(),
                pane_id: group
                    .find_pane_id_for_terminal_view(terminal.id(), ctx)
                    .unwrap(),
            })
        });
        app.update(|ctx| handle_notification_response(&legacy, ctx));
        assert_eq!(workspace.read(&app, |w, _| w.active_tab_index), 1);
        let valid = response(NotificationContext::for_terminal(terminal.id()));
        app.update(|ctx| handle_notification_response(&valid, ctx));
        assert_eq!(workspace.read(&app, |w, _| w.active_tab_index), 0);
        workspace.read(&app, |w, ctx| {
            let group = w.active_tab_pane_group().as_ref(ctx);
            assert_eq!(
                group.find_pane_id_for_terminal_view(terminal.id(), ctx),
                Some(group.focused_pane_id(ctx))
            );
        });
        workspace.update(&mut app, |w, ctx| {
            w.handle_action(&WorkspaceAction::CloseActiveTab, ctx)
        });
        let before = workspace.read(&app, |w, _| w.active_tab_index);
        app.update(|ctx| handle_notification_response(&valid, ctx));
        assert_eq!(workspace.read(&app, |w, _| w.active_tab_index), before);
    });
}
