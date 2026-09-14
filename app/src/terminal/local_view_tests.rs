use warpui::App;
use warpui::keymap::Keystroke;
use warpui::units::IntoPixels;

use super::*;
use crate::terminal::model::ansi::{Handler, Mode};
use crate::terminal::model::block::SerializedBlock;
use crate::terminal::model::index::Point;
use crate::terminal::model::mouse::{MouseAction, MouseButton, MouseState};
use crate::test_util::terminal::{
    add_window_with_id_and_terminal, initialize_app_for_terminal_view,
};
use crate::view_components::find::{FindAction, FindModel};

#[test]
fn alt_screen_wheel_and_mouse_events_reach_pty_without_holding_model_lock() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        let model = terminal.read(&app, |view, _| view.model.clone());
        let (tx, rx) = async_channel::unbounded();
        app.update(|ctx| {
            let model = model.clone();
            ctx.subscribe_to_view(&terminal, move |_, event, _| {
                if let Event::WriteBytesToPty { bytes } = event {
                    assert!(model.try_lock().is_some());
                    tx.try_send(bytes.to_vec()).unwrap();
                }
            });
        });
        terminal.update(&mut app, |view, ctx| {
            view.handle_action(
                &TerminalAction::AltScroll {
                    delta: 2,
                    point: Point::new(1, 2),
                },
                ctx,
            );
        });
        assert_eq!(rx.try_recv().unwrap(), b"\x1bOA\x1bOA");
        {
            let mut model = model.try_lock().unwrap();
            model.set_mode(Mode::SwapScreen {
                save_cursor_and_clear_screen: true,
            });
            model.set_mode(Mode::SgrMouse);
        }
        terminal.update(&mut app, |view, ctx| {
            view.handle_action(
                &TerminalAction::AltScroll {
                    delta: -1,
                    point: Point::new(1, 2),
                },
                ctx,
            );
            view.handle_action(
                &TerminalAction::AltMouseAction(
                    MouseState::new(MouseButton::Left, MouseAction::Pressed, Default::default())
                        .set_point(Point::new(1, 2)),
                ),
                ctx,
            );
        });
        assert_eq!(rx.try_recv().unwrap(), b"\x1b[<65;3;2M");
        assert_eq!(rx.try_recv().unwrap(), b"\x1b[<0;3;2M");
        terminal.update(&mut app, |view, ctx| {
            view.handle_action(
                &TerminalAction::AltScroll {
                    delta: 0,
                    point: Point::new(1, 2),
                },
                ctx,
            );
        });
        assert!(rx.try_recv().is_err());
    });
}

#[test]
fn terminal_find_searches_output_scrolls_and_closes() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let output = (0..120)
            .map(|line| {
                if line == 60 || line == 110 {
                    "TARGET\r\n"
                } else {
                    "ordinary output\r\n"
                }
            })
            .collect::<String>();
        let block = SerializedBlock::new_for_test(b"print rows".to_vec(), output.into_bytes());
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, Some(&[block.into()]));
        let find_bar = terminal.update(&mut app, |view, ctx| {
            view.handle_action(&TerminalAction::ShowFindBar, ctx);
            assert!(view.find_bar_open);
            view.find_bar.clone()
        });
        find_bar.update(&mut app, |bar, ctx| bar.set_query_text("target", ctx));
        terminal.read(&app, |view, ctx| {
            assert_eq!(view.find_model.as_ref(ctx).match_count(), 2)
        });
        find_bar.update(&mut app, |bar, ctx| {
            bar.handle_action(&FindAction::Down, ctx)
        });
        terminal.read(&app, |view, _| {
            assert!(view.transcript_scroll.scroll_start().as_f32() > 0.)
        });
        find_bar.update(&mut app, |bar, ctx| {
            bar.handle_action(&FindAction::ToggleCaseSensitivity, ctx)
        });
        terminal.read(&app, |view, ctx| {
            assert_eq!(view.find_model.as_ref(ctx).match_count(), 0)
        });
        find_bar.update(&mut app, |bar, ctx| {
            bar.handle_action(&FindAction::ToggleCaseSensitivity, ctx)
        });
        find_bar.update(&mut app, |bar, ctx| {
            bar.handle_action(&FindAction::Close, ctx)
        });
        terminal.read(&app, |view, _| assert!(!view.find_bar_open));
        terminal.update(&mut app, |view, ctx| {
            view.handle_action(&TerminalAction::ShowFindBar, ctx)
        });
        terminal.read(&app, |view, ctx| {
            assert_eq!(view.find_model.as_ref(ctx).match_count(), 2)
        });
    });
}

#[test]
fn page_keys_scroll_transcript_while_input_editor_is_focused() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let block = SerializedBlock::new_for_test(
            b"seq 1 100".to_vec(),
            (1..=100)
                .map(|line| format!("{line}\r\n"))
                .collect::<String>()
                .into_bytes(),
        );
        let (window_id, terminal) =
            add_window_with_id_and_terminal(&mut app, Some(&[block.into()]));
        let (input, editor) = terminal.read(&app, |view, ctx| {
            (view.input.clone(), view.input.as_ref(ctx).editor().clone())
        });
        terminal.update(&mut app, |view, ctx| {
            view.transcript_scroll.scroll_to(800.0.into_pixels());
            view.focus(ctx);
        });
        let before = terminal.read(&app, |view, _| view.transcript_scroll.scroll_start());
        assert!(before.as_f32() > 0.);
        assert!(
            app.dispatch_keystroke(
                window_id,
                &[terminal.id(), input.id(), editor.id()],
                &Keystroke::parse("pageup").unwrap(),
                false
            )
            .unwrap()
        );
        let after = terminal.read(&app, |view, _| view.transcript_scroll.scroll_start());
        assert!(
            after < before,
            "PageUp should scroll transcript towards its start"
        );
        assert!(
            app.dispatch_keystroke(
                window_id,
                &[terminal.id(), input.id(), editor.id()],
                &Keystroke::parse("pagedown").unwrap(),
                false
            )
            .unwrap()
        );
        terminal.read(&app, |view, ctx| {
            assert!(view.transcript_scroll.scroll_start() > after);
            assert!(view.input.as_ref(ctx).buffer_text(ctx).is_empty());
        });
    });
}

#[test]
fn commands_submitted_during_bootstrap_wait_for_the_shell() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        let (tx, rx) = async_channel::unbounded();
        app.update(|ctx| {
            ctx.subscribe_to_view(&terminal, move |_, event, _| {
                if let Event::ExecuteCommand(event) = event {
                    tx.try_send(event.command.clone()).unwrap();
                }
            });
        });
        terminal.update(&mut app, |view, ctx| {
            view.handle_input_event(&InputEvent::ExecuteCommand("echo queued".to_owned()), ctx);
        });
        assert!(
            rx.try_recv().is_err(),
            "An uninitialized shell cannot execute the command"
        );
        terminal.update(&mut app, |view, ctx| {
            view.handle_model_event(
                &ModelEvent::Handler(AnsiHandlerEvent::Bootstrapped {
                    session_id: Default::default(),
                    is_subshell: false,
                }),
                ctx,
            );
        });
        assert_eq!(rx.try_recv().unwrap(), "echo queued");
        assert!(
            rx.try_recv().is_err(),
            "The queued command must execute exactly once"
        );
    });
}

#[test]
fn prompt_tracks_directory_branch_and_leaving_a_repository() {
    use crate::terminal::model::ansi::PromptMetadata;

    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        terminal.update(&mut app, |view, _| {
            view.model
                .lock()
                .block_list_mut()
                .active_block_mut()
                .prompt_only_precmd(PromptMetadata {
                    pwd: Some("/tmp/project".into()),
                    git_head: Some("main".into()),
                    git_branch: Some("main".into()),
                    ..Default::default()
                });
        });
        terminal.read(&app, |view, ctx| {
            assert_eq!(view.full_prompt(ctx), "/tmp/project (main) %")
        });
        terminal.update(&mut app, |view, _| {
            view.model
                .lock()
                .block_list_mut()
                .active_block_mut()
                .prompt_only_precmd(PromptMetadata {
                    pwd: Some("/tmp".into()),
                    ..Default::default()
                });
        });
        terminal.read(&app, |view, ctx| {
            assert_eq!(view.full_prompt(ctx), "/tmp %")
        });
    });
}

#[test]
fn clear_moves_output_into_scrollback_without_deleting_history() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let block = SerializedBlock::new_for_test(b"echo old".to_vec(), b"old output\r\n".to_vec());
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, Some(&[block.into()]));
        terminal.update(&mut app, |view, ctx| {
            view.handle_wakeup(None, ctx);
            let before_height = view.transcript_height;
            let before_blocks = view.model.lock().block_list().blocks().len();
            view.model.lock().clear_visible_screen();
            view.handle_model_event(&ModelEvent::TerminalClear, ctx);
            assert_eq!(view.model.lock().block_list().blocks().len(), before_blocks);
            assert!(view.transcript_height >= before_height + view.size_info.pane_height_px);
            assert!(view.transcript_scroll.scroll_start().as_f32() >= before_height);
            let height = view.transcript_height;
            view.model.lock().clear_visible_screen();
            view.handle_model_event(&ModelEvent::TerminalClear, ctx);
            assert_eq!(
                view.transcript_height, height,
                "repeated clear must replace the old gap"
            );
        });
    });
}

#[test]
fn clicking_output_without_dragging_returns_focus_to_the_input() {
    use warpui::text::SelectionType;

    use crate::terminal::model::index::Side;

    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let block = SerializedBlock::new_for_test(b"echo old".to_vec(), b"old output\r\n".to_vec());
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, Some(&[block.into()]));
        terminal.update(&mut app, |view, ctx| {
            let point = BlockListPoint::from_within_block_point(
                &WithinBlock::new(Point::new(0, 0), BlockIndex(0), GridType::Output),
                view.model.lock().block_list(),
            );
            view.handle_action(
                &TerminalAction::SelectOutput(SelectAction::Begin {
                    point,
                    side: Side::Left,
                    selection_type: SelectionType::Simple,
                    position: Vector2F::zero(),
                }),
                ctx,
            );
            view.handle_action(&TerminalAction::SelectOutput(SelectAction::End), ctx);
        });
        terminal.read(&app, |view, ctx| {
            assert!(view.input.as_ref(ctx).editor().is_focused(ctx));
            assert!(view.model.lock().block_list().selection().is_none());
            assert!(view.input.as_ref(ctx).buffer_text(ctx).is_empty());
        });
    });
}

/// Collects the labels of a built menu, with `None` standing in for separators, so tests can
/// assert on both the entries and where the sections break.
fn menu_labels(items: &[MenuItem<TerminalAction>]) -> Vec<Option<String>> {
    items
        .iter()
        .map(|item| match item {
            MenuItem::Item(fields) => Some(fields.label().to_owned()),
            MenuItem::Separator => None,
            _ => Some(String::from("<unsupported>")),
        })
        .collect()
}

fn disabled_labels(items: &[MenuItem<TerminalAction>]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| match item {
            MenuItem::Item(fields) if fields.is_disabled() => Some(fields.label().to_owned()),
            _ => None,
        })
        .collect()
}

#[test]
fn right_clicking_a_block_opens_a_menu_with_local_entries_only() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let block = SerializedBlock::new_for_test(b"echo hi".to_vec(), b"hi\r\n".to_vec());
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, Some(&[block.into()]));

        terminal.update(&mut app, |view, ctx| {
            view.handle_action(
                &TerminalAction::BlockContextMenu {
                    position: Vector2F::new(12., 34.),
                    block_index: Some(BlockIndex(0)),
                },
                ctx,
            );
        });

        terminal.read(&app, |view, _| {
            let state = view
                .context_menu_state
                .as_ref()
                .expect("right-click should open the context menu");
            assert_eq!(state.position, Vector2F::new(12., 34.));
        });

        let labels = terminal.update(&mut app, |view, ctx| {
            menu_labels(&view.block_context_menu_items(Some(BlockIndex(0)), ctx))
        });
        let present: Vec<&str> = labels.iter().filter_map(|label| label.as_deref()).collect();
        for expected in [
            "Copy",
            "Paste",
            "Copy command",
            "Copy output",
            "Copy block",
            "Find in terminal",
            "Clear buffer",
        ] {
            assert!(
                present.contains(&expected),
                "missing {expected:?} in {present:?}"
            );
        }
        // Route A removed sharing, Warp Drive and AI; the restored menu must not bring them back.
        for forbidden in ["Share", "Ask", "Agent", "workflow", "Drive"] {
            assert!(
                !present.iter().any(|label| label.contains(forbidden)),
                "unexpected cloud/AI entry matching {forbidden:?} in {present:?}"
            );
        }
    });
}

#[test]
fn block_menu_copies_command_output_and_both() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let block = SerializedBlock::new_for_test(b"echo hi".to_vec(), b"hi\r\n".to_vec());
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, Some(&[block.into()]));

        for (action, expected) in [
            (TerminalAction::CopyBlockCommand(BlockIndex(0)), "echo hi"),
            (TerminalAction::CopyBlockOutput(BlockIndex(0)), "hi"),
            (TerminalAction::CopyBlock(BlockIndex(0)), "echo hi\nhi"),
        ] {
            terminal.update(&mut app, |view, ctx| view.handle_action(&action, ctx));
            let copied = app.update(|ctx| ctx.clipboard().read().plain_text);
            assert_eq!(copied.trim_end(), expected, "for {action:?}");
        }
    });
}

#[test]
fn copy_entries_are_disabled_without_a_selection_or_block_text() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let block = SerializedBlock::new_for_test(Vec::new(), Vec::new());
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, Some(&[block.into()]));

        let disabled = terminal.update(&mut app, |view, ctx| {
            disabled_labels(&view.block_context_menu_items(Some(BlockIndex(0)), ctx))
        });
        for expected in ["Copy", "Copy command", "Copy output", "Copy block"] {
            assert!(
                disabled.iter().any(|label| label == expected),
                "expected {expected:?} to be disabled, got {disabled:?}"
            );
        }
    });
}

#[test]
fn alt_screen_right_click_opens_a_menu_that_close_dismisses() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);

        let labels = terminal.update(&mut app, |view, ctx| {
            view.handle_action(
                &TerminalAction::AltScreenContextMenu {
                    position: Vector2F::new(4., 8.),
                },
                ctx,
            );
            menu_labels(&view.alt_screen_context_menu_items(ctx))
        });
        assert!(
            terminal.read(&app, |view, _| view.context_menu_state.is_some()),
            "alt-screen right-click should no longer be a no-op"
        );
        // The transcript is not addressable on the alt screen, so no block entries here.
        let present: Vec<&str> = labels.iter().filter_map(|label| label.as_deref()).collect();
        assert!(present.contains(&"Paste"), "{present:?}");
        assert!(
            !present
                .iter()
                .any(|label| label.starts_with("Copy command"))
        );

        terminal.update(&mut app, |view, ctx| {
            view.handle_action(&TerminalAction::CloseContextMenu, ctx);
        });
        assert!(terminal.read(&app, |view, _| view.context_menu_state.is_none()));
    });
}

#[test]
fn insert_into_input_moves_the_selection_into_the_input_box() {
    use warpui::text::SelectionType;

    use crate::terminal::model::index::Side;

    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let block = SerializedBlock::new_for_test(b"echo old".to_vec(), b"old output\r\n".to_vec());
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, Some(&[block.into()]));

        terminal.update(&mut app, |view, ctx| {
            let (start, end) = {
                let model = view.model.lock();
                let point_at = |col| {
                    BlockListPoint::from_within_block_point(
                        &WithinBlock::new(Point::new(0, col), BlockIndex(0), GridType::Output),
                        model.block_list(),
                    )
                };
                (point_at(0), point_at(2))
            };
            view.handle_action(
                &TerminalAction::SelectOutput(SelectAction::Begin {
                    point: start,
                    side: Side::Left,
                    selection_type: SelectionType::Simple,
                    position: Vector2F::zero(),
                }),
                ctx,
            );
            view.handle_action(
                &TerminalAction::SelectOutput(SelectAction::Update {
                    point: end,
                    side: Side::Right,
                    delta: warpui::units::Lines::zero(),
                    position: Vector2F::zero(),
                }),
                ctx,
            );
            view.handle_action(&TerminalAction::InsertSelectedTextIntoInput, ctx);
        });

        terminal.read(&app, |view, ctx| {
            assert_eq!(view.input.as_ref(ctx).buffer_text(ctx), "old");
            assert!(
                view.model.lock().block_list().selection().is_none(),
                "inserting should consume the selection"
            );
        });
    });
}

/// Drives real right-clicks through the element tree instead of calling `handle_action`
/// directly, so the wiring between the elements and the view is covered. Regression test for
/// the first restore attempt, where only glyph cells inside a block reacted and every gap,
/// prompt row and empty area below the transcript stayed dead.
#[test]
fn right_mouse_down_opens_the_menu_anywhere_in_the_transcript() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use pathfinder_geometry::vector::vec2f;
    use warpui::presenter::Presenter;
    use warpui::{EntityIdSet, Event as UiEvent, WindowInvalidation};

    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let block = SerializedBlock::new_for_test(b"echo hi".to_vec(), b"hi\r\n".to_vec());
        let (window_id, terminal) =
            add_window_with_id_and_terminal(&mut app, Some(&[block.into()]));

        let mut presenter = Presenter::new(window_id);
        let mut updated = EntityIdSet::default();
        updated.insert(app.root_view_id(window_id).unwrap());
        updated.insert(terminal.id());
        let invalidation = WindowInvalidation {
            updated,
            ..Default::default()
        };
        let presenter = app.update(move |ctx| {
            presenter.invalidate(invalidation, ctx);
            presenter.build_scene(vec2f(800., 600.), 1., None, ctx);
            Rc::new(RefCell::new(presenter))
        });

        let right_click = |app: &mut warpui::App, position| {
            let presenter = presenter.clone();
            app.update(move |ctx| {
                ctx.simulate_window_event(
                    UiEvent::RightMouseDown {
                        position,
                        cmd: false,
                        shift: false,
                        click_count: 1,
                    },
                    window_id,
                    presenter,
                );
            });
        };

        // Inside a painted block: the menu carries the block-specific entries.
        right_click(&mut app, vec2f(2., 556.));
        assert!(
            terminal.read(&app, |view, _| view.context_menu_state.is_some()),
            "right-click on a block should open the menu"
        );
        terminal.update(&mut app, |view, ctx| {
            view.handle_action(&TerminalAction::CloseContextMenu, ctx)
        });

        // Empty transcript space well above and to the right of any block. This is the case
        // the first attempt missed: no block grid covers these pixels.
        for position in [vec2f(400., 100.), vec2f(600., 300.), vec2f(200., 540.)] {
            right_click(&mut app, position);
            assert!(
                terminal.read(&app, |view, _| view.context_menu_state.is_some()),
                "right-click on empty transcript space at {position:?} should open the menu"
            );
            terminal.update(&mut app, |view, ctx| {
                view.handle_action(&TerminalAction::CloseContextMenu, ctx)
            });
        }
    });
}
