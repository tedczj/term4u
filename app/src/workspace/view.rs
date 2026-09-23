pub(crate) mod global_search;
pub(crate) mod left_panel;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use warp_util::local_or_remote_path::LocalOrRemotePath;
use warp_util::path::{LineAndColumnArg, ShellFamily};
use warpui::clipboard::ClipboardContent;
use warpui::elements::{
    Align, ChildView, ConstrainedBox, Container, CrossAxisAlignment, Element, EventHandler,
    Expanded, Flex, MainAxisSize, ParentElement, Stack, Text,
};
use warpui::platform::FilePickerConfiguration;
use warpui::{
    AppContext, Entity, EntityId, FocusContext, ModelHandle, SingletonEntity, TypedActionView,
    View, ViewContext, ViewHandle, WindowId,
};

use self::global_search::view::{Event as GlobalSearchViewEvent, GlobalSearchView};
use super::sync_inputs::SyncedInputState;
use super::tab_group::{TabGroup, TabGroupId};
use super::{
    ActiveSession, CommandSearchOptions, InitContent, PaneViewLocator, ToastStack, ToastStackEvent,
    WorkspaceAction, WorkspaceRegistry,
};
use crate::GlobalResourceHandles;
use crate::ai::persisted_workspace::PersistedWorkspace;
use crate::app_state::{
    LeftPanelDisplayedTab, LeftPanelSnapshot, PaneUuid, TabGroupSnapshot, TabSnapshot,
    WindowSnapshot,
};
use crate::appearance::Appearance;
use crate::code::editor_management::CodeSource;
use crate::code::file_tree::{FileTreeEvent, FileTreeView};
use crate::code_review::{GlobalCodeReviewEvent, GlobalCodeReviewModel};
use crate::notebooks::manager::{NotebookManager, NotebookSource};
use crate::palette::PaletteMode;
use crate::pane_group::{
    CodePane, Direction, Event as PaneGroupEvent, LeftPanelTargetView, NewTerminalOptions,
    PaneGroup, PanesLayout, WorkingDirectoriesEvent, WorkingDirectoriesModel,
};
use crate::quit_warning::UnsavedStateSummary;
use crate::root_view::NewWorkspaceSource;
use crate::search::QueryFilter;
use crate::search::command_palette::view::{
    Event as CommandPaletteEvent, NavigationMode, View as CommandPalette,
};
use crate::search::command_search::searcher::{AcceptedWorkflow, CommandSearchItemAction};
use crate::search::command_search::view::{CommandSearchEvent, CommandSearchView};
use crate::session_management::SessionSource;
use crate::settings_view::pane_manager::SettingsPaneManager;
use crate::settings_view::{SettingsSection, SettingsView};
use crate::tab::{SelectedTabColor, TabData};
use crate::terminal::TerminalView;
use crate::terminal::input::MenuPositioning;
use crate::terminal::model::SerializedBlockListItem;
use crate::terminal::model::terminal_model::TerminalInputState;
use crate::undo_close::UndoCloseStack;
use crate::user_config::{WarpConfig, WarpConfigUpdateEvent};
use crate::util::openable_file_type::{EditorLayout, FileTarget};
use crate::view_components::{DismissibleToast, DismissibleToastStack};
use crate::workflows::command_parser::{
    compute_workflow_display_data, compute_workflow_display_data_with_overrides,
};
use crate::workflows::manager::{WorkflowManager, WorkflowOpenSource};
use crate::workflows::{Workflow, WorkflowViewMode};

pub const WORKSPACE_PADDING: f32 = 8.;
pub const TAB_BAR_HEIGHT: f32 = 36.;
pub const TOTAL_TAB_BAR_HEIGHT: f32 = TAB_BAR_HEIGHT + crate::tab::TAB_BAR_BORDER_HEIGHT;
pub const PANEL_HEADER_HEIGHT: f32 = 36.;
pub const NEW_TAB_BUTTON_POSITION_ID: &str = "new_tab_button";
pub const NEW_SESSION_MENU_BUTTON_POSITION_ID: &str = "new_session_menu_button";
pub const TOGGLE_RIGHT_PANEL_BINDING_NAME: &str = "workspace:toggle_right_panel";

pub struct Workspace {
    resources: GlobalResourceHandles,
    tabs: Vec<TabData>,
    active_tab_index: usize,
    tab_groups: HashMap<TabGroupId, TabGroup>,
    left_panel_open: bool,
    vertical_tabs_panel_open: bool,
    tab_drag_preview: bool,
    command_palette: Option<ViewHandle<CommandPalette>>,
    command_search: Option<ViewHandle<CommandSearchView>>,
    working_directories: ModelHandle<WorkingDirectoriesModel>,
    global_search_views: HashMap<EntityId, ViewHandle<GlobalSearchView>>,
    toasts: ViewHandle<DismissibleToastStack<WorkspaceAction>>,
}

impl Workspace {
    pub fn new(
        resources: GlobalResourceHandles,
        source: NewWorkspaceSource,
        ctx: &mut ViewContext<Self>,
    ) -> Self {
        let window_id = ctx.window_id();
        let toasts =
            ctx.add_typed_action_view(|_| DismissibleToastStack::new(Duration::from_secs(8)));
        ctx.subscribe_to_model(&ToastStack::handle(ctx), |workspace, _, event, ctx| {
            match event {
                ToastStackEvent::AddEphemeralToast { window_id, toast }
                    if *window_id == ctx.window_id() =>
                {
                    workspace.toasts.update(ctx, |toasts, ctx| {
                        toasts.add_ephemeral_toast(toast.clone(), ctx)
                    });
                }
                ToastStackEvent::AddPersistentToast { window_id, toast }
                    if *window_id == ctx.window_id() =>
                {
                    workspace.toasts.update(ctx, |toasts, ctx| {
                        toasts.add_persistent_toast(toast.clone(), ctx)
                    });
                }
                ToastStackEvent::RemoveToast {
                    window_id,
                    identifier,
                } if *window_id == ctx.window_id() => {
                    workspace.toasts.update(ctx, |toasts, ctx| {
                        toasts.dismiss_older_toasts(identifier, ctx)
                    });
                }
                ToastStackEvent::AddEphemeralToast { .. }
                | ToastStackEvent::AddPersistentToast { .. }
                | ToastStackEvent::RemoveToast { .. } => return,
            }
            ctx.notify();
        });
        ctx.subscribe_to_model(&WarpConfig::handle(ctx), |workspace, _, event, ctx| {
            workspace.toasts.update(ctx, |toasts, ctx| match event {
                WarpConfigUpdateEvent::SettingsErrors(error) => {
                    let (heading, description) = error.heading_and_description();
                    toasts.add_persistent_toast(
                        DismissibleToast::error(format!("{heading} {description}"))
                            .with_object_id("settings-file-error".into()),
                        ctx,
                    );
                }
                WarpConfigUpdateEvent::SettingsErrorsCleared => {
                    toasts.dismiss_older_toasts("settings-file-error", ctx);
                }
                WarpConfigUpdateEvent::TabConfigErrors(errors) => {
                    toasts.dismiss_toasts_by_prefix("tab-config-error:", ctx);
                    for error in errors {
                        toasts.add_persistent_toast(
                            DismissibleToast::error(format!(
                                "Unable to load {}: {}",
                                error.file_name, error.error_message
                            ))
                            .with_object_id(format!(
                                "tab-config-error:{}",
                                error.file_path.display()
                            )),
                            ctx,
                        );
                    }
                }
                WarpConfigUpdateEvent::Themes
                | WarpConfigUpdateEvent::LocalUserWorkflows
                | WarpConfigUpdateEvent::LaunchConfigs
                | WarpConfigUpdateEvent::TabConfigs
                | WarpConfigUpdateEvent::Settings => {}
            });
        });
        let settings_view = ctx.add_typed_action_view(|ctx| SettingsView::new(None, ctx));
        SettingsPaneManager::handle(ctx).update(ctx, |manager, _| {
            manager.register_view(window_id, settings_view);
        });
        let working_directories = ctx.add_model(|_| WorkingDirectoriesModel::new());
        ctx.subscribe_to_model(&working_directories, |workspace, _, event, ctx| {
            if let WorkingDirectoriesEvent::DirectoriesChanged {
                pane_group_id,
                directories,
            } = event
            {
                if let Some(tree) = workspace
                    .working_directories
                    .as_ref(ctx)
                    .get_file_tree_view(*pane_group_id)
                {
                    let paths = directories
                        .iter()
                        .filter_map(|directory| directory.path.to_local_path().map(PathBuf::from))
                        .collect();
                    tree.update(ctx, |tree, ctx| tree.set_root_directories(paths, ctx));
                }
                if let Some(search) = workspace.global_search_views.get(pane_group_id) {
                    let paths = directories
                        .iter()
                        .map(|directory| directory.path.clone())
                        .collect();
                    search.update(ctx, |search, ctx| search.set_root_directories(paths, ctx));
                }
            }
        });
        let mut workspace = Self {
            resources,
            tabs: Vec::new(),
            active_tab_index: 0,
            tab_groups: HashMap::new(),
            left_panel_open: false,
            vertical_tabs_panel_open: false,
            tab_drag_preview: false,
            command_palette: None,
            command_search: None,
            working_directories,
            global_search_views: HashMap::new(),
            toasts,
        };
        workspace.restore_source(source, ctx);
        if workspace.tabs.is_empty() {
            workspace.add_terminal_tab(false, ctx);
        }
        let handle = ctx.handle();
        WorkspaceRegistry::handle(ctx).update(ctx, |registry, _| {
            registry.register(window_id, handle);
        });
        workspace
    }

    #[cfg(test)]
    pub fn new_for_test(resources: GlobalResourceHandles, ctx: &mut ViewContext<Self>) -> Self {
        Self::new(
            resources,
            NewWorkspaceSource::Empty {
                previous_active_window: None,
                shell: None,
            },
            ctx,
        )
    }

    fn restore_source(&mut self, source: NewWorkspaceSource, ctx: &mut ViewContext<Self>) {
        match source {
            NewWorkspaceSource::Empty { shell, .. } => {
                self.add_tab_with_pane_layout(
                    PanesLayout::SingleTerminal(Box::new(NewTerminalOptions {
                        shell,
                        ..Default::default()
                    })),
                    Arc::new(HashMap::new()),
                    None,
                    ctx,
                );
            }
            NewWorkspaceSource::Restored {
                window_snapshot,
                block_lists,
            } => {
                self.left_panel_open = window_snapshot.left_panel_open;
                self.vertical_tabs_panel_open = window_snapshot.vertical_tabs_panel_open;
                let active_tab_index = window_snapshot.active_tab_index;
                for group in window_snapshot.tab_groups {
                    self.tab_groups.insert(
                        group.id,
                        TabGroup {
                            id: group.id,
                            name: group.name,
                            color: group.color,
                            collapsed: group.collapsed,
                            draggable_state: Default::default(),
                            pinned: group.pinned,
                        },
                    );
                }
                for tab in window_snapshot.tabs {
                    let metadata = (
                        tab.default_directory_color,
                        tab.selected_color,
                        tab.group_id,
                        tab.pinned,
                    );
                    self.add_tab_with_pane_layout(
                        PanesLayout::Snapshot(Box::new(tab.root)),
                        block_lists.clone(),
                        tab.custom_title,
                        ctx,
                    );
                    if let Some(created) = self.tabs.last_mut() {
                        created.default_directory_color = metadata.0;
                        created.selected_color = metadata.1;
                        created.group_id = metadata.2;
                        created.pinned = metadata.3;
                        created.left_panel = tab.left_panel;
                    }
                }
                self.activate_tab(active_tab_index.min(self.tabs.len().saturating_sub(1)), ctx);
            }
            NewWorkspaceSource::FromTemplate { window_template } => {
                for tab in window_template.tabs {
                    self.add_tab_with_pane_layout(
                        PanesLayout::Template(tab.layout),
                        Arc::new(HashMap::new()),
                        tab.title,
                        ctx,
                    );
                }
            }
            NewWorkspaceSource::Session { options } => {
                self.add_tab_with_pane_layout(
                    PanesLayout::SingleTerminal(options),
                    Arc::new(HashMap::new()),
                    None,
                    ctx,
                );
            }
            NewWorkspaceSource::NotebookFromFilePath { file_path } => {
                if let Some(path) = file_path {
                    self.open_file_with_target(
                        path.clone(),
                        FileTarget::CodeEditor(EditorLayout::NewTab),
                        None,
                        CodeSource::Finder { path },
                        ctx,
                    );
                }
            }
            NewWorkspaceSource::TransferredTab {
                is_tab_drag_preview,
                ..
            } => {
                self.tab_drag_preview = is_tab_drag_preview;
            }
        }
    }

    fn subscribe_to_pane_group(
        &self,
        pane_group: &ViewHandle<PaneGroup>,
        ctx: &mut ViewContext<Self>,
    ) {
        ctx.subscribe_to_view(pane_group, |workspace, handle, event, ctx| {
            workspace.handle_pane_group_event(handle, event, ctx);
        });
    }

    fn handle_pane_group_event(
        &mut self,
        pane_group: ViewHandle<PaneGroup>,
        event: &PaneGroupEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        match event {
            PaneGroupEvent::Exited { add_to_undo_stack } => {
                if let Some(index) = self
                    .tabs
                    .iter()
                    .position(|tab| tab.pane_group == pane_group)
                {
                    self.close_tab(index, *add_to_undo_stack, ctx);
                }
            }
            PaneGroupEvent::FocusPaneInWorkspace { locator } => self.focus_pane(*locator, ctx),
            PaneGroupEvent::OpenSettings(section) => self.open_settings(*section, None, ctx),
            PaneGroupEvent::OpenFileWithTarget {
                path,
                target,
                line_col,
            } => self.open_file_with_target(
                path.clone(),
                target.clone(),
                *line_col,
                CodeSource::Link {
                    path: path.clone(),
                    range_start: *line_col,
                    range_end: None,
                },
                ctx,
            ),
            PaneGroupEvent::OpenCodeInWarp {
                source,
                layout,
                line_col,
            } => {
                if let Some(path) = source.path() {
                    self.open_file_with_target(
                        path,
                        FileTarget::CodeEditor(*layout),
                        *line_col,
                        source.clone(),
                        ctx,
                    );
                }
            }
            PaneGroupEvent::RunWorkflow {
                workflow,
                argument_override,
                ..
            } => {
                self.use_workflow(
                    workflow.as_workflow(),
                    argument_override.as_ref(),
                    true,
                    ctx,
                );
            }
            PaneGroupEvent::CDToDirectory { path } => self.run_command(
                format!(
                    "cd {}",
                    ShellFamily::Posix.shell_escape(path.to_string_lossy().as_ref())
                ),
                ctx,
            ),
            PaneGroupEvent::OpenDirectoryInNewTab { path } => self.add_tab_with_pane_layout(
                PanesLayout::SingleTerminal(Box::new(
                    NewTerminalOptions::default().with_initial_directory(path),
                )),
                Arc::new(HashMap::new()),
                None,
                ctx,
            ),
            PaneGroupEvent::PaneTitleUpdated => ctx.notify(),
            PaneGroupEvent::ShowCommandSearch(options) => self.show_command_search(options, ctx),
            PaneGroupEvent::AppStateChanged => {
                self.refresh_pane_directories(&pane_group, ctx);
                self.refresh_active_session(ctx);
                ctx.notify();
            }
            PaneGroupEvent::ActiveSessionChanged
            | PaneGroupEvent::TerminalViewStateChanged
            | PaneGroupEvent::RepoChanged
            | PaneGroupEvent::PaneFocused => {
                self.refresh_pane_directories(&pane_group, ctx);
                self.refresh_active_session(ctx);
            }
            PaneGroupEvent::OpenPalette { mode, query, .. } => {
                self.open_palette(*mode, query.as_deref(), ctx);
            }
            PaneGroupEvent::OpenFilesPalette { .. } => {
                self.open_palette(PaletteMode::Files, None, ctx);
            }
            PaneGroupEvent::ToggleLeftPanel {
                force_open,
                target_view,
            } => {
                let open =
                    *force_open || !self.left_panel_open || self.left_panel_view() != *target_view;
                self.set_left_panel_view(*target_view);
                self.set_left_panel_open(open, ctx);
            }
            PaneGroupEvent::OpenCodeReviewPane(arg) | PaneGroupEvent::ToggleCodeReviewPane(arg) => {
                if let Some(path) = arg
                    .repo_path
                    .as_ref()
                    .and_then(LocalOrRemotePath::to_local_path)
                {
                    let terminal_id = arg.terminal_view.upgrade(ctx).map(|view| view.id());
                    pane_group.update(ctx, |group, ctx| {
                        group.open_code_review(
                            path.to_owned(),
                            terminal_id,
                            matches!(event, PaneGroupEvent::ToggleCodeReviewPane(_)),
                            arg.focus_new_pane,
                            ctx,
                        )
                    });
                }
            }
            PaneGroupEvent::ExecuteCommand(_)
            | PaneGroupEvent::SyncInput(_)
            | PaneGroupEvent::OpenWorkflowModalWithCommand(_)
            | PaneGroupEvent::OpenWorkflowModalWithTemporary(_)
            | PaneGroupEvent::OpenFileInWarp { .. }
            | PaneGroupEvent::PreviewCodeInWarp { .. }
            | PaneGroupEvent::MaximizePaneToggled
            | PaneGroupEvent::FocusPaneGroup
            | PaneGroupEvent::FocusPane { .. }
            | PaneGroupEvent::DroppedOnTabBar { .. }
            | PaneGroupEvent::SwitchTabFocusAndMovePane { .. }
            | PaneGroupEvent::UpdateHoveredTabIndex { .. }
            | PaneGroupEvent::ClearHoveredTabIndex
            | PaneGroupEvent::ShowToast { .. }
            | PaneGroupEvent::OpenThemeChooser
            | PaneGroupEvent::LeftPanelToggled { .. }
            | PaneGroupEvent::FileRenamed { .. }
            | PaneGroupEvent::FileDeleted { .. }
            | PaneGroupEvent::OpenLspLogs { .. } => {}
        }
    }

    pub fn add_tab_with_pane_layout(
        &mut self,
        layout: PanesLayout,
        block_lists: Arc<HashMap<PaneUuid, Vec<SerializedBlockListItem>>>,
        custom_title: Option<String>,
        ctx: &mut ViewContext<Self>,
    ) {
        let pane_group = ctx.add_typed_action_view(|ctx| {
            let mut group = PaneGroup::new_with_panes_layout(
                self.resources.tips_completed.clone(),
                self.resources
                    .user_default_shell_unsupported_banner_model_handle
                    .clone(),
                layout,
                block_lists,
                self.resources.model_event_sender.clone(),
                ctx,
            );
            if let Some(title) = custom_title {
                group.set_title(&title, ctx);
            }
            group
        });
        self.subscribe_to_pane_group(&pane_group, ctx);
        self.tabs.push(TabData::new(pane_group));
        self.activate_tab(self.tabs.len() - 1, ctx);
    }

    fn add_tab_for_pane(
        &mut self,
        pane: Box<dyn crate::pane_group::AnyPaneContent>,
        ctx: &mut ViewContext<Self>,
    ) {
        let pane_group = ctx.add_typed_action_view(|ctx| {
            PaneGroup::new_from_existing_pane(
                pane,
                self.resources.tips_completed.clone(),
                self.resources
                    .user_default_shell_unsupported_banner_model_handle
                    .clone(),
                self.resources.model_event_sender.clone(),
                ctx,
            )
        });
        self.subscribe_to_pane_group(&pane_group, ctx);
        self.tabs.push(TabData::new(pane_group));
        self.activate_tab(self.tabs.len() - 1, ctx);
    }

    pub fn add_terminal_tab(&mut self, hide_homepage: bool, ctx: &mut ViewContext<Self>) {
        self.add_tab_with_pane_layout(
            PanesLayout::SingleTerminal(Box::new(NewTerminalOptions {
                hide_homepage,
                ..Default::default()
            })),
            Arc::new(HashMap::new()),
            None,
            ctx,
        );
    }

    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }
    pub fn active_tab_index(&self) -> usize {
        self.active_tab_index
    }
    pub fn tab_views(&self) -> impl Iterator<Item = &ViewHandle<PaneGroup>> {
        self.tabs.iter().map(|tab| &tab.pane_group)
    }
    pub fn active_tab_pane_group(&self) -> &ViewHandle<PaneGroup> {
        &self.tabs[self.active_tab_index].pane_group
    }
    pub fn get_pane_group_view(&self, id: EntityId) -> Option<&ViewHandle<PaneGroup>> {
        self.tabs
            .iter()
            .map(|tab| &tab.pane_group)
            .find(|group| group.id() == id)
    }
    pub fn is_tab_drag_preview(&self) -> bool {
        self.tab_drag_preview
    }
    pub fn window_id(&self, ctx: &AppContext) -> WindowId {
        ctx.window_ids()
            .find(|id| {
                self.tabs
                    .iter()
                    .any(|tab| tab.pane_group.window_id(ctx) == *id)
            })
            .unwrap_or_else(|| self.active_tab_pane_group().window_id(ctx))
    }

    pub fn handle_reopen(&mut self, ctx: &mut ViewContext<Self>) {
        let window_id = ctx.window_id();
        let handle = ctx.handle();
        WorkspaceRegistry::handle(ctx)
            .update(ctx, |registry, _| registry.register(window_id, handle));
        self.activate_tab(self.active_tab_index, ctx);
    }

    pub fn focus_pane(&mut self, locator: PaneViewLocator, ctx: &mut ViewContext<Self>) {
        if let Some(index) = self
            .tabs
            .iter()
            .position(|tab| tab.pane_group.id() == locator.pane_group_id)
        {
            self.active_tab_index = index;
            self.tabs[index].pane_group.update(ctx, |group, ctx| {
                group.reveal_and_focus_pane(locator.pane_id, ctx)
            });
            ctx.notify();
        }
    }

    pub fn workspace_sessions(
        &self,
        window_id: WindowId,
        app: &AppContext,
    ) -> Vec<crate::session_management::SessionNavigationData> {
        self.tabs
            .iter()
            .flat_map(|tab| {
                tab.pane_group
                    .as_ref(app)
                    .pane_sessions(tab.pane_group.id(), window_id, app)
            })
            .collect()
    }

    fn activate_tab(&mut self, index: usize, ctx: &mut ViewContext<Self>) {
        let Some(tab) = self.tabs.get(index) else {
            return;
        };
        self.active_tab_index = index;
        tab.pane_group.update(ctx, |group, ctx| group.focus(ctx));
        let group = tab.pane_group.clone();
        self.refresh_pane_directories(&group, ctx);
        self.refresh_active_session(ctx);
        if self.left_panel_open {
            match self.left_panel_view() {
                LeftPanelTargetView::ProjectExplorer => {
                    self.ensure_file_tree(ctx);
                }
                LeftPanelTargetView::GlobalSearch => {
                    self.ensure_global_search(ctx);
                }
            }
        }
        ctx.notify();
    }

    fn refresh_pane_directories(&self, group: &ViewHandle<PaneGroup>, ctx: &mut ViewContext<Self>) {
        let pane_group = group.as_ref(ctx);
        let terminal_cwds = pane_group
            .terminal_view_working_directories(ctx)
            .filter_map(|(id, path)| path.map(|path| (id, path)))
            .collect();
        let editor_paths = pane_group
            .code_view_paths(ctx)
            .filter_map(|(id, path)| path.map(|path| (id, path)))
            .collect();
        let focused_terminal_id = pane_group.focused_session_view(ctx).map(|view| view.id());
        self.working_directories.update(ctx, |directories, ctx| {
            directories.refresh_working_directories_for_pane_group(
                group.id(),
                terminal_cwds,
                editor_paths,
                focused_terminal_id,
                ctx,
            );
        });
    }

    fn ensure_file_tree(&self, ctx: &mut ViewContext<Self>) -> ViewHandle<FileTreeView> {
        let group = self.active_tab_pane_group();
        let group_id = group.id();
        if let Some(tree) = self
            .working_directories
            .as_ref(ctx)
            .get_file_tree_view(group_id)
        {
            return tree;
        }
        let tree = ctx.add_typed_action_view(FileTreeView::new);
        let directories = self
            .working_directories
            .as_ref(ctx)
            .most_recent_directories_for_pane_group(group_id)
            .into_iter()
            .flatten()
            .filter_map(|directory| directory.path.to_local_path().map(PathBuf::from))
            .collect();
        let active_file = group.as_ref(ctx).active_file_model().clone();
        let has_terminal = group.as_ref(ctx).has_terminal_panes();
        tree.update(ctx, |tree, ctx| {
            tree.set_active_file_model(active_file, ctx);
            tree.set_root_directories(directories, ctx);
            tree.set_has_terminal_session(has_terminal, ctx);
            tree.set_is_active(self.left_panel_open, ctx);
        });
        ctx.subscribe_to_view(&tree, |workspace, _, event, ctx| match event {
            FileTreeEvent::OpenFile {
                path,
                target,
                line_col,
            } => {
                if let Some(local_path) = path.to_local_path() {
                    workspace.open_file_with_target(
                        local_path.to_owned(),
                        target.clone(),
                        *line_col,
                        CodeSource::FileTree {
                            location: path.clone(),
                        },
                        ctx,
                    );
                }
            }
            FileTreeEvent::CDToDirectory { path } => workspace.run_command(
                format!(
                    "cd {}",
                    ShellFamily::Posix.shell_escape(path.to_string_lossy().as_ref())
                ),
                ctx,
            ),
            FileTreeEvent::OpenDirectoryInNewTab { path } => workspace.add_tab_with_pane_layout(
                PanesLayout::SingleTerminal(Box::new(
                    NewTerminalOptions::default().with_initial_directory(path),
                )),
                Arc::new(HashMap::new()),
                None,
                ctx,
            ),
            FileTreeEvent::FileRenamed { .. }
            | FileTreeEvent::FileDeleted { .. }
            | FileTreeEvent::AttachAsContext { .. } => {}
        });
        self.working_directories.update(ctx, |directories, _| {
            directories.store_file_tree_view(group_id, tree.clone())
        });
        tree
    }

    fn left_panel_view(&self) -> LeftPanelTargetView {
        match self.tabs[self.active_tab_index]
            .left_panel
            .as_ref()
            .map(|panel| &panel.left_panel_displayed_tab)
        {
            Some(LeftPanelDisplayedTab::GlobalSearch) => LeftPanelTargetView::GlobalSearch,
            Some(LeftPanelDisplayedTab::FileTree) | None => LeftPanelTargetView::ProjectExplorer,
        }
    }

    fn set_left_panel_view(&mut self, view: LeftPanelTargetView) {
        let tab = &mut self.tabs[self.active_tab_index];
        let panel = tab.left_panel.get_or_insert_with(|| LeftPanelSnapshot {
            left_panel_displayed_tab: LeftPanelDisplayedTab::FileTree,
            pane_group_id: tab.pane_group.id().to_string(),
            width: 280,
        });
        panel.left_panel_displayed_tab = match view {
            LeftPanelTargetView::ProjectExplorer => LeftPanelDisplayedTab::FileTree,
            LeftPanelTargetView::GlobalSearch => LeftPanelDisplayedTab::GlobalSearch,
        };
    }

    fn set_left_panel_open(&mut self, open: bool, ctx: &mut ViewContext<Self>) {
        self.left_panel_open = open;
        match self.left_panel_view() {
            LeftPanelTargetView::ProjectExplorer => {
                let tree = self.ensure_file_tree(ctx);
                tree.update(ctx, |tree, ctx| {
                    tree.set_is_active(open, ctx);
                    if open {
                        tree.on_left_panel_focused(ctx);
                    }
                });
            }
            LeftPanelTargetView::GlobalSearch => {
                let search = self.ensure_global_search(ctx);
                if open {
                    search.update(ctx, |search, ctx| search.on_left_panel_focused(ctx));
                }
            }
        }
        self.active_tab_pane_group().update(ctx, |group, ctx| {
            group.set_left_panel_open(open, ctx);
            if !open {
                group.focus(ctx);
            }
        });
        ctx.notify();
    }

    fn ensure_global_search(
        &mut self,
        ctx: &mut ViewContext<Self>,
    ) -> ViewHandle<GlobalSearchView> {
        let group_id = self.active_tab_pane_group().id();
        if let Some(search) = self.global_search_views.get(&group_id) {
            return search.clone();
        }
        let search = ctx.add_typed_action_view(GlobalSearchView::new);
        let directories = self
            .working_directories
            .as_ref(ctx)
            .most_recent_directories_for_pane_group(group_id)
            .into_iter()
            .flatten()
            .map(|directory| directory.path)
            .collect();
        search.update(ctx, |search, ctx| {
            search.set_root_directories(directories, ctx)
        });
        ctx.subscribe_to_view(&search, |workspace, _, event, ctx| match event {
            GlobalSearchViewEvent::OpenMatch {
                location,
                line_number,
                column_num,
            } => {
                if let Some(path) = location.to_local_path() {
                    let line_col = Some(LineAndColumnArg {
                        line_num: *line_number as usize,
                        column_num: *column_num,
                    });
                    workspace.open_file_with_target(
                        path.to_owned(),
                        FileTarget::CodeEditor(EditorLayout::NewTab),
                        line_col,
                        CodeSource::Link {
                            path: path.to_owned(),
                            range_start: line_col,
                            range_end: None,
                        },
                        ctx,
                    );
                }
            }
        });
        self.global_search_views.insert(group_id, search.clone());
        search
    }

    fn refresh_active_session(&self, ctx: &mut ViewContext<Self>) {
        let group = self.active_tab_pane_group().as_ref(ctx);
        let terminal = group.active_session_view(ctx);
        let session = terminal
            .as_ref()
            .and_then(|view| view.as_ref(ctx).active_session(ctx));
        let directory = terminal
            .as_ref()
            .and_then(|view| view.as_ref(ctx).pwd_as_local_or_remote(ctx))
            .or_else(|| {
                let path = PathBuf::from(group.path_from_focused_pane(ctx)?);
                if !path.is_file() {
                    return None;
                }
                path.parent()?
                    .canonicalize()
                    .ok()
                    .map(LocalOrRemotePath::Local)
            });
        let window_id = ctx.window_id();
        if ActiveSession::as_ref(ctx).working_directory(window_id) != directory.as_ref()
            && let Some(LocalOrRemotePath::Local(path)) = &directory
        {
            PersistedWorkspace::handle(ctx)
                .update(ctx, |workspace, ctx| workspace.navigated_to_path(path, ctx));
        }
        ActiveSession::handle(ctx).update(ctx, |active, ctx| {
            active.set_session_state(
                window_id,
                session,
                directory,
                terminal.map(|view| view.id()),
                ctx,
            );
        });
    }

    fn open_repository(&mut self, path: Option<&str>, ctx: &mut ViewContext<Self>) {
        if let Some(path) = path {
            PersistedWorkspace::handle(ctx).update(ctx, |workspace, ctx| {
                workspace.user_added_workspace(path.into(), ctx)
            });
            self.add_tab_with_pane_layout(
                PanesLayout::SingleTerminal(Box::new(
                    NewTerminalOptions::default().with_initial_directory(path),
                )),
                Arc::new(HashMap::new()),
                None,
                ctx,
            );
            self.set_left_panel_view(LeftPanelTargetView::ProjectExplorer);
            self.set_left_panel_open(true, ctx);
        } else {
            let window_id = ctx.window_id();
            let workspace_id = ctx.view_id();
            ctx.open_file_picker(
                move |result, ctx| {
                    if let Ok(paths) = result
                        && let Some(path) = paths.into_iter().next()
                    {
                        ctx.dispatch_typed_action_for_view(
                            window_id,
                            workspace_id,
                            &WorkspaceAction::OpenRepository { path: Some(path) },
                        );
                    }
                },
                FilePickerConfiguration::new().folders_only(),
            );
        }
    }

    fn open_code_review(&mut self, group: ViewHandle<PaneGroup>, ctx: &mut ViewContext<Self>) {
        let terminal = group.as_ref(ctx).active_session_view(ctx);
        let path = terminal
            .as_ref()
            .and_then(|view| view.as_ref(ctx).canonical_session_pwd_if_local(ctx))
            .or_else(|| {
                group
                    .as_ref(ctx)
                    .path_from_focused_pane(ctx)
                    .map(PathBuf::from)
                    .and_then(|path| path.parent().map(PathBuf::from))
            });
        let Some(path) = path else {
            return;
        };
        let future = repo_metadata::repositories::DetectedRepositories::handle(ctx).update(
            ctx,
            |repos, ctx| {
                repos.detect_possible_local_git_repo(
                    &path.to_string_lossy(),
                    repo_metadata::repositories::RepoDetectionSource::CodeReviewInitialization,
                    ctx,
                )
            },
        );
        ctx.spawn(future, move |workspace, root, ctx| {
            if workspace.tabs.iter().any(|tab| tab.pane_group == group) {
                group.update(ctx, |group, ctx| {
                    group.open_code_review(
                        root.unwrap_or(path),
                        terminal.map(|view| view.id()),
                        false,
                        true,
                        ctx,
                    )
                });
                ctx.notify();
            }
        });
    }

    fn open_palette(
        &mut self,
        mode: PaletteMode,
        query: Option<&str>,
        ctx: &mut ViewContext<Self>,
    ) {
        self.command_search = None;
        let filter = match mode {
            PaletteMode::Command => QueryFilter::Actions,
            PaletteMode::Navigation => QueryFilter::Sessions,
            PaletteMode::LaunchConfig => QueryFilter::LaunchConfigurations,
            PaletteMode::Files => QueryFilter::Files,
            PaletteMode::WarpDrive | PaletteMode::Conversations => return,
        };
        let window_id = ctx.window_id();
        let binding_view = self
            .command_palette
            .as_ref()
            .and_then(|palette| palette.as_ref(ctx).source_view(ctx))
            .or_else(|| ctx.focused_view_id(window_id))
            .unwrap_or(ctx.view_id());
        let group = self.active_tab_pane_group();
        let session_source = SessionSource::Set {
            active_pane_id: group.as_ref(ctx).focused_pane_id(ctx),
            active_tab_id: group.id(),
            active_window_id: window_id,
        };
        self.refresh_active_session(ctx);
        let palette =
            ctx.add_typed_action_view(|ctx| CommandPalette::new(NavigationMode::Normal, ctx));
        ctx.subscribe_to_view(&palette, |workspace, palette, event, ctx| match event {
            CommandPaletteEvent::Close { .. } => {
                if workspace.command_palette.as_ref() != Some(&palette) {
                    return;
                }
                workspace.command_palette = None;
                if workspace.command_search.is_none() {
                    workspace
                        .active_tab_pane_group()
                        .update(ctx, |group, ctx| group.focus(ctx));
                }
                ctx.notify();
            }
            CommandPaletteEvent::OpenFile {
                path,
                line_and_column_arg,
            } => {
                let path = PathBuf::from(path);
                workspace.open_file_with_target(
                    path.clone(),
                    FileTarget::CodeEditor(EditorLayout::NewTab),
                    *line_and_column_arg,
                    CodeSource::Link {
                        path,
                        range_start: *line_and_column_arg,
                        range_end: None,
                    },
                    ctx,
                );
            }
            CommandPaletteEvent::OpenDirectory { path } => workspace.add_tab_with_pane_layout(
                PanesLayout::SingleTerminal(Box::new(
                    NewTerminalOptions::default().with_initial_directory(path),
                )),
                Arc::new(HashMap::new()),
                None,
                ctx,
            ),
        });
        palette.update(ctx, |palette, ctx| {
            palette.set_binding_source(window_id, binding_view, ctx);
            palette.set_session_source(session_source, ctx);
            palette.set_active_query_filter(filter, ctx);
            if let Some(query) = query {
                palette.insert_query_text(query, ctx);
            }
        });
        ctx.focus(&palette);
        self.command_palette = Some(palette);
        ctx.notify();
    }

    fn show_command_search(&mut self, options: &CommandSearchOptions, ctx: &mut ViewContext<Self>) {
        self.command_palette = None;
        self.refresh_active_session(ctx);
        let active = ActiveSession::as_ref(ctx);
        let session = active.session(ctx.window_id());
        let directory = active.path_if_local(ctx.window_id()).map(PathBuf::from);
        let query = match &options.init_content {
            InitContent::Custom(query) => query.clone(),
            InitContent::FromInputBuffer => self
                .active_tab_pane_group()
                .as_ref(ctx)
                .active_session_view(ctx)
                .map(|view| view.as_ref(ctx).input().as_ref(ctx).buffer_text(ctx))
                .unwrap_or_default(),
        };
        let origin = self
            .active_tab_pane_group()
            .as_ref(ctx)
            .active_session_view(ctx);
        let origin = origin.map(|terminal| {
            let view = terminal.as_ref(ctx);
            let session_id = view.active_session(ctx).map(|session| session.id());
            let block_id = view.model.lock().block_list().active_block().id().clone();
            let revision = view
                .input()
                .as_ref(ctx)
                .editor()
                .as_ref(ctx)
                .content_and_selection_revision(ctx);
            (terminal.downgrade(), session_id, block_id, revision)
        });
        let search = ctx.add_typed_action_view(CommandSearchView::new);
        ctx.subscribe_to_view(&search, move |workspace, search, event, ctx| {
            if workspace.command_search.as_ref() != Some(&search) {
                return;
            }
            if let CommandSearchEvent::ItemSelected { payload, .. } = event
                && matches!(
                    payload.as_ref(),
                    CommandSearchItemAction::AcceptHistory(_)
                        | CommandSearchItemAction::ExecuteHistory(_)
                )
            {
                let valid =
                    origin
                        .as_ref()
                        .is_some_and(|(origin, session_id, block_id, revision)| {
                            let Some(terminal) = origin.upgrade(ctx) else {
                                return false;
                            };
                            if workspace
                                .active_tab_pane_group()
                                .as_ref(ctx)
                                .active_session_view(ctx)
                                .as_ref()
                                != Some(&terminal)
                            {
                                return false;
                            }
                            let view = terminal.as_ref(ctx);
                            let model = view.model.lock();
                            let session_valid =
                                view.active_session(ctx).map(|session| session.id()) == *session_id
                                    && model.block_list().active_block().id() == block_id
                                    && matches!(
                                        model.terminal_input_state(),
                                        TerminalInputState::InputEditor
                                            | TerminalInputState::NotBootstrapped
                                    );
                            drop(model);
                            session_valid
                                && view
                                    .input()
                                    .as_ref(ctx)
                                    .editor()
                                    .as_ref(ctx)
                                    .content_and_selection_revision(ctx)
                                    == *revision
                        });
                if !valid {
                    return;
                }
            }
            match event {
                CommandSearchEvent::ItemSelected { payload, .. } => match payload.as_ref() {
                    CommandSearchItemAction::AcceptHistory(item) => {
                        workspace.insert_in_input(&item.command, true, ctx)
                    }
                    CommandSearchItemAction::ExecuteHistory(command) => {
                        workspace.run_command(command.clone(), ctx)
                    }
                    CommandSearchItemAction::AcceptWorkflow(AcceptedWorkflow::Local {
                        workflow,
                        ..
                    }) => workspace.use_workflow(workflow.as_workflow(), None, false, ctx),
                },
                CommandSearchEvent::Close { .. } => {
                    workspace.command_search = None;
                    workspace
                        .active_tab_pane_group()
                        .update(ctx, |group, ctx| group.focus(ctx));
                    ctx.notify();
                }
                CommandSearchEvent::Blur => {
                    workspace.command_search = None;
                    ctx.notify();
                }
                CommandSearchEvent::Resize => ctx.notify(),
            }
        });
        search.update(ctx, |search, ctx| {
            search.reset_state(
                session,
                directory,
                query,
                options.filter,
                MenuPositioning::AboveInputBox,
                ctx,
            )
        });
        ctx.focus(&search);
        self.command_search = Some(search);
        ctx.notify();
    }

    fn terminal_for_input(&mut self, ctx: &mut ViewContext<Self>) -> ViewHandle<TerminalView> {
        if let Some(terminal) = self
            .active_tab_pane_group()
            .as_ref(ctx)
            .active_session_view(ctx)
        {
            return terminal;
        }
        let directory = ActiveSession::as_ref(ctx)
            .path_if_local(ctx.window_id())
            .map(PathBuf::from);
        self.add_tab_with_pane_layout(
            PanesLayout::SingleTerminal(Box::new(
                NewTerminalOptions::default().with_initial_directory_opt(directory),
            )),
            Arc::new(HashMap::new()),
            None,
            ctx,
        );
        self.active_tab_pane_group()
            .as_ref(ctx)
            .active_session_view(ctx)
            .expect("new terminal tab has a terminal")
    }

    fn close_tab(&mut self, index: usize, keep_for_undo: bool, ctx: &mut ViewContext<Self>) {
        if index >= self.tabs.len() {
            return;
        }
        let tab = self.tabs.remove(index);
        ctx.unsubscribe_to_view(&tab.pane_group);
        self.global_search_views.remove(&tab.pane_group.id());
        if keep_for_undo {
            tab.pane_group.update(ctx, |group, ctx| {
                group.detach_panes_for_close(&self.working_directories, ctx)
            });
            let workspace = ctx.handle();
            UndoCloseStack::handle(ctx).update(ctx, |stack, ctx| {
                stack.handle_tab_closed(workspace, index, tab, ctx)
            });
        } else {
            self.working_directories.update(ctx, |directories, ctx| {
                directories.remove_pane_group(tab.pane_group.id(), ctx)
            });
            tab.pane_group
                .update(ctx, |group, ctx| group.clean_up_panes(ctx));
        }
        if index < self.active_tab_index {
            self.active_tab_index -= 1;
        }
        if self.tabs.is_empty() {
            self.add_terminal_tab(false, ctx);
        }
        self.activate_tab(self.active_tab_index.min(self.tabs.len() - 1), ctx);
    }

    fn run_command(&mut self, command: String, ctx: &mut ViewContext<Self>) {
        let view = self.terminal_for_input(ctx);
        view.update(ctx, |terminal, ctx| {
            terminal
                .input()
                .update(ctx, |input, ctx| input.set_pending_command(&command, ctx))
        });
    }

    fn use_workflow(
        &mut self,
        workflow: &Workflow,
        overrides: Option<&HashMap<String, String>>,
        execute: bool,
        ctx: &mut ViewContext<Self>,
    ) {
        let data = if let Some(overrides) = overrides {
            compute_workflow_display_data_with_overrides(workflow, overrides.clone())
        } else {
            compute_workflow_display_data(workflow)
        };
        let missing = workflow
            .arguments
            .iter()
            .enumerate()
            .find(|(index, argument)| {
                argument.default_value.is_none()
                    && overrides.is_none_or(|overrides| !overrides.contains_key(&argument.name))
                    && data
                        .argument_index_to_highlight_index_map
                        .contains_key(&(*index).into())
            });
        if execute && missing.is_none() {
            self.run_command(data.command_with_replaced_arguments, ctx);
        } else {
            self.insert_in_input(&data.command_with_replaced_arguments, true, ctx);
            if let Some((index, _)) = missing {
                let ranges = data.argument_index_to_highlight_index_map[&index.into()]
                    .iter()
                    .map(|index| data.replaced_ranges[*index].clone())
                    .collect::<Vec<_>>();
                let terminal = self.terminal_for_input(ctx);
                let editor = terminal.as_ref(ctx).input().as_ref(ctx).editor().clone();
                editor.update(ctx, |editor, ctx| {
                    editor.select_ranges_by_byte_offset(ranges, ctx)
                });
            }
        }
    }

    fn insert_in_input(&mut self, content: &str, replace: bool, ctx: &mut ViewContext<Self>) {
        let view = self.terminal_for_input(ctx);
        view.update(ctx, |terminal, ctx| {
            terminal.input().update(ctx, |input, ctx| {
                if replace {
                    input.replace_buffer_content(content, ctx);
                } else {
                    input.append_to_buffer(content, ctx);
                }
                input.focus_input_box(ctx);
            })
        });
    }

    fn open_settings(
        &mut self,
        section: SettingsSection,
        query: Option<&str>,
        ctx: &mut ViewContext<Self>,
    ) {
        let existing = SettingsPaneManager::as_ref(ctx).find_pane(ctx.window_id());
        if let Some(locator) = existing {
            let view = SettingsPaneManager::as_ref(ctx).settings_view(ctx.window_id());
            view.update(ctx, |view, ctx| {
                view.set_and_refresh_current_page(section, ctx);
                if let Some(query) = query {
                    view.set_search_query(query, ctx);
                }
            });
            self.focus_pane(locator, ctx);
            return;
        }
        let pane = crate::pane_group::SettingsPane::new(section, query, ctx.window_id(), ctx);
        self.add_tab_for_pane(Box::new(pane), ctx);
    }

    #[cfg(feature = "local_fs")]
    pub fn open_file_with_target(
        &mut self,
        path: PathBuf,
        target: FileTarget,
        line_col: Option<warp_util::path::LineAndColumnArg>,
        source: CodeSource,
        ctx: &mut ViewContext<Self>,
    ) {
        match target {
            FileTarget::ExternalEditor(editor) => {
                crate::util::file::open_file_path_with_editor(line_col, path, Some(editor), ctx)
            }
            FileTarget::EnvEditor => {
                crate::util::file::open_file_path_with_editor(line_col, path, None, ctx)
            }
            FileTarget::SystemDefault | FileTarget::SystemGeneric => {
                ctx.open_file_path_in_explorer(&path)
            }
            FileTarget::MarkdownViewer(layout) | FileTarget::CodeEditor(layout) => {
                let pane = CodePane::new(source, line_col, ctx);
                match layout {
                    EditorLayout::NewTab => self.add_tab_for_pane(Box::new(pane), ctx),
                    EditorLayout::SplitPane => {
                        self.active_tab_pane_group().update(ctx, |group, ctx| {
                            group.add_pane_with_direction(Direction::Right, pane, true, ctx)
                        })
                    }
                }
            }
        }
    }

    #[cfg(not(feature = "local_fs"))]
    pub fn open_file_with_target(
        &mut self,
        _path: PathBuf,
        _target: FileTarget,
        _line_col: Option<warp_util::path::LineAndColumnArg>,
        _source: CodeSource,
        _ctx: &mut ViewContext<Self>,
    ) {
    }

    pub fn open_notebook(
        &mut self,
        source: &NotebookSource,
        ctx: &mut ViewContext<Self>,
        new_pane: bool,
    ) {
        let window_id = ctx.window_id();
        let pane = NotebookManager::handle(ctx).update(ctx, |manager, ctx| {
            manager.create_pane(source, window_id, ctx)
        });
        if new_pane {
            self.active_tab_pane_group().update(ctx, |group, ctx| {
                group.add_pane_with_direction(Direction::Right, pane, true, ctx)
            });
        } else {
            self.add_tab_for_pane(Box::new(pane), ctx);
        }
    }

    pub fn open_workflow_in_pane(
        &mut self,
        source: &WorkflowOpenSource,
        mode: WorkflowViewMode,
        ctx: &mut ViewContext<Self>,
    ) {
        let window_id = ctx.window_id();
        let pane = WorkflowManager::handle(ctx).update(ctx, |manager, ctx| {
            manager.create_pane(source, mode, window_id, ctx)
        });
        self.add_tab_for_pane(Box::new(pane), ctx);
    }

    pub fn snapshot(
        &self,
        window_id: WindowId,
        quake_mode: bool,
        app: &AppContext,
    ) -> WindowSnapshot {
        let tabs = self
            .tabs
            .iter()
            .map(|tab| TabSnapshot {
                custom_title: tab.pane_group.as_ref(app).custom_title(app),
                root: tab.pane_group.as_ref(app).snapshot(app),
                default_directory_color: tab.default_directory_color,
                selected_color: tab.selected_color,
                left_panel: tab.left_panel.clone(),
                group_id: tab.group_id,
                pinned: tab.pinned,
            })
            .collect();
        let tab_groups = self
            .tab_groups
            .values()
            .map(|group| TabGroupSnapshot {
                id: group.id,
                name: group.name.clone(),
                color: group.color,
                collapsed: group.collapsed,
                pinned: group.pinned,
            })
            .collect();
        WindowSnapshot {
            tabs,
            active_tab_index: self.active_tab_index,
            bounds: app.window_bounds(&window_id),
            fullscreen_state: app
                .windows()
                .platform_window(window_id)
                .map(|window| window.fullscreen_state())
                .unwrap_or_default(),
            quake_mode,
            universal_search_width: None,
            voltron_width: None,
            left_panel_open: self.left_panel_open,
            vertical_tabs_panel_open: self.vertical_tabs_panel_open,
            left_panel_width: None,
            tab_groups,
        }
    }

    pub fn set_tab_color(
        &mut self,
        index: usize,
        color: SelectedTabColor,
        ctx: &mut ViewContext<Self>,
    ) {
        if let Some(tab) = self.tabs.get_mut(index) {
            tab.selected_color = color;
            ctx.notify();
        }
    }

    pub fn close_tabs(
        &mut self,
        indices: impl Iterator<Item = usize>,
        force: bool,
        allow_window_close: bool,
        ctx: &mut ViewContext<Self>,
    ) -> bool {
        let mut indices = indices
            .filter(|index| *index < self.tabs.len())
            .collect::<Vec<_>>();
        indices.sort_unstable_by(|a, b| b.cmp(a));
        indices.dedup();
        if indices.is_empty() {
            return true;
        }
        if !force {
            let groups = indices
                .iter()
                .map(|index| self.tabs[*index].pane_group.downgrade())
                .collect();
            let summary = UnsavedStateSummary::for_tabs(groups, ctx);
            if summary.save_unsaved_code_and_should_warn(ctx) {
                let handle = ctx.handle();
                let ids = indices
                    .iter()
                    .map(|index| self.tabs[*index].pane_group.id())
                    .collect::<Vec<_>>();
                if summary
                    .dialog()
                    .on_confirm(move |ctx| {
                        if let Some(workspace) = handle.upgrade(ctx) {
                            workspace.update(ctx, |workspace, ctx| {
                                let indices = workspace
                                    .tabs
                                    .iter()
                                    .enumerate()
                                    .filter_map(|(index, tab)| {
                                        ids.contains(&tab.pane_group.id()).then_some(index)
                                    })
                                    .collect::<Vec<_>>();
                                workspace.close_tabs(
                                    indices.into_iter(),
                                    true,
                                    allow_window_close,
                                    ctx,
                                );
                            });
                        }
                    })
                    .on_cancel(|_| {})
                    .show(ctx)
                {
                    return false;
                }
            }
        }
        if allow_window_close && indices.len() == self.tabs.len() {
            ctx.close_window();
            return true;
        }
        for index in indices {
            self.close_tab(index, true, ctx);
        }
        true
    }

    pub fn restore_closed_tab(&mut self, index: usize, tab: TabData, ctx: &mut ViewContext<Self>) {
        let index = index.min(self.tabs.len());
        self.subscribe_to_pane_group(&tab.pane_group, ctx);
        tab.pane_group
            .update(ctx, |group, ctx| group.reattach_panes(ctx));
        self.tabs.insert(index, tab);
        self.activate_tab(index, ctx);
    }
}

impl Entity for Workspace {
    type Event = ();
}

impl TypedActionView for Workspace {
    type Action = WorkspaceAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            WorkspaceAction::ReopenClosedSession => {
                ctx.dispatch_global_action("app:undo_close", ())
            }
            WorkspaceAction::ShowCommandSearch(options) => self.show_command_search(options, ctx),
            WorkspaceAction::OpenCodeReview => {
                self.open_code_review(self.active_tab_pane_group().clone(), ctx)
            }
            WorkspaceAction::OpenCodeReviewPanel(locator) => {
                if let Some(group) = self.get_pane_group_view(locator.pane_group_id).cloned() {
                    self.focus_pane(*locator, ctx);
                    self.open_code_review(group, ctx);
                }
            }
            WorkspaceAction::OpenRepository { path } => self.open_repository(path.as_deref(), ctx),
            WorkspaceAction::OpenFileInNewTab {
                full_path,
                line_and_column,
            } => self.open_file_with_target(
                full_path.clone(),
                FileTarget::CodeEditor(EditorLayout::NewTab),
                *line_and_column,
                CodeSource::Link {
                    path: full_path.clone(),
                    range_start: *line_and_column,
                    range_end: None,
                },
                ctx,
            ),
            WorkspaceAction::OpenProjectExplorer => {
                self.set_left_panel_view(LeftPanelTargetView::ProjectExplorer);
                self.set_left_panel_open(true, ctx);
            }
            WorkspaceAction::OpenGlobalSearch => {
                self.set_left_panel_view(LeftPanelTargetView::GlobalSearch);
                self.set_left_panel_open(true, ctx);
            }
            WorkspaceAction::ToggleLeftPanel => {
                self.set_left_panel_open(!self.left_panel_open, ctx)
            }
            WorkspaceAction::OpenPalette { mode, query, .. } => {
                self.open_palette(*mode, query.as_deref(), ctx)
            }
            WorkspaceAction::TogglePalette { mode, .. } => {
                if self
                    .command_palette
                    .as_ref()
                    .is_some_and(|palette| palette.as_ref(ctx).is_mode_enabled(*mode, ctx))
                {
                    self.command_palette = None;
                    self.active_tab_pane_group()
                        .update(ctx, |group, ctx| group.focus(ctx));
                    ctx.notify();
                } else {
                    self.open_palette(*mode, None, ctx);
                }
            }
            WorkspaceAction::OpenNotebook(source) => self.open_notebook(source, ctx, false),
            WorkspaceAction::ActivateTab(index) | WorkspaceAction::ActivateTabByNumber(index) => {
                self.activate_tab(*index, ctx);
            }
            WorkspaceAction::ActivatePrevTab | WorkspaceAction::CyclePrevSession => {
                let index = self
                    .active_tab_index
                    .checked_sub(1)
                    .unwrap_or(self.tabs.len() - 1);
                self.activate_tab(index, ctx);
            }
            WorkspaceAction::ActivateNextTab | WorkspaceAction::CycleNextSession => {
                self.activate_tab((self.active_tab_index + 1) % self.tabs.len(), ctx);
            }
            WorkspaceAction::ActivateLastTab => {
                self.activate_tab(self.tabs.len() - 1, ctx);
            }
            WorkspaceAction::MoveTabLeft(index) if *index > 0 && *index < self.tabs.len() => {
                self.tabs.swap(*index, *index - 1);
                self.activate_tab(*index - 1, ctx);
            }
            WorkspaceAction::MoveTabRight(index) if *index + 1 < self.tabs.len() => {
                self.tabs.swap(*index, *index + 1);
                self.activate_tab(*index + 1, ctx);
            }
            WorkspaceAction::CloseTab(index) => {
                self.close_tabs(std::iter::once(*index), false, false, ctx);
            }
            WorkspaceAction::CloseActiveTab => {
                self.close_tabs(std::iter::once(self.active_tab_index), false, false, ctx);
            }
            WorkspaceAction::AddDefaultTab | WorkspaceAction::AddTerminalTab { .. } => {
                self.add_terminal_tab(false, ctx)
            }
            WorkspaceAction::AddTabWithShell { shell, .. } => self.add_tab_with_pane_layout(
                PanesLayout::SingleTerminal(Box::new(NewTerminalOptions {
                    shell: Some(shell.clone()),
                    ..Default::default()
                })),
                Arc::new(HashMap::new()),
                None,
                ctx,
            ),
            WorkspaceAction::AddWindowWithShell { shell } => {
                crate::root_view::open_new_window_get_handles(Some(shell.clone()), ctx);
            }
            WorkspaceAction::ShowSettings => {
                self.open_settings(SettingsSection::default(), None, ctx)
            }
            WorkspaceAction::ShowSettingsPage(section) => self.open_settings(*section, None, ctx),
            WorkspaceAction::ShowSettingsPageWithSearch {
                search_query,
                section,
            } => self.open_settings(
                section.unwrap_or_default(),
                Some(search_query.as_str()),
                ctx,
            ),
            WorkspaceAction::ScrollToSettingsWidget { page, .. } => {
                self.open_settings(*page, None, ctx)
            }
            WorkspaceAction::CopyVersion(version) => ctx
                .clipboard()
                .write(ClipboardContent::plain_text((*version).to_owned())),
            WorkspaceAction::CopyTextToClipboard(text) => ctx
                .clipboard()
                .write(ClipboardContent::plain_text(text.clone())),
            WorkspaceAction::UndoRevertInCodeReviewPane { window_id, view_id } => {
                GlobalCodeReviewModel::handle(ctx).update(ctx, |_, ctx| {
                    ctx.emit(GlobalCodeReviewEvent::DiffReverted {
                        window_id: *window_id,
                        view_id: *view_id,
                    });
                });
            }
            WorkspaceAction::OpenInExplorer { path } => ctx.open_file_path_in_explorer(path),
            WorkspaceAction::RunCommand(command) => self.run_command(command.clone(), ctx),
            WorkspaceAction::InsertInInput {
                content,
                replace_buffer,
            } => self.insert_in_input(content, *replace_buffer, ctx),
            WorkspaceAction::RunWorkflow {
                workflow,
                argument_override,
                ..
            } => {
                self.use_workflow(
                    workflow.as_workflow(),
                    argument_override.as_ref(),
                    true,
                    ctx,
                );
            }
            WorkspaceAction::FocusPane(locator) => self.focus_pane(*locator, ctx),
            WorkspaceAction::FocusTerminalViewInWorkspace { terminal_view_id } => {
                for tab in &self.tabs {
                    tab.pane_group.update(ctx, |group, ctx| {
                        group.focus_terminal_view(*terminal_view_id, ctx)
                    });
                }
            }
            WorkspaceAction::DisableTerminalInputSync => {
                let window_id = ctx.window_id();
                SyncedInputState::handle(ctx).update(ctx, |state, _| {
                    state.disable_sync_terminal_inputs(window_id)
                });
            }
            WorkspaceAction::ToggleSyncTerminalInputsInTab => {
                let window_id = ctx.window_id();
                let active_tab_id = self.active_tab_pane_group().id();
                let tab_ids = self
                    .tabs
                    .iter()
                    .map(|tab| tab.pane_group.id())
                    .collect::<Vec<_>>();
                let tab_count = tab_ids.len();
                SyncedInputState::handle(ctx).update(ctx, |state, _| {
                    state.toggle_sync_terminal_inputs_in_tab(
                        active_tab_id,
                        tab_ids.into_iter(),
                        tab_count,
                        window_id,
                    )
                });
            }
            WorkspaceAction::ToggleSyncAllTerminalInputsInAllTabs => {
                let window_id = ctx.window_id();
                SyncedInputState::handle(ctx).update(ctx, |state, _| {
                    state.toggle_sync_all_terminal_inputs_in_all_tabs(window_id)
                });
            }
            WorkspaceAction::FileDeleted { .. }
            | WorkspaceAction::FileRenamed { .. }
            | WorkspaceAction::ConfigureKeybindingSettings { .. }
            | WorkspaceAction::OpenSettingsFile
            | WorkspaceAction::ShowThemeChooser(_)
            | WorkspaceAction::ShowThemeChooserForActiveTheme
            | WorkspaceAction::ToggleResourceCenter
            | WorkspaceAction::ToggleRecordingMode
            | WorkspaceAction::ToggleInBandGenerators
            | WorkspaceAction::ToggleShowMemoryStats
            | WorkspaceAction::ToggleVerticalTabsPanel
            | WorkspaceAction::OpenVerticalTabsPanel
            | WorkspaceAction::MoveTabLeft(_)
            | WorkspaceAction::MoveTabRight(_) => {}
        }
    }
}

impl View for Workspace {
    fn ui_name() -> &'static str {
        "Workspace"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let mut tabs = Flex::row().with_spacing(4.);
        for (index, tab) in self.tabs.iter().enumerate() {
            let title = tab.pane_group.as_ref(app).display_title(app);
            tabs.add_child(
                EventHandler::new(
                    Container::new(
                        Text::new(
                            title,
                            appearance.ui_font_family(),
                            appearance.ui_font_size(),
                        )
                        .with_color(appearance.theme().foreground().into_solid())
                        .finish(),
                    )
                    .with_background(if index == self.active_tab_index {
                        appearance.theme().foreground().with_opacity(20)
                    } else {
                        appearance.theme().background()
                    })
                    .with_uniform_padding(8.)
                    .finish(),
                )
                .on_left_mouse_up(move |ctx, _, _| {
                    ctx.dispatch_typed_action(WorkspaceAction::ActivateTab(index));
                    warpui::elements::DispatchEventResult::StopPropagation
                })
                .finish(),
            );
        }
        let mut body = Flex::row().with_cross_axis_alignment(CrossAxisAlignment::Stretch);
        if self.left_panel_open {
            let group_id = self.active_tab_pane_group().id();
            let panel = match self.left_panel_view() {
                LeftPanelTargetView::ProjectExplorer => self
                    .working_directories
                    .as_ref(app)
                    .get_file_tree_view(group_id)
                    .map(|tree| ChildView::new(&tree).finish()),
                LeftPanelTargetView::GlobalSearch => self
                    .global_search_views
                    .get(&group_id)
                    .map(|search| ChildView::new(search).finish()),
            };
            if let Some(panel) = panel {
                body.add_child(
                    ConstrainedBox::new(panel)
                        .with_width(
                            self.tabs[self.active_tab_index]
                                .left_panel
                                .as_ref()
                                .map_or(280., |panel| panel.width as f32),
                        )
                        .finish(),
                );
            }
        }
        body.add_child(
            Expanded::new(1., ChildView::new(self.active_tab_pane_group()).finish()).finish(),
        );
        let workspace = Flex::column()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(
                warpui::elements::ConstrainedBox::new(Container::new(tabs.finish()).finish())
                    .with_height(TAB_BAR_HEIGHT)
                    .finish(),
            )
            .with_child(Expanded::new(1., body.finish()).finish())
            .finish();
        let mut stack = Stack::new().with_child(workspace);
        stack.add_child(
            Align::new(
                Container::new(ChildView::new(&self.toasts).finish())
                    .with_uniform_margin(8.)
                    .finish(),
            )
            .top_right()
            .finish(),
        );
        if let Some(palette) = &self.command_palette {
            stack.add_child(ChildView::new(palette).finish());
        }
        if let Some(search) = &self.command_search {
            stack.add_child(
                Align::new(ChildView::new(search).finish())
                    .bottom_left()
                    .finish(),
            );
        }
        stack.finish()
    }

    fn keymap_context(&self, app: &AppContext) -> warpui::keymap::Context {
        // The binding belongs to Workspace. Keep availability here so ancestor fallback
        // and native menu dispatch cannot bypass the terminal's input state.
        let mut context = Self::default_keymap_context();
        let available = self
            .active_tab_pane_group()
            .as_ref(app)
            .active_session_view(app)
            .is_none_or(|terminal| {
                matches!(
                    terminal.as_ref(app).model.lock().terminal_input_state(),
                    TerminalInputState::InputEditor | TerminalInputState::NotBootstrapped
                )
            });
        if available {
            context.set.insert(super::HISTORY_SEARCH_AVAILABLE_KEY);
        }
        context
    }

    fn on_focus(&mut self, focus: &FocusContext, ctx: &mut ViewContext<Self>) {
        if focus.is_self_focused() {
            self.active_tab_pane_group()
                .update(ctx, |group, ctx| group.focus(ctx));
        }
    }
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
