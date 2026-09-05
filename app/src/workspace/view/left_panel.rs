#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolPanelView {
    ProjectExplorer,
    GlobalSearch { is_searching: bool },
}
