use warp::tui_export::Appearance;
use warpui_core::App;

#[test]
fn local_terminal_styles_follow_the_active_theme() {
    App::test((), |app| async move {
        app.add_singleton_model(|_| Appearance::mock());
        app.read(|ctx| {
            let builder = super::TuiUiBuilder::from_app(ctx);
            assert_ne!(builder.primary_text_style(), builder.selection_style());
            assert_ne!(
                builder.shell_command_prefix_style(),
                builder.muted_text_style()
            );
        });
    });
}
