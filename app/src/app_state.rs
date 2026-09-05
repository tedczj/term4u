use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use pathfinder_geometry::rect::RectF;
use serde::{Deserialize, Serialize};
use warpui::platform::FullscreenState;
use warpui::{AppContext, SingletonEntity as _};

use crate::code::editor_management::CodeSource;
use crate::notebooks::NotebookId;
use crate::root_view::quake_mode_window_id;
use crate::settings_view::SettingsSection;
use crate::tab::SelectedTabColor;
use crate::terminal::ShellLaunchData;
use crate::terminal::model::SerializedBlockListItem;
use crate::themes::theme::AnsiColorIdentifier;
use crate::workflows::{Workflow, WorkflowId};
use crate::workspace::WorkspaceRegistry;
use crate::workspace::tab_group::TabGroupId;
use crate::workspace::view::left_panel::ToolPanelView;

#[derive(Debug, Clone, PartialEq)]
pub struct AppState {
    pub windows: Vec<WindowSnapshot>,
    pub active_window_index: Option<usize>,
    pub block_lists: Arc<HashMap<PaneUuid, Vec<SerializedBlockListItem>>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PaneUuid(pub Vec<u8>);

#[derive(Clone, Debug, PartialEq)]
pub struct WindowSnapshot {
    pub tabs: Vec<TabSnapshot>,
    pub active_tab_index: usize,
    pub bounds: Option<RectF>,
    pub fullscreen_state: FullscreenState,
    pub quake_mode: bool,
    pub universal_search_width: Option<f32>,
    pub voltron_width: Option<f32>,
    pub left_panel_open: bool,
    pub vertical_tabs_panel_open: bool,
    pub left_panel_width: Option<f32>,
    pub tab_groups: Vec<TabGroupSnapshot>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TabGroupSnapshot {
    pub id: TabGroupId,
    pub name: Option<String>,
    pub color: SelectedTabColor,
    pub collapsed: bool,
    pub pinned: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TabSnapshot {
    pub custom_title: Option<String>,
    pub root: PaneNodeSnapshot,
    pub default_directory_color: Option<AnsiColorIdentifier>,
    pub selected_color: SelectedTabColor,
    pub left_panel: Option<LeftPanelSnapshot>,
    pub group_id: Option<TabGroupId>,
    pub pinned: bool,
}

impl TabSnapshot {
    pub(crate) fn color(&self) -> Option<AnsiColorIdentifier> {
        self.selected_color.resolve(self.default_directory_color)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PaneNodeSnapshot {
    Branch(BranchSnapshot),
    Leaf(LeafSnapshot),
}

impl PaneNodeSnapshot {
    pub fn has_horizontal_split(&self) -> bool {
        match self {
            Self::Leaf(_) => false,
            Self::Branch(branch) => {
                (branch.direction == SplitDirection::Horizontal && branch.children.len() > 1)
                    || branch
                        .children
                        .iter()
                        .any(|(_, child)| child.has_horizontal_split())
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BranchSnapshot {
    pub direction: SplitDirection,
    pub children: Vec<(PaneFlex, PaneNodeSnapshot)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LeafSnapshot {
    pub is_focused: bool,
    pub custom_vertical_tabs_title: Option<String>,
    pub contents: LeafContents,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LeafContents {
    Terminal(TerminalPaneSnapshot),
    Notebook(NotebookPaneSnapshot),
    Code(CodePaneSnapShot),
    Workflow(WorkflowPaneSnapshot),
    Settings(SettingsPaneSnapshot),
    CodeReview(CodeReviewPaneSnapshot),
    GetStarted,
}

#[cfg(feature = "local_fs")]
impl LeafContents {
    pub(crate) fn is_persisted(&self) -> bool {
        true
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TerminalPaneSnapshot {
    pub uuid: Vec<u8>,
    pub cwd: Option<String>,
    pub shell_launch_data: Option<ShellLaunchData>,
    pub is_active: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NotebookPaneSnapshot {
    LocalNotebook { notebook_id: Option<NotebookId> },
    LocalFileNotebook { path: Option<PathBuf> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct CodePaneTabSnapshot {
    pub path: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CodePaneSnapShot {
    Local {
        tabs: Vec<CodePaneTabSnapshot>,
        active_tab_index: usize,
        source: Option<CodeSource>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum WorkflowPaneSnapshot {
    LocalWorkflow {
        workflow_id: WorkflowId,
        workflow: Workflow,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum SettingsPaneSnapshot {
    Local {
        current_page: SettingsSection,
        search_query: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum CodeReviewPaneSnapshot {
    Local {
        terminal_uuid: Vec<u8>,
        repo_path: PathBuf,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LeftPanelDisplayedTab {
    FileTree,
    GlobalSearch,
}

impl From<ToolPanelView> for LeftPanelDisplayedTab {
    fn from(view: ToolPanelView) -> Self {
        match view {
            ToolPanelView::ProjectExplorer => Self::FileTree,
            ToolPanelView::GlobalSearch { .. } => Self::GlobalSearch,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LeftPanelSnapshot {
    pub left_panel_displayed_tab: LeftPanelDisplayedTab,
    pub pane_group_id: String,
    pub width: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PaneFlex(pub f32);

pub fn get_app_state(app: &AppContext) -> AppState {
    let active_window_id = app.windows().active_window();
    let quake_mode_id = quake_mode_window_id();
    let mut active_window_index = None;
    let mut windows = Vec::new();

    for (index, window_id) in app.window_ids().enumerate() {
        if active_window_id == Some(window_id) {
            active_window_index = Some(index);
        }
        if let Some(workspace) = WorkspaceRegistry::as_ref(app).get(window_id, app) {
            let workspace = workspace.as_ref(app);
            if workspace.is_tab_drag_preview() {
                continue;
            }
            let snapshot = workspace.snapshot(
                window_id,
                quake_mode_id.is_some_and(|id| id == window_id),
                app,
            );
            if !snapshot.tabs.is_empty() {
                windows.push(snapshot);
            }
        }
    }

    AppState {
        windows,
        active_window_index,
        block_lists: Arc::new(HashMap::new()),
    }
}

#[cfg(test)]
#[path = "app_state_tests.rs"]
mod tests;
