use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use warp_util::path::LineAndColumnArg;
use warpui::{EntityId, WindowId};

use crate::palette::PaletteMode;
use crate::pane_group::PaneViewLocator;
use crate::prompt::editor_modal::OpenSource as PromptEditorOpenSource;
use crate::server::telemetry::{AddTabWithShellSource, PaletteSource};
use crate::settings_view::SettingsSection;
use crate::terminal::available_shells::AvailableShell;
use crate::themes::theme_chooser::ThemeChooserMode;
use crate::workflows::{WorkflowSelectionSource, WorkflowSource, WorkflowType};

#[derive(Clone, Default, Debug)]
pub enum InitContent {
    #[default]
    FromInputBuffer,
    Custom(String),
}

#[derive(Clone, Default, Debug)]
pub struct CommandSearchOptions {
    pub filter: Option<crate::search::QueryFilter>,
    pub init_content: InitContent,
}

#[derive(Debug, Clone, Copy)]
pub enum TabContextMenuAnchor {
    Pointer(pathfinder_geometry::vector::Vector2F),
    VerticalTabsKebab,
}

#[derive(Debug, Clone, Copy)]
pub enum VerticalTabsPaneContextMenuTarget {
    ClickedPane(PaneViewLocator),
    ActivePane(PaneViewLocator),
}

impl VerticalTabsPaneContextMenuTarget {
    pub fn locator(self) -> PaneViewLocator {
        match self {
            Self::ClickedPane(locator) | Self::ActivePane(locator) => locator,
        }
    }
}

#[derive(Debug, Clone)]
pub enum WorkspaceAction {
    ActivateTab(usize),
    ActivateTabByNumber(usize),
    ActivatePrevTab,
    ActivateNextTab,
    ActivateLastTab,
    CyclePrevSession,
    CycleNextSession,
    MoveTabLeft(usize),
    MoveTabRight(usize),
    CloseTab(usize),
    CloseActiveTab,
    AddDefaultTab,
    AddTerminalTab { hide_homepage: bool },
    AddTabWithShell { shell: AvailableShell, source: AddTabWithShellSource },
    AddWindowWithShell { shell: AvailableShell },
    ShowSettings,
    ShowSettingsPage(SettingsSection),
    ShowSettingsPageWithSearch { search_query: String, section: Option<SettingsSection> },
    ScrollToSettingsWidget { page: SettingsSection, widget_id: &'static str },
    ConfigureKeybindingSettings { keybinding_name: Option<String> },
    OpenSettingsFile,
    ShowThemeChooser(ThemeChooserMode),
    ShowThemeChooserForActiveTheme,
    OpenPalette { mode: PaletteMode, source: PaletteSource, query: Option<String> },
    TogglePalette { mode: PaletteMode, source: PaletteSource },
    ShowCommandSearch(CommandSearchOptions),
    ToggleResourceCenter,
    CopyVersion(&'static str),
    CopyTextToClipboard(String),
    SendFeedback,
    OpenRepository { path: Option<String> },
    OpenInExplorer { path: PathBuf },
    OpenPromptEditor { open_source: PromptEditorOpenSource },
    RunCommand(String),
    InsertInInput { content: String, replace_buffer: bool },
    ReopenClosedSession,
    DisableTerminalInputSync,
    ToggleSyncTerminalInputsInTab,
    ToggleSyncAllTerminalInputsInAllTabs,
    ToggleRecordingMode,
    ToggleInBandGenerators,
    ToggleDebugNetworkStatus,
    ToggleShowMemoryStats,
    OpenProjectExplorer,
    OpenGlobalSearch,
    ToggleLeftPanel,
    ToggleVerticalTabsPanel,
    OpenVerticalTabsPanel,
    OpenCodeReviewPanel(PaneViewLocator),
    FocusTerminalViewInWorkspace { terminal_view_id: EntityId },
    FocusPane(PaneViewLocator),
    OpenFileInNewTab { full_path: PathBuf, line_and_column: Option<LineAndColumnArg> },
    RunWorkflow {
        workflow: Arc<WorkflowType>,
        workflow_source: WorkflowSource,
        workflow_selection_source: WorkflowSelectionSource,
        argument_override: Option<HashMap<String, String>>,
    },
    UndoRevertInCodeReviewPane { window_id: WindowId, view_id: EntityId },
    #[cfg(feature = "local_fs")]
    FileRenamed { old_path: PathBuf, new_path: PathBuf },
    #[cfg(feature = "local_fs")]
    FileDeleted { path: PathBuf },
}
