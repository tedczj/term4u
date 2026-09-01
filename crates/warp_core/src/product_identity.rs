use std::borrow::Cow;

use crate::AppId;

pub const GUI_NAME: &str = "Term4u";
pub const TUI_NAME: &str = "Term4u TUI";
pub const GUI_BINARY_NAME: &str = "term4u";
pub const TUI_BINARY_NAME: &str = "term4u-tui";
pub const APP_ID: &str = "dev.term4u.Term4u";
pub const GUI_LOG_FILE: &str = "term4u.log";
pub const TUI_LOG_FILE: &str = "term4u-tui.log";
pub const URL_SCHEME: &str = "term4u";
pub const MACOS_GUI_CONFIG_DIR: &str = ".term4u";
pub const MACOS_TUI_CONFIG_DIR: &str = ".term4u-tui";
pub const LINUX_XDG_APP_DIR: &str = "term4u";
pub const GUI_KEYRING_SERVICE: &str = APP_ID;
pub const TUI_KEYRING_SERVICE: &str = "dev.term4u.Term4u.tui";
pub const TUI_KEYRING_SUFFIX: &str = ".tui";
pub const DOCK_TILE_PLUGIN_ID: &str = "dev.term4u.Term4uDockTilePlugin";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProductFrontend {
    Gui,
    Tui,
}

pub fn app_id() -> AppId {
    AppId::new("dev", "term4u", GUI_NAME)
}

pub const fn binary_name(frontend: ProductFrontend) -> &'static str {
    match frontend {
        ProductFrontend::Gui => GUI_BINARY_NAME,
        ProductFrontend::Tui => TUI_BINARY_NAME,
    }
}

pub const fn display_name(frontend: ProductFrontend) -> &'static str {
    match frontend {
        ProductFrontend::Gui => GUI_NAME,
        ProductFrontend::Tui => TUI_NAME,
    }
}

pub const fn log_file(frontend: ProductFrontend) -> &'static str {
    match frontend {
        ProductFrontend::Gui => GUI_LOG_FILE,
        ProductFrontend::Tui => TUI_LOG_FILE,
    }
}

pub fn keyring_service(data_domain: &str, frontend: ProductFrontend) -> Cow<'_, str> {
    match frontend {
        ProductFrontend::Gui => Cow::Borrowed(data_domain),
        ProductFrontend::Tui => Cow::Owned(format!("{data_domain}{TUI_KEYRING_SUFFIX}")),
    }
}

#[cfg(test)]
#[path = "product_identity_tests.rs"]
mod tests;
