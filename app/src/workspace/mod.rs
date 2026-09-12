mod action;
mod active_session;
pub mod header_toolbar_item;
mod registry;
pub mod sync_inputs;
pub mod tab_group;
pub mod tab_settings;
mod toast_stack;
pub mod util;
pub mod view;

pub use action::{
    CommandSearchOptions, InitContent, TabContextMenuAnchor, VerticalTabsPaneContextMenuTarget,
    WorkspaceAction,
};
pub use active_session::ActiveSession;
pub use registry::WorkspaceRegistry;
use serde::{Deserialize, Serialize};
pub use toast_stack::{ToastStack, ToastStackEvent};
pub use util::{PaneViewLocator, TabMovement, active_terminal_in_window};
pub use view::{
    NEW_SESSION_MENU_BUTTON_POSITION_ID, NEW_TAB_BUTTON_POSITION_ID, PANEL_HEADER_HEIGHT,
    TAB_BAR_HEIGHT, TOTAL_TAB_BAR_HEIGHT, WORKSPACE_PADDING, Workspace,
};
use warpui::AppContext;
use warpui::elements::DropTargetData;
use warpui::keymap::EditableBinding;

use crate::settings_view::SettingsSection;
use crate::util::bindings::CustomAction;

#[derive(PartialEq, Copy, Clone, Debug)]
pub struct TabBarDropTargetData {
    pub tab_bar_location: TabBarLocation,
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub struct VerticalTabsPaneDropTargetData {
    pub tab_bar_location: TabBarLocation,
}

#[derive(PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum TabBarLocation {
    TabIndex(usize),
    AfterTabIndex(usize),
}

impl DropTargetData for TabBarDropTargetData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl DropTargetData for VerticalTabsPaneDropTargetData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub fn panel_header_corner_radius() -> warpui::elements::CornerRadius {
    warpui::elements::CornerRadius::with_top(warpui::elements::Radius::Pixels(8.))
}

pub fn init(app: &mut AppContext) {
    use warpui::keymap::macros::*;

    app.add_singleton_model(|_| WorkspaceRegistry::new());
    app.add_singleton_model(|_| ActiveSession::default());
    app.add_singleton_model(|_| ToastStack);
    app.add_singleton_model(|_| sync_inputs::SyncedInputState::new());
    sync_inputs::init(app);
    app.register_editable_bindings(
        [
            (
                "workspace:new_tab",
                "New Tab",
                WorkspaceAction::AddDefaultTab,
                CustomAction::NewTab,
            ),
            (
                "workspace:new_terminal_tab",
                "New Terminal Tab",
                WorkspaceAction::AddDefaultTab,
                CustomAction::NewTerminalTab,
            ),
            (
                "workspace:close_active_tab",
                "Close Tab",
                WorkspaceAction::CloseActiveTab,
                CustomAction::CloseTab,
            ),
            (
                "workspace:activate_next_tab",
                "Next Tab",
                WorkspaceAction::ActivateNextTab,
                CustomAction::ActivateNextTab,
            ),
            (
                "workspace:activate_prev_tab",
                "Previous Tab",
                WorkspaceAction::ActivatePrevTab,
                CustomAction::ActivatePreviousTab,
            ),
            (
                "workspace:cycle_next_session",
                "Next Session",
                WorkspaceAction::CycleNextSession,
                CustomAction::CycleNextSession,
            ),
            (
                "workspace:cycle_prev_session",
                "Previous Session",
                WorkspaceAction::CyclePrevSession,
                CustomAction::CyclePrevSession,
            ),
            (
                "workspace:show_settings",
                "Settings",
                WorkspaceAction::ShowSettings,
                CustomAction::ShowSettings,
            ),
            (
                "workspace:show_settings_about_page",
                "About Term4u",
                WorkspaceAction::ShowSettingsPage(SettingsSection::About),
                CustomAction::ShowAboutWarp,
            ),
            (
                "workspace:show_settings_appearance_page",
                "Appearance",
                WorkspaceAction::ShowSettingsPage(SettingsSection::Appearance),
                CustomAction::ShowAppearance,
            ),
            (
                "workspace:show_settings_keyboard_shortcuts_page",
                "Keyboard Shortcuts",
                WorkspaceAction::ShowSettingsPage(SettingsSection::Keybindings),
                CustomAction::ConfigureKeybindings,
            ),
        ]
        .into_iter()
        .map(|(name, description, action, custom_action)| {
            EditableBinding::new(name, description, action)
                .with_context_predicate(id!("Workspace"))
                .with_custom_action(custom_action)
        }),
    );
}
