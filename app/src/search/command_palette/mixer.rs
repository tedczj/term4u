use std::sync::Arc;

use strum_macros::IntoStaticStr;
use warp_util::path::LineAndColumnArg;
use warpui::keymap::BindingId;
use warpui::{EntityId, WindowId};

use crate::launch_configs::launch_config::LaunchConfig;
use crate::search::command_palette::new_session::{NewSessionOption, NewSessionOptionId};
use crate::search::mixer::SearchMixer;
use crate::util::bindings::CommandBinding;
use crate::workspace::PaneViewLocator;

pub type CommandPaletteMixer = SearchMixer<CommandPaletteItemAction>;

#[derive(Clone, Debug)]
pub enum CommandPaletteItemAction {
    AcceptBinding { binding: Arc<CommandBinding> },
    NavigateToSession { pane_view_locator: PaneViewLocator, window_id: WindowId },
    NavigateToTab { pane_group_id: EntityId, window_id: WindowId },
    OpenLaunchConfiguration { config: Arc<LaunchConfig>, open_in_active_window: bool },
    NewSession { source: Arc<NewSessionOption> },
    OpenFile { path: String, project_directory: String, line_and_column_arg: Option<LineAndColumnArg> },
    OpenDirectory { path: String, project_directory: String },
    CreateFile { file_name: String, current_directory: String },
    NoOp,
}

impl CommandPaletteItemAction {
    pub fn to_summary(&self) -> ItemSummary {
        match self {
            Self::AcceptBinding { binding } => ItemSummary::Action { binding_id: binding.id },
            Self::NavigateToSession { pane_view_locator, .. } => ItemSummary::Session { pane_view_locator: *pane_view_locator },
            Self::NavigateToTab { pane_group_id, .. } => ItemSummary::Tab { pane_group_id: *pane_group_id },
            Self::NewSession { source } => ItemSummary::NewSession { id: source.id().clone() },
            Self::OpenLaunchConfiguration { .. } => ItemSummary::LaunchConfiguration,
            Self::OpenFile { path, project_directory, line_and_column_arg } => ItemSummary::File { path: path.clone(), project_directory: project_directory.clone(), line_and_column_arg: *line_and_column_arg },
            Self::OpenDirectory { path, project_directory } => ItemSummary::Directory { path: path.clone(), project_directory: project_directory.clone() },
            Self::CreateFile { .. } | Self::NoOp => ItemSummary::NoOp,
        }
    }

    pub fn result_type(&self) -> &'static str {
        self.to_summary().into()
    }
}

#[derive(Clone, Debug, PartialEq, IntoStaticStr)]
pub enum ItemSummary {
    Action { binding_id: BindingId },
    Session { pane_view_locator: PaneViewLocator },
    Tab { pane_group_id: EntityId },
    NewSession { id: NewSessionOptionId },
    LaunchConfiguration,
    File { path: String, project_directory: String, line_and_column_arg: Option<LineAndColumnArg> },
    Directory { path: String, project_directory: String },
    NoOp,
}
