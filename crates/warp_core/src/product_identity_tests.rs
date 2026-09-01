use super::*;

#[test]
fn identity_values_are_canonical() {
    assert_eq!(app_id().to_string(), APP_ID);
    assert_eq!(display_name(ProductFrontend::Gui), "Term4u");
    assert_eq!(display_name(ProductFrontend::Tui), "Term4u TUI");
    assert_eq!(binary_name(ProductFrontend::Gui), "term4u");
    assert_eq!(binary_name(ProductFrontend::Tui), "term4u-tui");
    assert_eq!(log_file(ProductFrontend::Gui), "term4u.log");
    assert_eq!(log_file(ProductFrontend::Tui), "term4u-tui.log");
    assert_eq!(URL_SCHEME, "term4u");
    assert_eq!(MACOS_GUI_CONFIG_DIR, ".term4u");
    assert_eq!(MACOS_TUI_CONFIG_DIR, ".term4u-tui");
    assert_eq!(LINUX_XDG_APP_DIR, "term4u");
    assert_eq!(GUI_KEYRING_SERVICE, APP_ID);
    assert_eq!(TUI_KEYRING_SERVICE, format!("{APP_ID}.tui"));
    assert_eq!(DOCK_TILE_PLUGIN_ID, "dev.term4u.Term4uDockTilePlugin");
}

#[test]
fn frontends_use_distinct_keyring_services() {
    assert_eq!(
        keyring_service(APP_ID, ProductFrontend::Gui),
        GUI_KEYRING_SERVICE
    );
    assert_eq!(
        keyring_service(APP_ID, ProductFrontend::Tui),
        TUI_KEYRING_SERVICE
    );
}
