use typed_path::TypedPathBuf;
use warpui::App;
use warpui::keymap::Keystroke;

use super::*;
use crate::terminal::model::session::command_executor::testing::TestCommandExecutor;
use crate::terminal::model::session::{Session, SessionInfo};
use crate::test_util::terminal::{
    add_window_with_id_and_terminal, initialize_app_for_terminal_view,
};

#[test]
fn tab_completes_a_directory_through_the_terminal_editor() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir(directory.path().join("workspace")).unwrap();
        std::fs::write(directory.path().join("work-file"), "").unwrap();
        let session = Arc::new(Session::new(
            SessionInfo::new_for_test(),
            Arc::new(TestCommandExecutor::default()),
        ));
        let (window, terminal) = add_window_with_id_and_terminal(&mut app, None);
        let input = terminal.read(&app, |view, _| view.input().clone());
        let editor = input.read(&app, |input, _| input.editor().clone());
        let (tx, rx) = async_channel::unbounded();
        let cwd = TypedPathBuf::from(directory.path().to_str().unwrap());
        app.update(|ctx| {
            ctx.subscribe_to_view(&input, move |input, event, ctx| {
                if matches!(event, Event::Complete) {
                    input.update(ctx, |input, ctx| {
                        input.complete(
                            SessionContext::for_terminal(session.clone(), cwd.clone()),
                            ctx,
                        )
                    });
                }
            });
            ctx.subscribe_to_view(&editor, move |editor, event, ctx| {
                if matches!(event, EditorEvent::Edited(_)) {
                    let _ = tx.try_send(editor.as_ref(ctx).buffer_text(ctx));
                }
            });
        });
        input.update(&mut app, |input, ctx| {
            input.replace_buffer_content("cd work", ctx);
            input.focus_input_box(ctx);
        });
        app.dispatch_keystroke(
            window,
            &[terminal.id(), input.id(), editor.id()],
            &Keystroke::parse("tab").unwrap(),
            false,
        )
        .unwrap();
        loop {
            if rx.recv().await.unwrap() == "cd workspace/" {
                break;
            }
        }
        input.read(&app, |input, ctx| {
            assert_eq!(input.buffer_text(ctx), "cd workspace/")
        });
    });
}

#[test]
fn ambiguous_directory_completions_cycle_and_preserve_text_after_cursor() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let directory = tempfile::tempdir().unwrap();
        for name in ["work-a", "work-b"] {
            std::fs::create_dir(directory.path().join(name)).unwrap();
        }
        let session = Arc::new(Session::new(
            SessionInfo::new_for_test(),
            Arc::new(TestCommandExecutor::default()),
        ));
        let context = SessionContext::for_terminal(
            session,
            TypedPathBuf::from(directory.path().to_str().unwrap()),
        );
        let buffer = "cd work && pwd";
        let cursor = "cd work".len();
        let result = suggestions(buffer, cursor, None, CompleterOptions::default(), &context).await;
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        let input = terminal.read(&app, |view, _| view.input().clone());
        input.update(&mut app, |input, ctx| {
            input.replace_buffer_content(buffer, ctx);
            input.editor.update(ctx, |editor, ctx| {
                editor.select_ranges_by_byte_offset([cursor.into()..cursor.into()], ctx)
            });
            input.finish_completion(input.completion_request, buffer.into(), cursor, result, ctx);
            assert_eq!(input.buffer_text(ctx), "cd work- && pwd");
            assert!(input.completions.is_some());
        });
        input.update(&mut app, |input, ctx| {
            input.tab(ctx);
            assert_eq!(input.buffer_text(ctx), "cd work-a/ && pwd");
        });
        input.update(&mut app, |input, ctx| {
            input.tab(ctx);
            assert_eq!(input.buffer_text(ctx), "cd work-b/ && pwd");
        });
    });
}

#[test]
fn late_completion_does_not_replace_new_input() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir(directory.path().join("workspace")).unwrap();
        let session = Arc::new(Session::new(
            SessionInfo::new_for_test(),
            Arc::new(TestCommandExecutor::default()),
        ));
        let context = SessionContext::for_terminal(
            session,
            TypedPathBuf::from(directory.path().to_str().unwrap()),
        );
        let result = suggestions("cd work", 7, None, CompleterOptions::default(), &context).await;
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        let input = terminal.read(&app, |view, _| view.input().clone());
        input.update(&mut app, |input, ctx| {
            input.replace_buffer_content("echo keep", ctx);
            input.finish_completion(input.completion_request, "cd work".into(), 7, result, ctx);
            assert_eq!(input.buffer_text(ctx), "echo keep");
        });
    });
}
