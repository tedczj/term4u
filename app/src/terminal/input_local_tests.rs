use warpui::App;

use super::*;
use crate::test_util::terminal::{
    add_window_with_id_and_terminal, initialize_app_for_terminal_view,
};

fn new_input(app: &mut App) -> ViewHandle<Input> {
    initialize_app_for_terminal_view(app);
    let (_, terminal) = add_window_with_id_and_terminal(app, None);
    terminal.read(app, |view, _| view.input().clone())
}

#[test]
fn inserting_at_a_cursor_preserves_the_suffix_and_undo() {
    App::test((), |mut app| async move {
        let input = new_input(&mut app);
        input.update(&mut app, |input, ctx| {
            input.replace_buffer_content("echo 世界", ctx);
            input.editor.update(ctx, |editor, ctx| {
                editor.select_ranges_by_byte_offset([5_usize.into()..5_usize.into()], ctx);
            });
            input.insert_text("你好 ", ctx);
            assert_eq!(input.buffer_text(ctx), "echo 你好 世界");
            input.editor.update(ctx, |editor, ctx| {
                editor.handle_action(&EditorAction::Undo, ctx);
            });
            assert_eq!(input.buffer_text(ctx), "echo 世界");
        });
    });
}

#[test]
fn insertion_replaces_the_selected_range_without_submitting() {
    App::test((), |mut app| async move {
        let input = new_input(&mut app);
        let (tx, rx) = async_channel::unbounded();
        app.update(|ctx| {
            ctx.subscribe_to_view(&input, move |_, event, _| {
                if let Event::ExecuteCommand(command) = event {
                    tx.try_send(command.clone()).unwrap();
                }
            });
        });
        input.update(&mut app, |input, ctx| {
            input.replace_buffer_content("echo old && pwd", ctx);
            input.editor.update(ctx, |editor, ctx| {
                editor.select_ranges_by_byte_offset([5_usize.into()..8_usize.into()], ctx);
            });
            input.insert_text("new\nline", ctx);
            assert_eq!(input.buffer_text(ctx), "echo new\nline && pwd");
        });
        assert!(rx.try_recv().is_err());
    });
}

#[test]
fn history_navigation_restores_the_draft_and_original_cursor() {
    App::test((), |mut app| async move {
        let input = new_input(&mut app);
        input.update(&mut app, |input, ctx| {
            input.replace_buffer_content("echo", ctx);
            input.editor.update(ctx, |editor, ctx| {
                editor.select_ranges_by_byte_offset([2_usize.into()..2_usize.into()], ctx);
            });
            input.navigate_history(
                SessionId::from(1_u64),
                vec!["pwd".into(), "echo first".into(), "echo last".into()],
                true,
                ctx,
            );
            assert_eq!(input.buffer_text(ctx), "echo last");
            input.navigate_history(SessionId::from(1_u64), vec![], true, ctx);
            assert_eq!(input.buffer_text(ctx), "echo first");
            input.navigate_history(SessionId::from(1_u64), vec![], true, ctx);
            assert_eq!(input.buffer_text(ctx), "echo first");
            input.navigate_history(SessionId::from(1_u64), vec![], false, ctx);
            assert_eq!(input.buffer_text(ctx), "echo last");
            input.navigate_history(SessionId::from(1_u64), vec![], false, ctx);
            assert_eq!(input.buffer_text(ctx), "echo");
            assert_eq!(input.cursor(ctx), 2);
        });
    });
}

#[test]
fn changing_sessions_does_not_reuse_another_sessions_history() {
    App::test((), |mut app| async move {
        let input = new_input(&mut app);
        input.update(&mut app, |input, ctx| {
            input.navigate_history(SessionId::from(1_u64), vec!["first".into()], true, ctx);
            assert_eq!(input.buffer_text(ctx), "first");
            input.navigate_history(
                SessionId::from(2_u64),
                vec!["first in second session".into()],
                true,
                ctx,
            );
            assert_eq!(input.buffer_text(ctx), "first in second session");
            input.navigate_history(SessionId::from(2_u64), vec![], false, ctx);
            assert_eq!(input.buffer_text(ctx), "first");
        });
    });
}

#[test]
fn editing_a_history_result_starts_a_new_traversal() {
    App::test((), |mut app| async move {
        let input = new_input(&mut app);
        input.update(&mut app, |input, ctx| {
            input.navigate_history(SessionId::from(1_u64), vec!["echo".into()], true, ctx);
            input.insert_text(" local", ctx);
            input.navigate_history(SessionId::from(1_u64), vec![], false, ctx);
            assert_eq!(input.buffer_text(ctx), "echo local");
            input.navigate_history(SessionId::from(1_u64), vec!["pwd".into()], true, ctx);
            assert_eq!(input.buffer_text(ctx), "echo local");
        });
    });
}

#[test]
fn history_does_not_replace_a_selected_range() {
    App::test((), |mut app| async move {
        let input = new_input(&mut app);
        input.update(&mut app, |input, ctx| {
            input.replace_buffer_content("echo original", ctx);
            input.editor.update(ctx, |editor, ctx| {
                editor.select_ranges_by_byte_offset([5_usize.into()..13_usize.into()], ctx);
            });
            input.navigate_history(
                SessionId::from(1_u64),
                vec!["echo original from history".into()],
                true,
                ctx,
            );
            assert_eq!(input.buffer_text(ctx), "echo original");
            assert_eq!(input.editor.as_ref(ctx).selected_text(ctx), "original");
        });
    });
}
