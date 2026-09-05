use serde::{Deserialize, Serialize};
use warpui::elements::{DraggableState, MouseStateHandle};
use warpui::{Entity, SingletonEntity, ViewHandle};

use crate::launch_configs::launch_config::LaunchConfig;
use crate::pane_group::PaneGroup;
use crate::themes::theme::AnsiColorIdentifier;
use crate::workspace::tab_group::TabGroupId;

pub const TAB_BAR_BORDER_HEIGHT: f32 = 1.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectedTabColor {
    #[default]
    Unset,
    Cleared,
    Color(AnsiColorIdentifier),
}

impl SelectedTabColor {
    pub(crate) fn resolve(
        self,
        default: Option<AnsiColorIdentifier>,
    ) -> Option<AnsiColorIdentifier> {
        match self {
            Self::Color(color) => Some(color),
            Self::Cleared => None,
            Self::Unset => default,
        }
    }
}

#[derive(Debug, Clone)]
pub enum NewSessionMenuItem {
    OpenLaunchConfig(LaunchConfig),
    OpenLaunchConfigDocs,
    CreateNewTabConfig,
    CreateNewTabGroup,
}

#[derive(Clone)]
pub struct TabData {
    pub pane_group: ViewHandle<PaneGroup>,
    pub tab_mouse_state: MouseStateHandle,
    pub close_mouse_state: MouseStateHandle,
    pub tooltip_mouse_state: MouseStateHandle,
    pub draggable_state: DraggableState,
    pub default_directory_color: Option<AnsiColorIdentifier>,
    pub selected_color: SelectedTabColor,
    pub indicator_hover_state: MouseStateHandle,
    pub detached: bool,
    pub group_id: Option<TabGroupId>,
    pub in_multi_selection: bool,
    pub pinned: bool,
}

impl TabData {
    pub fn new(pane_group: ViewHandle<PaneGroup>) -> Self {
        Self {
            pane_group,
            tab_mouse_state: Default::default(),
            close_mouse_state: Default::default(),
            tooltip_mouse_state: Default::default(),
            draggable_state: Default::default(),
            default_directory_color: None,
            selected_color: SelectedTabColor::Unset,
            indicator_hover_state: Default::default(),
            detached: false,
            group_id: None,
            in_multi_selection: false,
            pinned: false,
        }
    }

    pub fn color(&self) -> Option<AnsiColorIdentifier> {
        self.selected_color.resolve(self.default_directory_color)
    }
}

pub struct TabShortcutModifierState;

impl TabShortcutModifierState {
    pub fn new() -> Self {
        Self
    }
}

impl Entity for TabShortcutModifierState {
    type Event = ();
}

impl SingletonEntity for TabShortcutModifierState {}

pub fn tab_position_id(index: usize) -> String {
    format!("tab_{index}")
}
