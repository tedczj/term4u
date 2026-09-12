use warpui::App;
use warpui::keymap::Keystroke;
use warpui::units::IntoPixels;

use super::*;
use crate::terminal::model::block::SerializedBlock;
use crate::test_util::terminal::{
    add_window_with_id_and_terminal, initialize_app_for_terminal_view,
};

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
