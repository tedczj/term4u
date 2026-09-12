#[cfg(test)]
use warpui::App;

#[cfg(test)]
pub fn initialize_settings_for_tests(app: &mut App) {
    use warp_core::execution_mode::ExecutionMode;
    initialize_settings_for_tests_with_mode(app, ExecutionMode::App, false);
}

#[cfg(test)]
pub fn initialize_history_persistence_for_tests(app: &mut App) {
    use crate::{GlobalResourceHandles, GlobalResourceHandlesProvider};

    initialize_settings_for_tests(app);

    let global_resource_handles = GlobalResourceHandles::mock(app);
    app.add_singleton_model(|_| GlobalResourceHandlesProvider::new(global_resource_handles));
}

#[cfg(test)]
pub fn initialize_settings_for_tests_with_mode(
    app: &mut App,
    mode: warp_core::execution_mode::ExecutionMode,
    is_sandboxed: bool,
) {
    use warp_core::execution_mode::AppExecutionMode;

    use crate::settings::manager::SettingsManager;
    use crate::settings::{init_and_register_user_preferences, register_all_settings};
    use crate::user_config::WarpConfig;

    app.add_singleton_model(|ctx| AppExecutionMode::new(mode, is_sandboxed, ctx));
    app.update(init_and_register_user_preferences);
    app.add_singleton_model(|_| SettingsManager::default());
    app.add_singleton_model(WarpConfig::mock);
    app.update(|ctx| warpui_extras::secure_storage::register_noop("test", ctx));
    app.update(register_all_settings);
}
