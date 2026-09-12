use std::fmt::Display;

use regex::Regex;
use serde::{Deserialize, Serialize};
use settings::macros::{maybe_define_setting, register_settings_events};
use settings::{
    ChangeEventReason, RespectUserSyncSetting, Setting, SupportedPlatforms, SyncToCloud,
};
use warp_errors::report_error;
pub use warp_terminal::model::secrets::RegexDisplayInfo;
use warpui::{AppContext, Entity, ModelContext, SingletonEntity, UpdateModel};

use super::local_privacy_policy::LocalPrivacyPolicy;

pub const TELEMETRY_ENABLED_DEFAULTS_KEY: &str = "TelemetryEnabled";
pub const CRASH_REPORTING_ENABLED_DEFAULTS_KEY: &str = "CrashReportingEnabled";
pub const CLOUD_CONVERSATION_STORAGE_ENABLED_DEFAULTS_KEY: &str = "CloudConversationStorageEnabled";

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(description = "A custom regex pattern for detecting and redacting secrets.")]
pub struct CustomSecretRegex {
    #[serde(with = "serde_regex")]
    #[schemars(with = "String", description = "The regex pattern to match secrets.")]
    pub pattern: Regex,
    #[serde(default)]
    #[schemars(description = "Optional display name for this secret pattern.")]
    pub name: Option<String>,
}

impl CustomSecretRegex {
    pub fn pattern(&self) -> &Regex {
        &self.pattern
    }
}

impl RegexDisplayInfo for CustomSecretRegex {
    fn pattern(&self) -> &str {
        self.pattern.as_str()
    }

    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

impl Display for CustomSecretRegex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.pattern.as_str())
    }
}

impl PartialEq for CustomSecretRegex {
    /// We do not factor in the name to equality checks --
    /// if the regex is the same, then the regex is the same.
    /// This allows us to avoid adding duplicate regexes.
    fn eq(&self, other: &Self) -> bool {
        self.pattern.as_str() == other.pattern.as_str()
    }
}

impl settings_value::SettingsValue for CustomSecretRegex {}

maybe_define_setting!(CustomSecretRegexList, group: PrivacySettings, {
    type: Vec<CustomSecretRegex>,
    default: Vec::new(),
    supported_platforms: SupportedPlatforms::ALL,
    sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::No),
    surface: settings::SettingSurfaces::GUI,
    private: false,
    toml_path: "privacy.custom_secret_regex_list",
    description: "Custom regex patterns for detecting and redacting secrets.",
});

maybe_define_setting!(HasInitializedDefaultSecretRegexes, group: PrivacySettings, {
    type: bool,
    default: false,
    supported_platforms: SupportedPlatforms::ALL,
    sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::No),
    surface: settings::SettingSurfaces::GUI,
    private: true,
});

/// Singleton model for managing the user's privacy settings (whether the user has enabled crash
/// reporting and/or telemetry).
pub struct PrivacySettings {
    pub is_telemetry_enabled: bool,
    pub is_crash_reporting_enabled: bool,
    pub is_cloud_conversation_storage_enabled: bool,
    pub has_initialized_default_secret_regexes: HasInitializedDefaultSecretRegexes,
    pub user_secret_regex_list: CustomSecretRegexList,
}

/// A snapshot of a user's [`PrivacySettings`] settings at some point in time.
#[derive(Clone, Copy)]
pub struct PrivacySettingsSnapshot;

impl PrivacySettingsSnapshot {
    pub fn cloud_conversation_storage_enabled(&self) -> Option<bool> {
        Some(LocalPrivacyPolicy::CLOUD_STORAGE_ENABLED)
    }

    pub fn is_telemetry_enabled(&self) -> bool {
        LocalPrivacyPolicy::TELEMETRY_ENABLED
    }

    pub fn is_crash_reporting_enabled(&self) -> bool {
        LocalPrivacyPolicy::CRASH_REPORTING_ENABLED
    }

    pub fn is_telemetry_force_enabled(&self) -> bool {
        false
    }

    pub fn should_disable_telemetry(&self) -> bool {
        true
    }

    pub fn should_collect_ai_ugc_telemetry(&self) -> bool {
        false
    }

    #[cfg(test)]
    pub fn mock() -> Self {
        Self
    }
}

impl PrivacySettings {
    /// Registers a singleton PrivacySettings model on `app`.
    ///
    /// We expose this function publicly (while keeping the constructor private) to prevent
    /// instantiation another PrivacySettings struct, in the case where a developer might be
    /// unaware that it is registered as a singleton model.
    pub fn register_singleton(ctx: &mut AppContext) {
        let handle = ctx.add_singleton_model(PrivacySettings::new);

        register_settings_events!(
            PrivacySettings,
            user_secret_regex_list,
            CustomSecretRegexList,
            handle,
            ctx
        );
    }

    /// Returns a new PrivacySettings object initialized from locally cached values. Server-side
    /// settings are fetched later via `fetch_or_update_settings`, which is called from
    /// `on_user_fetched` after the user's auth state is established.
    fn new(ctx: &mut ModelContext<Self>) -> Self {
        let user_secret_regex_list: CustomSecretRegexList =
            CustomSecretRegexList::new_from_storage(ctx);
        let has_initialized_default_secret_regexes: HasInitializedDefaultSecretRegexes =
            HasInitializedDefaultSecretRegexes::new_from_storage(ctx);

        Self {
            is_crash_reporting_enabled: LocalPrivacyPolicy::CRASH_REPORTING_ENABLED,
            is_telemetry_enabled: LocalPrivacyPolicy::TELEMETRY_ENABLED,
            is_cloud_conversation_storage_enabled: LocalPrivacyPolicy::CLOUD_STORAGE_ENABLED,
            user_secret_regex_list,
            has_initialized_default_secret_regexes,
        }
    }

    pub fn is_telemetry_force_enabled(&self) -> bool {
        false
    }

    pub fn refresh_to_default(&mut self) {
        // TODO(zach): this seems incorrect - should we also update the values on disk?
        self.is_telemetry_enabled = LocalPrivacyPolicy::TELEMETRY_ENABLED;
        self.is_crash_reporting_enabled = LocalPrivacyPolicy::CRASH_REPORTING_ENABLED;
        self.is_cloud_conversation_storage_enabled = LocalPrivacyPolicy::CLOUD_STORAGE_ENABLED;
    }

    pub fn fetch_or_update_settings(&self, ctx: &mut ModelContext<Self>) {
        let _ = ctx;
    }

    /// Constructor for tests only.
    #[cfg(any(test, feature = "test-util"))]
    pub fn mock(_ctx: &mut ModelContext<Self>) -> Self {
        Self {
            is_crash_reporting_enabled: LocalPrivacyPolicy::CRASH_REPORTING_ENABLED,
            is_telemetry_enabled: LocalPrivacyPolicy::TELEMETRY_ENABLED,
            is_cloud_conversation_storage_enabled: LocalPrivacyPolicy::CLOUD_STORAGE_ENABLED,
            user_secret_regex_list: CustomSecretRegexList::new(None),
            has_initialized_default_secret_regexes: HasInitializedDefaultSecretRegexes::new(None),
        }
    }

    /// Returns a snapshot of the user's privacy settings.
    ///
    /// The returned snapshot is not stateful, thus its values should be used shortly after the
    /// snapshot is returned.
    pub fn get_snapshot(&self, app: &AppContext) -> PrivacySettingsSnapshot {
        let _ = app;
        PrivacySettingsSnapshot
    }

    pub fn set_is_crash_reporting_enabled(
        &mut self,
        new_value: bool,
        ctx: &mut ModelContext<PrivacySettings>,
    ) {
        let _ = (new_value, ctx);
        self.is_crash_reporting_enabled = LocalPrivacyPolicy::CRASH_REPORTING_ENABLED;
    }

    pub fn set_is_telemetry_enabled(
        &mut self,
        new_value: bool,
        ctx: &mut ModelContext<PrivacySettings>,
    ) {
        let _ = (new_value, ctx);
        self.is_telemetry_enabled = LocalPrivacyPolicy::TELEMETRY_ENABLED;
    }

    pub fn set_is_cloud_conversation_storage_enabled(
        &mut self,
        new_value: bool,
        ctx: &mut ModelContext<PrivacySettings>,
    ) {
        let _ = (new_value, ctx);
        self.is_cloud_conversation_storage_enabled = LocalPrivacyPolicy::CLOUD_STORAGE_ENABLED;
    }

    pub fn remove_user_secret_regex(&mut self, idx: &usize, ctx: &mut ModelContext<Self>) {
        let mut new_user_secret_regex_list = self.user_secret_regex_list.to_vec();
        new_user_secret_regex_list.remove(*idx);
        if self
            .user_secret_regex_list
            .set_value(new_user_secret_regex_list, ctx)
            .is_err()
        {
            report_error!("Custom Secret Regex List failed to serialize")
        }
    }

    /// Initializes the custom secret regex list with the default regexes, provided
    /// non matches can be found.
    /// This can be called when a user first enables secret redaction.
    pub fn add_all_recommended_regex(&mut self, ctx: &mut ModelContext<Self>) {
        let mut new_user_secret_regex_list = self.user_secret_regex_list.to_vec();
        let num_existing_regexes = new_user_secret_regex_list.len();

        // Add all the default regexes if they don't already exist
        for default_regex in crate::terminal::model::secrets::regexes::DEFAULT_REGEXES_WITH_NAMES {
            match Regex::new(default_regex.pattern) {
                Ok(regex) => {
                    let custom_regex = CustomSecretRegex {
                        pattern: regex,
                        name: Some(default_regex.name.to_string()),
                    };
                    if !new_user_secret_regex_list.contains(&custom_regex) {
                        new_user_secret_regex_list.push(custom_regex);
                    }
                }
                _ => {
                    report_error!(
                        "Failed to compile default regex",
                        extra: { "pattern" => %default_regex.pattern }
                    );
                }
            }
        }

        if num_existing_regexes == new_user_secret_regex_list.len() {
            return;
        }

        if self
            .user_secret_regex_list
            .set_value(new_user_secret_regex_list, ctx)
            .is_err()
        {
            report_error!("Failed to serialize default regexes to custom secret regex list")
        }

        ctx.notify();
    }

    /// Disables the default regex trigger, so that it will not be executed.
    pub fn disable_default_regex_trigger(&mut self, ctx: &mut ModelContext<Self>) {
        if self
            .has_initialized_default_secret_regexes
            .set_value(true, ctx)
            .is_err()
        {
            report_error!("Failed to disable default regex trigger");
        }
    }

    /// Initializes the custom secret regex list with the default regexes.
    /// This will only be executed once per user, and only if they haven't already initialized.
    pub fn initialize_default_regexes_once(&mut self, ctx: &mut ModelContext<Self>) {
        // Only initialize if we haven't done so before
        if !*self.has_initialized_default_secret_regexes.value() {
            self.add_all_recommended_regex(ctx);

            // Mark as initialized
            if self
                .has_initialized_default_secret_regexes
                .set_value(true, ctx)
                .is_err()
            {
                report_error!("Failed to set has_initialized_default_secret_regexes flag");
            }
        }
    }

    pub fn maybe_sync_with_warp_drive_prefs(&mut self, ctx: &mut ModelContext<Self>) {
        self.initialize_default_regexes_once(ctx);
    }
}

/// Events emitted when PrivacySettings is updated.
#[derive(Clone, Copy)]
pub enum PrivacySettingsChangedEvent {
    UpdateIsTelemetryEnabled {
        old_value: bool,
        new_value: bool,
    },
    UpdateIsCrashReportingEnabled {
        old_value: bool,
        new_value: bool,
    },
    UpdateIsCloudConversationStorageEnabled {
        old_value: bool,
        new_value: bool,
    },
    CustomSecretRegexList {
        change_event_reason: ChangeEventReason,
    },
    HasInitializedDefaultSecretRegexes {
        change_event_reason: ChangeEventReason,
    },
}

impl Entity for PrivacySettings {
    type Event = PrivacySettingsChangedEvent;
}

impl SingletonEntity for PrivacySettings {}

#[cfg(test)]
#[path = "privacy_tests.rs"]
mod tests;
