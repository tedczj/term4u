use onboarding::{SelectedSettings, UICustomizationSettings};
use warp_core::features::FeatureFlag;

use super::{refresh_pending_onboarding_choices, requires_post_onboarding_login};

#[test]
fn account_first_requires_login_even_without_ai_or_drive_settings() {
    let _account_first = FeatureFlag::AccountFirstOnboarding.override_enabled(true);

    assert!(requires_post_onboarding_login(false, false, false));
    assert!(!requires_post_onboarding_login(true, false, false));
}

#[test]
fn fallback_flow_only_requires_login_for_account_backed_settings() {
    let _account_first = FeatureFlag::AccountFirstOnboarding.override_enabled(false);

    assert!(!requires_post_onboarding_login(false, false, false));
    assert!(requires_post_onboarding_login(false, true, false));
    assert!(requires_post_onboarding_login(false, false, true));
}
#[test]
fn refreshing_pending_onboarding_choices_replaces_stale_settings() {
    let settings = |use_vertical_tabs| SelectedSettings::Terminal {
        ui_customization: Some(UICustomizationSettings {
            use_vertical_tabs,
            show_conversation_history: false,
            show_project_explorer: true,
            show_global_search: false,
            show_warp_drive: false,
            show_code_review_button: true,
        }),
        cli_agent_toolbar_enabled: true,
        show_agent_notifications: false,
    };

    let mut pending_settings = Some(settings(false));
    let mut pending_tutorial = None;
    let latest_settings = settings(true);

    refresh_pending_onboarding_choices(
        &latest_settings,
        &mut pending_settings,
        &mut pending_tutorial,
    );

    let Some(SelectedSettings::Terminal {
        ui_customization: Some(ui),
        ..
    }) = pending_settings
    else {
        panic!("latest terminal settings should replace the pending snapshot");
    };
    assert!(ui.use_vertical_tabs);
    assert!(pending_tutorial.is_some());
}
