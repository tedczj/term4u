use warp_core::execution_mode::ExecutionMode;
use warpui::{App, SingletonEntity};

use super::PrivacySettings;
use crate::test_util::settings::initialize_settings_for_tests_with_mode;

#[test]
fn privacy_settings_stay_disabled_in_gui_and_tui() {
    for mode in [ExecutionMode::App, ExecutionMode::Tui] {
        App::test((), move |mut app| async move {
            initialize_settings_for_tests_with_mode(&mut app, mode, false);
            app.update(PrivacySettings::register_singleton);
            PrivacySettings::handle(&app).update(&mut app, |settings, ctx| {
                settings.set_is_telemetry_enabled(true, ctx);
                settings.set_is_crash_reporting_enabled(true, ctx);
                settings.set_is_cloud_conversation_storage_enabled(true, ctx);
                assert!(!settings.is_telemetry_enabled);
                assert!(!settings.is_crash_reporting_enabled);
                assert!(!settings.is_cloud_conversation_storage_enabled);
            });
        });
    }
}
