use serde::{Deserialize, Serialize};

pub use warp_terminal::ImageProtocol;
use crate::terminal::model::session::SessionId;

#[derive(Clone, Serialize, Deserialize)]
pub struct BootstrappingInfo {
    pub shell: &'static str,
    pub is_ssh: bool,
    pub is_subshell: bool,
    pub is_wsl: bool,
    pub is_msys2: bool,
    pub was_triggered_by_rc_file: bool,
    pub bootstrap_duration_seconds: Option<f64>,
    pub rcfiles_duration_seconds: Option<f64>,
    pub warp_attributed_bootstrap_duration_seconds: Option<f64>,
    pub shell_version: Option<String>,
    pub terminal_session_id: Option<SessionId>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum CloseTarget {
    App,
    Window,
    Tab,
    Pane,
    EditorTab,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum PtySpawnMode {
    TerminalServer,
    FallbackToDirect,
    Direct,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum LaunchConfigUiLocation {
    CommandPalette,
    AppMenu,
    TabMenu,
    Uri,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum FindOption {
    CaseSensitive,
    FindInBlock,
    Regex,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum PaletteSource {
    PrefixChange,
    Keybinding,
    CtrlTab { shift_pressed_initially: bool },
    QuitModal,
    IntegrationTest,
    TitleBarSearchBar,
    WarpDrive,
    ConversationManager,
    ContextChip,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum FileTreeSource {
    PaneHeader,
    Keybinding,
    LeftPanelToolbelt,
    ForceOpened,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum CodePanelsFileOpenEntrypoint {
    CodeReview,
    ProjectExplorer,
    GlobalSearch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CLIAgentType {
    Unknown,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum UndoCloseItemType {
    Window,
    Tab,
    Pane,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PromptChoice {
    PS1,
    Default,
    Custom { builtin_chips: Vec<String> },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToggleBlockFilterSource {
    Binding,
    ContextMenu,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum InteractionSource {
    Button,
    Keybinding,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddTabWithShellSource {
    CommandPalette,
    ShellSelectorMenu,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum CodeContextDestination {
    Pty,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum InputUXChangeOrigin {
    #[default]
    Settings,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AutoReloadModalAction {
    Dismissed,
    EnabledAutoReload,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum SharingDialogSource {
    PaneHeader,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum WarpDriveSource {
    Legacy,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum AgentModeRewindEntrypoint {
    Button,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum TelemetrySpace {
    Personal,
}

#[derive(Clone, Debug)]
pub struct NotebookTelemetryMetadata;

#[derive(Clone, Debug)]
pub struct NotebookActionEvent;

#[derive(Clone, Debug)]
pub enum TelemetryEvent {}
