mod action;
mod active_session;
mod registry;
pub mod header_toolbar_editor;
pub mod header_toolbar_item;
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
pub use toast_stack::{ToastStack, ToastStackEvent};
pub use util::{PaneViewLocator, TabMovement, active_terminal_in_window};
pub use view::{
    NEW_SESSION_MENU_BUTTON_POSITION_ID, NEW_TAB_BUTTON_POSITION_ID, PANEL_HEADER_HEIGHT,
    TAB_BAR_HEIGHT, TOTAL_TAB_BAR_HEIGHT, WORKSPACE_PADDING, Workspace,
};

use serde::{Deserialize, Serialize};
use warpui::elements::DropTargetData;
use warpui::{AppContext, SingletonEntity as _};

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
    fn as_any(&self) -> &dyn std::any::Any { self }
}

impl DropTargetData for VerticalTabsPaneDropTargetData {
    fn as_any(&self) -> &dyn std::any::Any { self }
}

pub fn panel_header_corner_radius() -> warpui::elements::CornerRadius {
    warpui::elements::CornerRadius::with_top(warpui::elements::Radius::Pixels(8.))
}

pub fn init(app: &mut AppContext) {
    app.add_singleton_model(|_| WorkspaceRegistry::new());
    app.add_singleton_model(|_| ActiveSession::default());
    app.add_singleton_model(|_| ToastStack);
    app.add_singleton_model(|_| sync_inputs::SyncedInputState::new());
    sync_inputs::init(app);
}
