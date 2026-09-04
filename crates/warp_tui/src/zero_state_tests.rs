use warp::tui_export::Appearance;
use warpui_core::App;
use warpui_core::elements::tui::{TuiBufferExt as _, TuiRect};
use warpui_core::presenter::tui::TuiPresenter;

#[test]
fn fixed_width_zero_state_only_describes_local_terminal_operations() {
    App::test((), |app| async move {
        app.add_singleton_model(|_| Appearance::mock());
        let lines = app.read(|ctx| {
            TuiPresenter::new()
                .present_element(super::local_zero_state(ctx), TuiRect::new(0, 0, 60, 5), ctx)
                .buffer
                .to_lines()
        });
        let rendered = lines.join("\n");
        assert!(rendered.contains("Term4u local terminal"));
        assert!(rendered.contains("Type a command to begin."));
        assert!(rendered.contains("ctrl-t new tab"));
        assert!(!rendered.to_ascii_lowercase().contains("agent"));
        assert!(!rendered.to_ascii_lowercase().contains("cloud"));
        assert!(!rendered.to_ascii_lowercase().contains("sign in"));
    });
}
