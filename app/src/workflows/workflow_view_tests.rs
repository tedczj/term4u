use warpui::App;
use warpui::platform::WindowStyle;

use super::*;

#[test]
fn loading_workflow_preserves_parameters_and_metadata() {
    App::test((), |mut app| async move {
        crate::test_util::settings::initialize_settings_for_tests(&mut app);
        app.add_singleton_model(|_| crate::appearance::Appearance::mock());
        app.add_singleton_model(|_| {
            crate::settings_view::keybindings::KeybindingChangedNotifier::new()
        });
        let (_, view) = app.add_window(WindowStyle::NotStealFocus, WorkflowView::new_in_pane);
        let workflow: Workflow = serde_json::from_value(serde_json::json!({
            "name": "Local workflow", "command": "echo {{message}}",
            "arguments": [{"name": "message", "default_value": "hello"}],
            "description": "Keep this description", "tags": ["local"], "author": "Fixture author"
        }))
        .unwrap();
        view.update(&mut app, |view, ctx| {
            view.load(workflow.clone(), WorkflowViewMode::Edit, ctx)
        });
        view.read(&app, |view, ctx| assert_eq!(view.workflow(ctx), workflow));
    });
}
