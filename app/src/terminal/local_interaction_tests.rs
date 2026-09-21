use warpui::App;
use warpui::keymap::Keystroke;

use super::*;
use crate::terminal::{History, HistoryEvent};
use crate::terminal::model::session::SessionInfo;
use crate::test_util::terminal::{
    add_window_with_id_and_terminal, initialize_app_for_terminal_view,
};

#[test]
fn arrow_keys_recall_session_history_and_restore_the_draft() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (window, terminal) = add_window_with_id_and_terminal(&mut app, None);
        let session_info = SessionInfo::new_for_test();
        let session_id = session_info.session_id;
        let session = terminal.update(&mut app, |view, ctx| {
            view.sessions.update(ctx, |sessions, _| {
                sessions.register_session_for_test(session_info);
            });
            view.model_events.update(ctx, |events, _| {
                events.set_active_session_id(session_id);
            });
            view.active_session(ctx).unwrap()
        });
        let (tx, rx) = async_channel::unbounded();
        app.update(|ctx| {
            let history = History::handle(ctx);
            ctx.subscribe_to_model(&history, move |_, event, _| {
                if matches!(event, HistoryEvent::Initialized(id) if *id == session_id) {
                    tx.try_send(()).unwrap();
                }
            });
            history.update(ctx, |history, ctx| {
                history.init_session_with(
                    session,
                    async { vec!["echo previous".to_owned()] },
                    ctx,
                );
            });
        });
        rx.recv().await.unwrap();
        let input = terminal.read(&app, |view, _| view.input().clone());
        let editor = input.read(&app, |input, _| input.editor().clone());
        input.update(&mut app, |input, ctx| {
            input.replace_buffer_content("echo", ctx);
            input.focus_input_box(ctx);
        });
        app.read(|ctx| assert!(editor.is_focused(ctx)));
        app.dispatch_keystroke(
            window,
            &[terminal.id(), input.id(), editor.id()],
            &Keystroke::parse("up").unwrap(),
            false,
        )
        .unwrap();
        input.read(&app, |input, ctx| assert_eq!(input.buffer_text(ctx), "echo previous"));
        app.dispatch_keystroke(
            window,
            &[terminal.id(), input.id(), editor.id()],
            &Keystroke::parse("down").unwrap(),
            false,
        )
        .unwrap();
        input.read(&app, |input, ctx| assert_eq!(input.buffer_text(ctx), "echo"));
    });
}

#[test]
fn multiline_vertical_navigation_stays_inside_the_editor() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (window, terminal) = add_window_with_id_and_terminal(&mut app, None);
        let input = terminal.read(&app, |view, _| view.input().clone());
        let editor = input.read(&app, |input, _| input.editor().clone());
        input.update(&mut app, |input, ctx| {
            input.replace_buffer_content("one\ntwo", ctx);
            input.focus_input_box(ctx);
            input.editor().update(ctx, |editor, ctx| {
                editor.select_ranges_by_byte_offset([7_usize.into()..7_usize.into()], ctx);
            });
        });
        app.dispatch_keystroke(
            window,
            &[terminal.id(), input.id(), editor.id()],
            &Keystroke::parse("up").unwrap(),
            false,
        )
        .unwrap();
        editor.read(&app, |editor, ctx| {
            assert_eq!(editor.buffer_text(ctx), "one\ntwo");
            assert_eq!(editor.end_byte_index_of_last_selection(ctx).as_usize(), 3);
        });
    });
}

#[test]
fn terminal_paste_inserts_at_the_cursor_without_executing() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        let (tx, rx) = async_channel::unbounded();
        app.update(|ctx| {
            ctx.subscribe_to_view(&terminal, move |_, event, _| {
                if let Event::ExecuteCommand(command) = event {
                    tx.try_send(command.command.clone()).unwrap();
                }
            });
        });
        terminal.update(&mut app, |view, ctx| {
            view.input.update(ctx, |input, ctx| {
                input.replace_buffer_content("echo tail", ctx);
                input.editor().update(ctx, |editor, ctx| {
                    editor.select_ranges_by_byte_offset([5_usize.into()..5_usize.into()], ctx);
                });
            });
            ctx.clipboard().write(ClipboardContent::plain_text("new\nline ".to_owned()));
            view.handle_action(&TerminalAction::Paste, ctx);
            assert_eq!(view.input.as_ref(ctx).buffer_text(ctx), "echo new\nline tail");
        });
        assert!(rx.try_recv().is_err());
    });
}

#[test]
fn escape_closes_the_context_menu_and_restores_input_focus() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        app.update(crate::menu::init);
        let (window, terminal) = add_window_with_id_and_terminal(&mut app, None);
        let menu = terminal.update(&mut app, |view, ctx| {
            view.input.update(ctx, |input, ctx| {
                input.replace_buffer_content("echo untouched", ctx);
                input.focus_input_box(ctx);
            });
            view.handle_action(
                &TerminalAction::BlockContextMenu {
                    position: Vector2F::zero(),
                    block_index: None,
                },
                ctx,
            );
            assert!(view.context_menu.is_focused(ctx));
            view.focus(ctx);
            assert!(view.context_menu.is_focused(ctx));
            view.context_menu.clone()
        });
        app.dispatch_keystroke(
            window,
            &[terminal.id(), menu.id()],
            &Keystroke::parse("escape").unwrap(),
            false,
        )
        .unwrap();
        terminal.read(&app, |view, ctx| {
            assert!(view.context_menu_state.is_none());
            assert!(view.input.as_ref(ctx).editor().is_focused(ctx));
            assert_eq!(view.input.as_ref(ctx).buffer_text(ctx), "echo untouched");
        });
    });
}

#[test]
fn menu_enter_runs_the_menu_action_not_the_draft_command() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        app.update(crate::menu::init);
        let block = SerializedBlock::new_for_test(b"echo original".to_vec(), b"out\r\n".to_vec());
        let (window, terminal) =
            add_window_with_id_and_terminal(&mut app, Some(&[block.into()]));
        let (tx, rx) = async_channel::unbounded();
        app.update(|ctx| {
            ctx.subscribe_to_view(&terminal, move |_, event, _| {
                if let Event::ExecuteCommand(command) = event {
                    tx.try_send(command.command.clone()).unwrap();
                }
            });
        });
        let menu = terminal.update(&mut app, |view, ctx| {
            view.is_bootstrapped = true;
            view.input.update(ctx, |input, ctx| {
                input.replace_buffer_content("echo DO_NOT_EXECUTE", ctx);
                input.focus_input_box(ctx);
            });
            view.handle_action(
                &TerminalAction::BlockContextMenu {
                    position: Vector2F::zero(),
                    block_index: Some(BlockIndex(0)),
                },
                ctx,
            );
            view.context_menu.update(ctx, |menu, ctx| {
                assert!(menu.set_selected_by_name("Copy command", ctx));
            });
            assert!(view.context_menu.is_focused(ctx));
            view.context_menu.clone()
        });
        app.dispatch_keystroke(
            window,
            &[terminal.id(), menu.id()],
            &Keystroke::parse("enter").unwrap(),
            false,
        )
        .unwrap();
        terminal.update(&mut app, |view, ctx| {
            assert_eq!(ctx.clipboard().read().plain_text, "echo original");
            assert!(view.context_menu_state.is_none());
            assert_eq!(view.input.as_ref(ctx).buffer_text(ctx), "echo DO_NOT_EXECUTE");
        });
        assert!(rx.try_recv().is_err());
    });
}

#[test]
fn closing_the_menu_does_not_take_focus_back_from_find() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        app.update(crate::menu::init);
        let (window, terminal) = add_window_with_id_and_terminal(&mut app, None);
        let menu = terminal.update(&mut app, |view, ctx| {
            view.handle_action(
                &TerminalAction::BlockContextMenu {
                    position: Vector2F::zero(),
                    block_index: None,
                },
                ctx,
            );
            view.context_menu.update(ctx, |menu, ctx| {
                assert!(menu.set_selected_by_name("Find in terminal", ctx));
            });
            assert!(view.context_menu.is_focused(ctx));
            view.context_menu.clone()
        });
        app.dispatch_keystroke(
            window,
            &[terminal.id(), menu.id()],
            &Keystroke::parse("enter").unwrap(),
            false,
        )
        .unwrap();
        terminal.read(&app, |view, ctx| {
            assert!(view.context_menu_state.is_none());
            assert!(view.find_bar_open);
            assert!(!view.input.as_ref(ctx).editor().is_focused(ctx));
            assert!(!view.context_menu.is_focused(ctx));
        });
    });
}

#[test]
fn dropping_paths_inserts_into_the_draft_without_running_it() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        let (tx, rx) = async_channel::unbounded();
        app.update(|ctx| {
            ctx.subscribe_to_view(&terminal, move |_, event, _| {
                if let Event::ExecuteCommand(command) = event {
                    tx.try_send(command.command.clone()).unwrap();
                }
            });
        });
        terminal.update(&mut app, |view, ctx| {
            view.input.update(ctx, |input, ctx| {
                input.replace_buffer_content("cat  suffix", ctx);
                input.editor().update(ctx, |editor, ctx| {
                    editor.select_ranges_by_byte_offset([4_usize.into()..4_usize.into()], ctx);
                });
            });
            view.handle_action(
                &TerminalAction::DragAndDropFiles(vec![
                    std::path::PathBuf::from("/tmp/one"),
                    std::path::PathBuf::from("/tmp/two"),
                ]),
                ctx,
            );
            assert_eq!(view.input.as_ref(ctx).buffer_text(ctx), "cat /tmp/one /tmp/two suffix");
        });
        assert!(rx.try_recv().is_err());
    });
}
