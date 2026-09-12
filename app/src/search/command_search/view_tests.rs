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
