use warpui_core::App;

#[test]
fn local_tui_bindings_register_without_cross_surface_conflicts() {
    App::test((), |mut app| async move {
        app.update(super::init);
    });
}

#[test]
fn editable_bindings_only_expose_local_tab_operations() {
    App::test((), |mut app| async move {
        app.update(super::init);
        app.read(|ctx| {
            let mut names = ctx
                .editable_bindings()
                .map(|binding| binding.name)
                .collect::<Vec<_>>();
            names.sort_unstable();
            assert_eq!(
                names,
                vec![
                    "tui:close_tab",
                    "tui:new_tab",
                    "tui:next_tab",
                    "tui:previous_tab",
                ]
            );
        });
    });
}
