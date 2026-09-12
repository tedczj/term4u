use warpui::platform::WindowStyle;
use warpui::{App, TypedActionView};

use super::*;
use crate::editor::EditorAction;
use crate::test_util::terminal::initialize_app_for_terminal_view;

#[test]
fn enter_in_notebook_body_inserts_and_persists_newline() {
    let directory = tempfile::tempdir().unwrap();
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        app.add_singleton_model(|_| {
            NotebookStore::load(
                directory.path().to_owned(),
                vec![(7, Some("Title".to_owned()), Some("line one".to_owned()))],
            )
        });
        let id = NotebookId::from_legacy_id(7);
        let (_, view) = app.add_window(WindowStyle::NotStealFocus, NotebookView::new);
        view.update(&mut app, |view, ctx| {
            assert!(view.load(id.clone(), ctx));
            view.body.update(ctx, |body, ctx| {
                body.handle_action(&EditorAction::MoveToBufferEnd, ctx);
                body.handle_action(&EditorAction::Enter, ctx);
            });
        });
        view.read(&app, |view, ctx| {
            assert_eq!(view.body.as_ref(ctx).buffer_text(ctx), "line one\n");
            assert_eq!(
                NotebookStore::as_ref(ctx).get(&id).unwrap().data,
                "line one\n"
            );
        });
    });
}
