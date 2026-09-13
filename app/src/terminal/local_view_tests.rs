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
