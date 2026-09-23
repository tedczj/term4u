use std::iter;

use warp_util::path::CleanPathResult;

use super::super::TerminalView;
use super::{GridHighlightedLink, path_without_trailing_sentence_punctuation};
use crate::terminal::model::grid::grid_handler::PossiblePath;
use crate::terminal::model::index::Point;
use crate::terminal::model::terminal_model::WithinModel;

#[test]
fn strips_only_sentence_periods() {
    // A trailing period after a real file name is sentence punctuation.
    assert_eq!(
        path_without_trailing_sentence_punctuation("notes/README.md.").map(|trimmed| trimmed.path),
        Some("notes/README.md")
    );
    assert_eq!(
        path_without_trailing_sentence_punctuation(".gitignore.").map(|trimmed| trimmed.path),
        Some(".gitignore")
    );
    assert_eq!(
        path_without_trailing_sentence_punctuation("C:/Users/c/warp-md-test.md.")
            .map(|trimmed| trimmed.path),
        Some("C:/Users/c/warp-md-test.md")
    );

    // No trailing period -> nothing to trim.
    assert_eq!(
        path_without_trailing_sentence_punctuation("notes/README.md").map(|trimmed| trimmed.path),
        None
    );

    // `.`/`..` path components must be preserved, not treated as punctuation.
    assert_eq!(
        path_without_trailing_sentence_punctuation(".").map(|trimmed| trimmed.path),
        None
    );
    assert_eq!(
        path_without_trailing_sentence_punctuation("..").map(|trimmed| trimmed.path),
        None
    );
    assert_eq!(
        path_without_trailing_sentence_punctuation("foo/.").map(|trimmed| trimmed.path),
        None
    );
    assert_eq!(
        path_without_trailing_sentence_punctuation("foo/..").map(|trimmed| trimmed.path),
        None
    );
    assert_eq!(
        path_without_trailing_sentence_punctuation("foo..").map(|trimmed| trimmed.path),
        None
    );
}

#[test]
fn strips_trailing_fullwidth_sentence_punctuation() {
    let trimmed = path_without_trailing_sentence_punctuation("notes/README.md，")
        .expect("fullwidth comma should be stripped");
    assert_eq!(trimmed.path, "notes/README.md");
    assert_eq!(trimmed.removed_width, 2);

    let trimmed = path_without_trailing_sentence_punctuation("notes/README.md。！？")
        .expect("CJK sentence punctuation should be stripped");
    assert_eq!(trimmed.path, "notes/README.md");
    assert_eq!(trimmed.removed_width, 6);
}

// Regression test for https://github.com/warpdotdev/warp/issues/11477:
// a `.md` path at the end of a sentence captured the trailing period, so the
// resolved file and the highlight range ended in `.md.` and the file failed
// markdown classification. The trailing period must be excluded from both.
#[test]
fn compute_valid_paths_excludes_trailing_sentence_period() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("warp-md-test.md");
    std::fs::write(&file, "# Hello\n").unwrap();

    // The captured token as it would appear in `Drafted at <abs path>.`
    let token = format!("{}.", file.to_string_lossy());
    let end_col = token.chars().count() - 1;
    let candidate = WithinModel::AltScreen(PossiblePath {
        path: CleanPathResult {
            path: token,
            line_and_column_num: None,
        },
        range: Point { row: 0, col: 0 }..=Point {
            row: 0,
            col: end_col,
        },
    });

    let link = TerminalView::compute_valid_paths(
        dir.path().to_str().unwrap(),
        iter::once(candidate),
        1000,
        None,
    )
    .expect("the markdown file should be detected as a link");

    let GridHighlightedLink::File(file_link) = link else {
        panic!("expected a file link");
    };
    let file_link = file_link.get_inner();

    // The resolved file excludes the trailing period (so it classifies as `.md`)...
    assert_eq!(
        file_link.absolute_path.file_name().unwrap(),
        "warp-md-test.md"
    );
    // ...and the highlighted range stops before the trailing period.
    assert_eq!(
        *file_link.link.range().end(),
        Point {
            row: 0,
            col: end_col - 1,
        }
    );
}

#[test]
fn compute_valid_paths_excludes_trailing_fullwidth_sentence_punctuation() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("warp-md-test.md");
    std::fs::write(&file, "# Hello\n").unwrap();

    let token = format!("{}，", file.to_string_lossy());
    let punctuation_width = 2;
    let end_col = token.chars().count();
    let candidate = WithinModel::AltScreen(PossiblePath {
        path: CleanPathResult {
            path: token,
            line_and_column_num: None,
        },
        range: Point { row: 0, col: 0 }..=Point {
            row: 0,
            col: end_col,
        },
    });

    let link = TerminalView::compute_valid_paths(
        dir.path().to_str().unwrap(),
        iter::once(candidate),
        1000,
        None,
    )
    .expect("the markdown file should be detected as a link");

    let GridHighlightedLink::File(file_link) = link else {
        panic!("expected a file link");
    };
    let file_link = file_link.get_inner();

    assert_eq!(
        file_link.absolute_path.file_name().unwrap(),
        "warp-md-test.md"
    );
    assert_eq!(
        *file_link.link.range().end(),
        Point {
            row: 0,
            col: end_col - punctuation_width,
        }
    );
}

#[test]
fn compute_valid_paths_keeps_trailing_fullwidth_punctuation_when_it_is_the_filename() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("warp-md-test.md，");
    std::fs::write(&file, "# Hello\n").unwrap();

    let token = file.to_string_lossy().to_string();
    let end_col = token.chars().count();
    let candidate = WithinModel::AltScreen(PossiblePath {
        path: CleanPathResult {
            path: token,
            line_and_column_num: None,
        },
        range: Point { row: 0, col: 0 }..=Point {
            row: 0,
            col: end_col,
        },
    });

    let link = TerminalView::compute_valid_paths(
        dir.path().to_str().unwrap(),
        iter::once(candidate),
        1000,
        None,
    )
    .expect("the file with fullwidth punctuation should be detected as a link");

    let GridHighlightedLink::File(file_link) = link else {
        panic!("expected a file link");
    };
    let file_link = file_link.get_inner();

    assert_eq!(
        file_link.absolute_path.file_name().unwrap(),
        "warp-md-test.md，"
    );
    assert_eq!(
        *file_link.link.range().end(),
        Point {
            row: 0,
            col: end_col,
        }
    );
}

use std::cell::RefCell;
use std::rc::Rc;

use warp_core::features::FeatureFlag;
use warpui::event::ModifiersState;
use warpui::{App, TypedActionView, ViewHandle};

use crate::terminal::GridType;
use crate::terminal::model::terminal_model::{BlockIndex, WithinBlock};
use crate::terminal::view::TerminalAction;
use crate::test_util::terminal::{
    add_window_with_id_and_terminal, initialize_app_for_terminal_view,
};

async fn await_initial_resize(app: &mut App, terminal: &ViewHandle<TerminalView>) {
    let (tx, rx) = async_channel::unbounded();
    app.update(|ctx| {
        ctx.subscribe_to_view(terminal, move |_, event, _| {
            if matches!(event, crate::terminal::view::Event::Resize { .. }) {
                let _ = tx.try_send(());
            }
        });
    });
    rx.recv().await.unwrap();
}

fn opened_urls(app: &mut App, terminal: &ViewHandle<TerminalView>) -> Rc<RefCell<Vec<String>>> {
    let urls = Rc::new(RefCell::new(Vec::new()));
    let recorded = urls.clone();
    let model = terminal.read(app, |view, _| view.model.clone());
    app.update(|ctx| {
        ctx.set_before_open_url(move |url, _| {
            assert!(
                model.try_lock().is_some(),
                "opener must run outside the model lock"
            );
            recorded.borrow_mut().push(url.to_owned());
            url.to_owned()
        });
    });
    urls
}

fn output_point() -> WithinModel<Point> {
    WithinModel::BlockList(WithinBlock::new(
        Point { row: 0, col: 2 },
        BlockIndex(0),
        GridType::Output,
    ))
}

#[test]
fn l0_06_url_hover_and_plain_click_do_not_open_but_cmd_click_opens_once() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        let urls = opened_urls(&mut app, &terminal);
        terminal.update(&mut app, |view, ctx| {
            view.model
                .lock()
                .simulate_long_running_block("printf", "https://example.invalid/one");
            view.handle_action(
                &TerminalAction::MaybeLinkHover {
                    position: Some(output_point()),
                },
                ctx,
            );
            assert!(view.has_highlighted_link());
            assert_eq!(
                view.links.highlighted.as_ref().unwrap().label(),
                "https://example.invalid/one"
            );
            view.handle_action(
                &TerminalAction::ClickOnGrid {
                    position: output_point(),
                    modifiers: ModifiersState::default(),
                },
                ctx,
            );
        });
        assert!(urls.borrow().is_empty());
        terminal.update(&mut app, |view, ctx| {
            view.handle_action(
                &TerminalAction::ClickOnGrid {
                    position: output_point(),
                    modifiers: ModifiersState {
                        cmd: true,
                        ..Default::default()
                    },
                },
                ctx,
            );
        });
        assert_eq!(*urls.borrow(), ["https://example.invalid/one"]);
    });
}

#[test]
fn l0_06_osc8_uses_true_target_and_rejects_dangerous_schemes() {
    let _flag = FeatureFlag::OscHyperlinks.override_enabled(true);
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        let urls = opened_urls(&mut app, &terminal);
        terminal.update(&mut app, |view, ctx| {
            view.model.lock().process_bytes(
                b"\x1b[?1049h\x1b]8;;https://example.invalid/target\x1b\\label\x1b]8;;\x1b\\"
                    .as_slice(),
            );
            let position = WithinModel::AltScreen(Point { row: 0, col: 1 });
            view.hover_link(Some(position), None, ctx);
            assert_eq!(
                view.links.highlighted.as_ref().unwrap().label(),
                "https://example.invalid/target"
            );
            assert!(urls.borrow().is_empty());
            view.hover_link(
                Some(position),
                Some(ModifiersState {
                    cmd: true,
                    ..Default::default()
                }),
                ctx,
            );
        });
        assert_eq!(*urls.borrow(), ["https://example.invalid/target"]);
        terminal.update(&mut app, |view, ctx| {
            view.model.lock().process_bytes(
                b"\r\x1b[2K\x1b]8;;javascript:alert(1)\x1b\\label\x1b]8;;\x1b\\".as_slice(),
            );
            view.hover_link(
                Some(WithinModel::AltScreen(Point { row: 0, col: 1 })),
                Some(ModifiersState {
                    cmd: true,
                    ..Default::default()
                }),
                ctx,
            );
            assert!(!view.has_highlighted_link());
        });
        assert_eq!(urls.borrow().len(), 1);
        assert!(!super::allowed_url("data:text/html,hello"));
        assert!(!super::allowed_url("warp://action"));
        assert!(!super::allowed_url("file:///tmp/secret"));
    });
}

#[test]
fn l0_06_pending_tooltip_cannot_open_replaced_output_or_session() {
    let _flag = FeatureFlag::OscHyperlinks.override_enabled(true);
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        let urls = opened_urls(&mut app, &terminal);
        terminal.update(&mut app, |view, ctx| {
            view.model
                .lock()
                .process_bytes(b"\x1b[?1049hhttps://example.invalid/one".as_slice());
            let position = WithinModel::AltScreen(Point { row: 0, col: 1 });
            view.hover_link(Some(position), None, ctx);
            let generation = view.links.generation;
            view.model
                .lock()
                .process_bytes(b"\r\x1b[2Khttps://example.invalid/two".as_slice());
            view.open_grid_link(generation, ctx);
            assert!(urls.borrow().is_empty());
            view.hover_link(Some(position), None, ctx);
            let generation = view.links.generation;
            view.model_events.update(ctx, |events, _| {
                events.set_active_session_id(crate::terminal::model::session::SessionId::default());
            });
            view.open_grid_link(generation, ctx);
        });
        assert!(urls.borrow().is_empty());
    });
}

#[test]
fn l0_06_dragging_suppresses_link_activation() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        let urls = opened_urls(&mut app, &terminal);
        terminal.update(&mut app, |view, ctx| {
            view.model
                .lock()
                .simulate_long_running_block("printf", "https://example.invalid/one");
            view.hover_link(Some(output_point()), None, ctx);
            view.links.dragged = true;
            view.handle_action(
                &TerminalAction::ClickOnGrid {
                    position: output_point(),
                    modifiers: ModifiersState {
                        cmd: true,
                        ..Default::default()
                    },
                },
                ctx,
            );
        });
        assert!(urls.borrow().is_empty());
    });
}

fn file_open_position(
    editor: crate::util::file::external_editor::settings::EditorChoice,
    expected_column: usize,
) {
    App::test((), move |mut app| async move {
        use settings::Setting;
        use warpui::SingletonEntity;
        initialize_app_for_terminal_view(&mut app);
        app.update(|ctx| {
            super::EditorSettings::handle(ctx).update(ctx, |settings, ctx| {
                settings.open_file_editor.set_value(editor, ctx).unwrap();
            });
        });
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("a file.rs");
        std::fs::write(&file, "one\ntwo\n").unwrap();
        let mut block = crate::terminal::model::block::SerializedBlock::new_for_test(
            b"compile".to_vec(),
            b"a file.rs:7:3".to_vec(),
        );
        block.pwd = Some(directory.path().to_str().unwrap().to_owned());
        block.is_local = Some(true);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, Some(&[block.into()]));
        await_initial_resize(&mut app, &terminal).await;
        let (tx, rx) = async_channel::unbounded();
        let model = terminal.read(&app, |view, _| view.model.clone());
        app.update(|ctx| {
            ctx.subscribe_to_view(&terminal, move |_, event, _| {
                if let crate::terminal::view::Event::OpenFileWithTarget { path, line_col, .. } =
                    event
                {
                    assert!(model.try_lock().is_some());
                    tx.try_send((path.clone(), *line_col)).unwrap();
                }
            });
        });
        let scan = terminal.update(&mut app, |view, ctx| {
            assert_ne!(view.pwd().as_deref(), directory.path().to_str());
            let position = WithinModel::BlockList(WithinBlock::new(
                Point { row: 0, col: 5 },
                BlockIndex(0),
                GridType::Output,
            ));
            let paths = view
                .model
                .lock()
                .possible_file_paths_at_point(position)
                .collect::<Vec<_>>();
            assert!(
                TerminalView::compute_valid_paths(
                    directory.path().to_str().unwrap(),
                    paths.into_iter(),
                    view.size_info.columns,
                    None
                )
                .is_some(),
                "model file candidates must resolve"
            );
            use warpui::text::SelectionType;

            use crate::terminal::model::blocks::BlockListPoint;
            use crate::terminal::model::index::Side;
            use crate::terminal::model::selection::SelectAction;
            let point = BlockListPoint::from_within_block_point(
                &WithinBlock::new(Point { row: 0, col: 5 }, BlockIndex(0), GridType::Output),
                view.model.lock().block_list(),
            );
            view.handle_action(
                &TerminalAction::SelectOutput(SelectAction::Begin {
                    point,
                    side: Side::Left,
                    selection_type: SelectionType::Simple,
                    position: pathfinder_geometry::vector::Vector2F::zero(),
                }),
                ctx,
            );
            view.hover_link(
                Some(position),
                Some(ModifiersState {
                    cmd: true,
                    ..Default::default()
                }),
                ctx,
            );
            let scan = view.links.scan.as_ref().unwrap().future_id();
            view.handle_action(&TerminalAction::FinishSelection, ctx);
            scan
        });
        app.update(|ctx| ctx.await_spawned_future(scan)).await;
        assert_eq!(
            rx.try_recv().unwrap(),
            (
                file,
                Some(warp_util::path::LineAndColumnArg {
                    line_num: 7,
                    column_num: Some(expected_column),
                })
            )
        );
        assert!(rx.is_empty());
    });
}

#[test]
fn l0_06_file_open_uses_historical_cwd_spaces_and_line_column() {
    file_open_position(
        crate::util::file::external_editor::settings::EditorChoice::SystemDefault,
        3,
    );
}

#[test]
fn l0_06_file_link_converts_one_based_column_for_builtin_editor() {
    file_open_position(
        crate::util::file::external_editor::settings::EditorChoice::Warp,
        2,
    );
}

#[test]
fn l0_06_file_scan_is_cancelled_when_pointer_leaves() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("file.rs"), "one").unwrap();
        let mut block = crate::terminal::model::block::SerializedBlock::new_for_test(
            b"compile".to_vec(),
            b"file.rs".to_vec(),
        );
        block.pwd = Some(directory.path().to_str().unwrap().to_owned());
        block.is_local = Some(true);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, Some(&[block.into()]));
        let scan = terminal.update(&mut app, |view, ctx| {
            view.hover_link(Some(output_point()), None, ctx);
            let scan = view.links.scan.as_ref().unwrap().future_id();
            view.hover_link(None, None, ctx);
            scan
        });
        app.update(|ctx| ctx.await_spawned_future(scan)).await;
        assert!(!terminal.read(&app, |view, _| view.has_highlighted_link()));
    });
}

#[test]
fn l0_06_remote_file_output_never_starts_a_local_file_scan() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let mut block = crate::terminal::model::block::SerializedBlock::new_for_test(
            b"compile".to_vec(),
            b"/etc/hosts".to_vec(),
        );
        block.pwd = Some("/".to_owned());
        block.is_local = Some(false);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, Some(&[block.into()]));
        terminal.update(&mut app, |view, ctx| {
            view.hover_link(
                Some(output_point()),
                Some(ModifiersState {
                    cmd: true,
                    ..Default::default()
                }),
                ctx,
            );
            assert!(view.links.scan.is_none());
            assert!(!view.has_highlighted_link());
        });
    });
}

fn platform_link_interaction(alt_screen: bool) {
    use pathfinder_geometry::vector::vec2f;
    use warpui::presenter::Presenter;
    use warpui::{EntityIdSet, Event as UiEvent, WindowInvalidation};

    App::test((), move |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let block = crate::terminal::model::block::SerializedBlock::new_for_test(
            b"printf".to_vec(),
            b"https://example.invalid/one\r\nhttps://example.invalid/one\r\nhttps://example.invalid/one\r\nhttps://example.invalid/one\r\nhttps://example.invalid/one\r\nhttps://example.invalid/one".to_vec(),
        );
        let (window, terminal) = add_window_with_id_and_terminal(&mut app, Some(&[block.into()]));
        await_initial_resize(&mut app, &terminal).await;
        if alt_screen {
            terminal.update(&mut app, |view, ctx| {
                view.model
                    .lock()
                    .process_bytes("\x1b[?1049h\x1b[Hhttps://example.invalid/one");
                assert!(
                    view.model
                        .lock()
                        .url_at_point(&WithinModel::AltScreen(Point { row: 0, col: 1 }))
                        .is_some(),
                    "alt fixture contains URL before layout"
                );
                ctx.focus_self();
            });
        }
        terminal.update(&mut app, |view, _| {
            use warpui::units::IntoPixels;
            let size = crate::terminal::SizeInfo::new(
                vec2f(800., 600.),
                10_f32.into_pixels(),
                20_f32.into_pixels(),
                0_f32.into_pixels(),
                0_f32.into_pixels(),
            );
            view.model.lock().resize(crate::terminal::SizeUpdate {
                update_reason: crate::terminal::SizeUpdateReason::AfterLayout,
                last_size: view.size_info,
                new_size: size,
                new_gap_height: None,
                natural_rows: size.rows(),
                natural_cols: size.columns(),
            });
            view.size_info = size;
        });
        let urls = opened_urls(&mut app, &terminal);
        let mut presenter = Presenter::new(window);
        let mut updated = EntityIdSet::default();
        updated.insert(app.root_view_id(window).unwrap());
        updated.insert(terminal.id());
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
        let send = |app: &mut App, event| {
            let presenter = presenter.clone();
            app.update(move |ctx| ctx.simulate_window_event(event, window, presenter));
        };
        let point = if alt_screen {
            vec2f(10., 5.)
        } else {
            vec2f(10., 556.)
        };
        send(
            &mut app,
            UiEvent::MouseMoved {
                position: point,
                cmd: false,
                shift: false,
                is_synthetic: false,
            },
        );
        terminal.read(&app, |view, _| {
            assert!(
                view.has_highlighted_link(),
                "platform hover missing: position={:?}, selecting={}, generation={}",
                view.links.position,
                view.is_selecting,
                view.links.generation
            );
        });
        assert!(urls.borrow().is_empty());
        send(
            &mut app,
            UiEvent::LeftMouseDown {
                position: point,
                modifiers: ModifiersState::default(),
                click_count: 1,
                is_first_mouse: false,
            },
        );
        send(
            &mut app,
            UiEvent::LeftMouseUp {
                position: point,
                modifiers: ModifiersState::default(),
            },
        );
        assert!(urls.borrow().is_empty());
        let cmd = ModifiersState {
            cmd: true,
            ..Default::default()
        };
        send(
            &mut app,
            UiEvent::LeftMouseDown {
                position: point,
                modifiers: cmd,
                click_count: 1,
                is_first_mouse: false,
            },
        );
        send(
            &mut app,
            UiEvent::LeftMouseUp {
                position: point,
                modifiers: cmd,
            },
        );
        assert_eq!(*urls.borrow(), ["https://example.invalid/one"]);
        send(
            &mut app,
            UiEvent::LeftMouseDown {
                position: point,
                modifiers: cmd,
                click_count: 1,
                is_first_mouse: false,
            },
        );
        let end = point + vec2f(150., 0.);
        send(
            &mut app,
            UiEvent::LeftMouseDragged {
                position: end,
                modifiers: cmd,
            },
        );
        send(
            &mut app,
            UiEvent::LeftMouseUp {
                position: end,
                modifiers: cmd,
            },
        );
        assert_eq!(urls.borrow().len(), 1, "drag release must not open a link");
        assert!(terminal.read(&app, |view, ctx| {
            view.selected_text(ctx).is_some_and(|text| !text.is_empty())
        }));
    });
}

#[test]
fn l0_06_transcript_platform_mouse_hover_click_and_drag() {
    platform_link_interaction(false);
}

#[test]
fn l0_06_alt_screen_platform_mouse_hover_click_and_drag() {
    platform_link_interaction(true);
}

#[test]
fn l0_06_snapshot_preserves_known_local_remote_and_unknown_origins() {
    use crate::terminal::model::session::{BootstrapSessionType, SessionId, SessionInfo};

    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        let local = SessionInfo::new_for_test();
        let mut remote =
            SessionInfo::new_for_test().with_session_type(BootstrapSessionType::WarpifiedRemote);
        remote.session_id = SessionId::from(1);
        let serialized = terminal.update(&mut app, |view, ctx| {
            view.sessions.update(ctx, |sessions, _| {
                sessions.register_session_for_test(local);
                sessions.register_session_for_test(remote);
            });
            let mut model = view.model.lock();
            model.simulate_block("local", "local");
            model.simulate_block("remote", "remote");
            model.simulate_block("unknown", "unknown");
            let block = model.block_list_mut().block_at_mut(BlockIndex(0)).unwrap();
            block.set_session_id(SessionId::from(0));
            let local = view.serialize_block(block, ctx);
            let block = model.block_list_mut().block_at_mut(BlockIndex(1)).unwrap();
            block.set_session_id(SessionId::from(1));
            let remote = view.serialize_block(block, ctx);
            let block = model.block_list_mut().block_at_mut(BlockIndex(2)).unwrap();
            block.set_session_id(SessionId::from(2));
            let unknown = view.serialize_block(block, ctx);
            assert_eq!(local.is_local, Some(true));
            assert_eq!(remote.is_local, Some(false));
            assert_eq!(unknown.is_local, None);
            vec![local.into(), remote.into(), unknown.into()]
        });

        let (_, restored) = add_window_with_id_and_terminal(&mut app, Some(&serialized));
        restored.read(&app, |view, ctx| {
            let model = view.model.lock();
            assert_eq!(
                view.serialize_block(model.block_list().block_at(BlockIndex(0)).unwrap(), ctx)
                    .is_local,
                Some(true)
            );
            assert_eq!(
                view.serialize_block(model.block_list().block_at(BlockIndex(1)).unwrap(), ctx)
                    .is_local,
                Some(false)
            );
            assert_eq!(
                view.serialize_block(model.block_list().block_at(BlockIndex(2)).unwrap(), ctx)
                    .is_local,
                None
            );
        });
    });
}
