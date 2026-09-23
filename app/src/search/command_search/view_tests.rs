use warpui::App;
use warpui::platform::WindowStyle;

use super::*;
fn initialize_app(app: &mut App) {
    crate::test_util::terminal::initialize_app_for_terminal_view(app);
}

#[test]
fn test_render_view() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);

        let (_window_id, _view) = app.add_window(WindowStyle::NotStealFocus, |ctx| {
            CommandSearchView::new(ctx)
        });

        app.update(|_| {
            // This will force a redraw of the window, which lays out the
            // window, including the command search view.
        });
    });
}

#[test]
fn l0_01_history_loading_refreshes_the_current_search_query() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let session = Arc::new(crate::terminal::model::session::Session::test());
        let (loaded_tx, loaded_rx) = async_channel::bounded(1);
        app.update(|ctx| {
            History::handle(ctx).update(ctx, |history, ctx| {
                history.init_session_with(
                    session.clone(),
                    async move { loaded_rx.recv().await.unwrap() },
                    ctx,
                );
            });
        });
        let (_, view) = app.add_window(WindowStyle::NotStealFocus, |ctx| {
            let mut view = CommandSearchView::new(ctx);
            view.reset_state(
                Some(session),
                None,
                "initial".into(),
                Some(QueryFilter::History),
                MenuPositioning::AboveInputBox,
                ctx,
            );
            view
        });
        let (tx, rx) = async_channel::unbounded();
        view.update(&mut app, |view, ctx| {
            view.search_bar
                .update(ctx, |bar, ctx| bar.set_query("needle".into(), ctx));
            assert!(view.mixer.as_ref(ctx).results().is_empty());
            ctx.subscribe_to_model(&view.mixer, move |_, mixer, _, ctx| {
                if mixer.as_ref(ctx).results().iter().any(|item| {
                    matches!(item.accept_result(),
                    CommandSearchItemAction::AcceptHistory(item) if item.command == "echo needle")
                }) {
                    let _ = tx.try_send(());
                }
            });
        });
        loaded_tx
            .send(vec!["echo needle".to_owned(), "other".to_owned()])
            .await
            .unwrap();
        let result = futures::future::select(
            Box::pin(rx.recv()),
            Box::pin(warpui::r#async::Timer::after(Duration::from_secs(5))),
        )
        .await;
        assert!(
            matches!(result, futures::future::Either::Left((Ok(()), _))),
            "loading history must refresh an already-open search"
        );
        view.read(&app, |view, ctx| {
            assert_eq!(view.search_bar.as_ref(ctx).query(ctx), "needle");
            assert_eq!(
                view.mixer.as_ref(ctx).current_query().unwrap().text,
                "needle"
            );
        });
    });
}
