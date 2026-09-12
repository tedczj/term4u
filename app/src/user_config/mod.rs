pub mod util;

#[cfg_attr(not(target_family = "wasm"), path = "native.rs")]
#[cfg_attr(target_family = "wasm", path = "wasm.rs")]
mod imp;

#[cfg(feature = "local_fs")]
use std::path::Path;
use std::path::PathBuf;

#[cfg(feature = "local_fs")]
pub use imp::load_workflows;
pub use imp::{load_launch_configs, load_theme_configs};
use lazy_static::lazy_static;
use warp_core::ui::theme::WarpTheme;
use warpui::{Entity, ModelContext, SingletonEntity};

use crate::launch_configs::launch_config::LaunchConfig;
use crate::tab_configs::{TabConfig, TabConfigError};
use crate::themes::theme::{ThemeKind, WarpThemeConfig};
use crate::workflows::workflow::Workflow;

lazy_static! {
    pub static ref LAUNCH_CONFIG_COMMENT: String = format!(
        "# Warp Launch Configuration
#
#
# Use this to start a certain configuration of windows, tabs, and panes.
# Open the launch configuration palette to access and open any launch configuration.
#
# This file defines your launch configuration.
# More on how to do so here:
# https://docs.warp.dev/terminal/sessions/launch-configurations
#
# All launch configurations are stored under {}.
# Edit them anytime!
#
# You can also add commands that run on-start for your launch configurations like so:
# ---
# name: Example with Command
# windows:
#  - tabs:
#      - layout:
#          cwd: /Users/warp-user/project
#          commands:
#            - exec: code .
",
        warp_core::paths::home_relative_path(&crate::user_config::launch_configs_dir())
    );
}

#[derive(Clone)]
pub enum WarpConfigUpdateEvent {
    Themes,
    #[cfg_attr(not(feature = "local_fs"), expect(dead_code))]
    LocalUserWorkflows,
    LaunchConfigs,
    #[cfg_attr(not(feature = "local_fs"), allow(dead_code))]
    TabConfigs,
    /// Emitted when one or more tab config files failed to parse.
    #[cfg_attr(target_family = "wasm", allow(dead_code))]
    TabConfigErrors(Vec<TabConfigError>),
    /// The settings file (`settings.toml`) was created, modified, or deleted.
    #[cfg_attr(not(feature = "local_fs"), expect(dead_code))]
    Settings,
    /// One or more settings in `settings.toml` could not be loaded.
    #[cfg_attr(not(feature = "local_fs"), expect(dead_code))]
    SettingsErrors(crate::settings::SettingsFileError),
    /// A previously-errored settings reload succeeded with no errors.
    #[cfg_attr(not(feature = "local_fs"), expect(dead_code))]
    SettingsErrorsCleared,
}

/// Singleton model containing user configurable file entities like themes, launch configs, and
/// workflows.
///
/// Emits events when entities are changed, which are detected via filesystem
/// watchers on the user's `data_dir()` (themes, workflows, launch configs,
/// tab configs, etc.) and, on platforms where it differs, `config_local_dir()`
/// (`settings.toml`, `keybindings.yaml`, `user_preferences.json`).
#[derive(Default)]
pub struct WarpConfig {
    launch_configs: Vec<LaunchConfig>,
    tab_configs: Vec<TabConfig>,
    #[cfg_attr(target_family = "wasm", allow(dead_code))]
    tab_config_errors: Vec<TabConfigError>,
    theme_config: WarpThemeConfig,
    local_user_workflows: Vec<Workflow>,
}

/// Platform-independent parts of WarpConfig.
///
/// Additional platform-dependent functionality can be found in impl blocks
/// in native.rs and wasm.rs.
impl WarpConfig {
    #[cfg(test)]
    pub fn mock(_ctx: &mut ModelContext<Self>) -> Self {
        Self {
            theme_config: WarpThemeConfig::new(),
            ..Default::default()
        }
    }

    pub fn launch_configs(&self) -> &Vec<LaunchConfig> {
        &self.launch_configs
    }

    pub fn theme_config(&self) -> &WarpThemeConfig {
        &self.theme_config
    }

    pub fn local_user_workflows(&self) -> &Vec<Workflow> {
        &self.local_user_workflows
    }

    pub fn update_theme_config(
        &mut self,
        theme_config: WarpThemeConfig,
        ctx: &mut ModelContext<Self>,
    ) {
        self.theme_config = theme_config;
        ctx.emit(WarpConfigUpdateEvent::Themes);
    }

    pub fn add_new_theme_to_config(
        &mut self,
        theme_name: ThemeKind,
        theme: WarpTheme,
        ctx: &mut ModelContext<Self>,
    ) {
        self.theme_config.add_new_theme(theme_name, theme);
        ctx.emit(WarpConfigUpdateEvent::Themes);
    }
}

/// Returns the base directory in which all of the user's data is stored.
fn base_dir() -> PathBuf {
    warp_core::paths::data_dir()
}

/// Returns the path to the directory containing the user's custom themes.
pub fn themes_dir() -> PathBuf {
    warp_core::paths::themes_dir()
}

/// Returns the path to the directory containing the user's custom workflows.
#[cfg_attr(target_family = "wasm", expect(dead_code))]
pub fn workflows_dir() -> PathBuf {
    crate::workflows::local_workflows::workflows_dir(base_dir())
}

/// Returns the path to the directory containing the user's launch
/// configurations.
pub fn launch_configs_dir() -> PathBuf {
    base_dir().join("launch_configurations")
}

/// Returns the path to the directory containing the user's tab configs.
pub fn tab_configs_dir() -> PathBuf {
    base_dir().join("tab_configs")
}

/// Returns the path to the directory containing the built-in default tab configs.
/// These are shipped with Warp and user-editable (Warp does not overwrite modifications).
#[cfg_attr(target_family = "wasm", expect(dead_code))]
pub fn default_tab_configs_dir() -> PathBuf {
    base_dir().join("default_tab_configs")
}

/// Returns whether the path points to a tab config TOML file under one of Warp's
/// tab config directories.
#[cfg(feature = "local_fs")]
pub fn is_tab_config_toml(path: &Path) -> bool {
    let is_toml = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension == "toml");
    if !is_toml {
        return false;
    }

    [tab_configs_dir(), default_tab_configs_dir()]
        .into_iter()
        .any(|dir| path.starts_with(dir))
}

/// Returns a `.toml` path in `dir` that does not yet exist.
///
/// Tries `{base_name}.toml`, then `{base_name}_1.toml`, `{base_name}_2.toml`, etc.
#[cfg(feature = "local_fs")]
pub(crate) fn find_unused_toml_path(dir: &Path, base_name: &str) -> PathBuf {
    let base = dir.join(format!("{base_name}.toml"));
    if !base.exists() {
        return base;
    }
    let mut n = 1u32;
    loop {
        let candidate = dir.join(format!("{base_name}_{n}.toml"));
        if !candidate.exists() {
            return candidate;
        }
        n = n.saturating_add(1);
    }
}

impl Entity for WarpConfig {
    type Event = WarpConfigUpdateEvent;
}

impl SingletonEntity for WarpConfig {}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
