use settings::macros::define_settings_group;
use settings::{RespectUserSyncSetting, Setting, SupportedPlatforms, SyncToCloud};

use crate::terminal::model::terminal_model::BlockSortDirection;

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename_all = "snake_case")]
pub enum InputMode {
    #[default]
    PinnedToBottom,
    PinnedToTop,
    Waterfall,
}

impl InputMode {
    pub fn is_inverted_blocklist(&self) -> bool {
        matches!(self, Self::PinnedToTop)
    }

    pub fn block_sort_direction(&self) -> BlockSortDirection {
        if self.is_inverted_blocklist() {
            BlockSortDirection::MostRecentFirst
        } else {
            BlockSortDirection::MostRecentLast
        }
    }
}

define_settings_group!(InputModeSettings, settings: [
    input_mode: InputModeState {
        type: InputMode,
        // Note that for new users, we now override this default value in SettingsInitializer
        // to set it to InputMode::Waterfall.
        default: InputMode::PinnedToBottom,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        surface: settings::SettingSurfaces::GUI,
        private: false,
        storage_key: "InputMode",
        toml_path: "appearance.input.input_mode",
        description: "The position of the terminal input.",
    },
]);

impl InputModeSettings {
    pub fn is_pinned_to_top(&self) -> bool {
        *self.input_mode.value() == InputMode::PinnedToTop
    }
}
