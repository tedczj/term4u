pub(crate) mod left_panel;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use warpui::clipboard::ClipboardContent;
use warpui::elements::{ChildView, Container, Element, EventHandler, Flex, ParentElement, Shrinkable, Text};
use warpui::{
    AppContext, Entity, EntityId, FocusContext, SingletonEntity, TypedActionView, View, ViewContext,
    ViewHandle, WindowId,
};

use super::sync_inputs::SyncedInputState;
use super::tab_group::{TabGroup, TabGroupId};
use super::{ActiveSession, PaneViewLocator, WorkspaceAction, WorkspaceRegistry};
use crate::app_state::{
    AppState, LeftPanelSnapshot, PaneUuid, TabGroupSnapshot, TabSnapshot, WindowSnapshot,
};
use crate::appearance::Appearance;
use crate::code::buffer_location::LocalOrRemotePath;
use crate::code::editor_management::CodeSource;
use crate::notebooks::manager::{NotebookManager, NotebookSource};
use crate::pane_group::pane::code_pane::CodePane;
use crate::pane_group::{
    Direction, Event as PaneGroupEvent, NewTerminalOptions, PaneGroup, PanesLayout,
};
use crate::root_view::NewWorkspaceSource;
use crate::settings_view::{SettingsSection, SettingsView};
use crate::tab::{SelectedTabColor, TabData};
use crate::terminal::available_shells::AvailableShell;
use crate::terminal::model::SerializedBlockListItem;
use crate::terminal::session_settings::NewSessionSource;
use crate::themes::theme::AnsiColorIdentifier;
use crate::util::openable_file_type::{EditorLayout, FileTarget};
use crate::workflows::manager::{WorkflowManager, WorkflowOpenSource};
use crate::workflows::{WorkflowSelectionSource, WorkflowSource, WorkflowType, WorkflowViewMode};
use crate::{GlobalResourceHandles, send_telemetry_from_ctx};

pub const WORKSPACE_PADDING: f32 = 8.;
pub const TAB_BAR_HEIGHT: f32 = 36.;
pub const TOTAL_TAB_BAR_HEIGHT: f32 = TAB_BAR_HEIGHT + crate::tab::TAB_BAR_BORDER_HEIGHT;
pub const PANEL_HEADER_HEIGHT: f32 = 36.;
pub const NEW_TAB_BUTTON_POSITION_ID: &str = "new_tab_button";
pub const NEW_SESSION_MENU_BUTTON_POSITION_ID: &str = "new_session_menu_button";
pub const TOGGLE_RIGHT_PANEL_BINDING_NAME: &str = "workspace:toggle_right_panel";

#[derive(Clone, Copy, Debug)]
pub enum OpenDialogSource {
    CloseTab { tab_index: usize },
}

pub struct Workspace {
    resources: GlobalResourceHandles,
    tabs: Vec<TabData>,
    active_tab_index: usize,
    tab_groups: HashMap<TabGroupId, TabGroup>,
    left_panel_open: bool,
    vertical_tabs_panel_open: bool,
    tab_drag_preview: bool,
}

impl Workspace {
    pub fn new(
        resources: GlobalResourceHandles,
        source: NewWorkspaceSource,
        ctx: &mut ViewContext<Self>,
    ) -> Self {
        let mut workspace = Self {
            resources,
            tabs: Vec::new(),
            active_tab_index: 0,
            tab_groups: HashMap::new(),
            left_panel_open: false,
            vertical_tabs_panel_open: false,
            tab_drag_preview: false,
        };
        workspace.restore_source(source, ctx);
        if workspace.tabs.is_empty() {
            workspace.add_terminal_tab(false, ctx);
        }
        WorkspaceRegistry::handle(ctx).update(ctx, |registry, _| {
            registry.register(ctx.window_id(), ctx.handle().downgrade());
        });
        workspace
    }

    #[cfg(any(test, feature = "integration_tests"))]
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
                self.active_tab_index = window_snapshot.active_tab_index;
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
                    }
                }
                self.active_tab_index = self.active_tab_index.min(self.tabs.len().saturating_sub(1));
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
                self.add_tab_with_pane_layout(PanesLayout::SingleTerminal(options), Arc::new(HashMap::new()), None, ctx);
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
            NewWorkspaceSource::TransferredTab { is_tab_drag_preview, .. } => {
                self.tab_drag_preview = is_tab_drag_preview;
            }
        }
    }

    fn subscribe_to_pane_group(&self, pane_group: &ViewHandle<PaneGroup>, ctx: &mut ViewContext<Self>) {
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
            PaneGroupEvent::Exited { .. } => {
                if let Some(index) = self.tabs.iter().position(|tab| tab.pane_group == pane_group) {
                    self.close_tab(index, ctx);
                }
            }
            PaneGroupEvent::FocusPaneInWorkspace { locator } => self.focus_pane(*locator, ctx),
            PaneGroupEvent::OpenSettings(section) => self.open_settings(*section, None, ctx),
            PaneGroupEvent::OpenFileWithTarget { path, target, line_col } => self.open_file_with_target(
                path.clone(), target.clone(), *line_col,
                CodeSource::Link { path: path.clone(), range_start: *line_col, range_end: None }, ctx,
            ),
            PaneGroupEvent::OpenCodeInWarp { source, layout, line_col } => {
                if let Some(path) = source.path() {
                    self.open_file_with_target(path, FileTarget::CodeEditor(*layout), *line_col, source.clone(), ctx);
                }
            }
            PaneGroupEvent::RunWorkflow { workflow, argument_override, .. } => {
                let mut command = workflow.as_workflow().content().to_owned();
                if let Some(values) = argument_override {
                    for (name, value) in values {
                        command = command.replace(&format!("{{{{{name}}}}}"), value);
                    }
                }
                self.run_command(command, ctx);
            }
            PaneGroupEvent::CDToDirectory { path } => self.run_command(format!("cd {}", shell_escape::escape(path.to_string_lossy())), ctx),
            PaneGroupEvent::OpenDirectoryInNewTab { path } => self.add_tab_with_pane_layout(
                PanesLayout::SingleTerminal(Box::new(NewTerminalOptions::default().with_initial_directory(path))),
                Arc::new(HashMap::new()), None, ctx,
            ),
            PaneGroupEvent::AppStateChanged
            | PaneGroupEvent::ExecuteCommand(_)
            | PaneGroupEvent::PaneTitleUpdated
            | PaneGroupEvent::SyncInput(_)
            | PaneGroupEvent::ShowCommandSearch(_)
            | PaneGroupEvent::TerminalViewStateChanged
            | PaneGroupEvent::OpenWorkflowModalWithCommand(_)
            | PaneGroupEvent::OpenWorkflowModalWithTemporary(_)
            | PaneGroupEvent::OpenFileInWarp { .. }
            | PaneGroupEvent::PreviewCodeInWarp { .. }
            | PaneGroupEvent::OpenCodeReviewPane(_)
            | PaneGroupEvent::ToggleCodeReviewPane(_)
            | PaneGroupEvent::MaximizePaneToggled
            | PaneGroupEvent::ActiveSessionChanged
            | PaneGroupEvent::FocusPaneGroup
            | PaneGroupEvent::FocusPane { .. }
            | PaneGroupEvent::PaneFocused
            | PaneGroupEvent::DroppedOnTabBar { .. }
            | PaneGroupEvent::SwitchTabFocusAndMovePane { .. }
            | PaneGroupEvent::UpdateHoveredTabIndex { .. }
            | PaneGroupEvent::ClearHoveredTabIndex
            | PaneGroupEvent::OpenPalette { .. }
            | PaneGroupEvent::ShowToast { .. }
            | PaneGroupEvent::OpenThemeChooser
            | PaneGroupEvent::OpenFilesPalette { .. }
            | PaneGroupEvent::ToggleLeftPanel { .. }
            | PaneGroupEvent::FileRenamed { .. }
            | PaneGroupEvent::FileDeleted { .. }
            | PaneGroupEvent::RepoChanged
            | PaneGroupEvent::InsertCodeReviewComments { .. }
            | PaneGroupEvent::OpenCodeReviewPaneAndScrollToComment { .. }
            | PaneGroupEvent::ImportAllCodeReviewComments { .. }
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
                self.resources.user_default_shell_unsupported_banner_model_handle.clone(),
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
        self.active_tab_index = self.tabs.len() - 1;
        ctx.notify();
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
                self.resources.user_default_shell_unsupported_banner_model_handle.clone(),
                self.resources.model_event_sender.clone(),
                ctx,
            )
        });
        self.subscribe_to_pane_group(&pane_group, ctx);
        self.tabs.push(TabData::new(pane_group));
        self.active_tab_index = self.tabs.len() - 1;
        ctx.notify();
    }

    pub fn add_terminal_tab(&mut self, hide_homepage: bool, ctx: &mut ViewContext<Self>) {
        self.add_tab_with_pane_layout(
            PanesLayout::SingleTerminal(Box::new(NewTerminalOptions { hide_homepage, ..Default::default() })),
            Arc::new(HashMap::new()), None, ctx,
        );
    }

    pub fn tab_count(&self) -> usize { self.tabs.len() }
    pub fn active_tab_index(&self) -> usize { self.active_tab_index }
    pub fn tab_views(&self) -> impl Iterator<Item = &ViewHandle<PaneGroup>> { self.tabs.iter().map(|tab| &tab.pane_group) }
    pub fn active_tab_pane_group(&self) -> &ViewHandle<PaneGroup> { &self.tabs[self.active_tab_index] .pane_group }
    pub fn get_pane_group_view(&self, id: EntityId) -> Option<&ViewHandle<PaneGroup>> { self.tabs.iter().map(|tab| &tab.pane_group).find(|group| group.id() == id) }
    pub fn is_tab_drag_preview(&self) -> bool { self.tab_drag_preview }
    pub fn window_id(&self, ctx: &AppContext) -> WindowId { ctx.window_ids().find(|id| self.tabs.iter().any(|tab| tab.pane_group.window_id(ctx) == *id)).unwrap_or_else(|| self.active_tab_pane_group().window_id(ctx)) }

    pub fn focus_pane(&mut self, locator: PaneViewLocator, ctx: &mut ViewContext<Self>) {
        if let Some(index) = self.tabs.iter().position(|tab| tab.pane_group.id() == locator.pane_group_id) {
            self.active_tab_index = index;
            self.tabs[index].pane_group.update(ctx, |group, ctx| group.reveal_and_focus_pane(locator.pane_id, ctx));
            ctx.notify();
        }
    }

    pub fn workspace_sessions(&self, window_id: WindowId, app: &AppContext) -> Vec<crate::session_management::SessionNavigationData> {
        self.tabs.iter().flat_map(|tab| tab.pane_group.as_ref(app).pane_sessions(tab.pane_group.id(), window_id, app)).collect()
    }

    fn close_tab(&mut self, index: usize, ctx: &mut ViewContext<Self>) {
        if index >= self.tabs.len() { return; }
        let tab = self.tabs.remove(index);
        tab.pane_group.update(ctx, |group, ctx| group.clean_up_panes(ctx));
        if self.tabs.is_empty() { self.add_terminal_tab(false, ctx); }
        self.active_tab_index = self.active_tab_index.min(self.tabs.len() - 1);
        ctx.notify();
    }

    fn run_command(&mut self, command: String, ctx: &mut ViewContext<Self>) {
        if let Some(view) = self.active_tab_pane_group().as_ref(ctx).focused_session_view(ctx) {
            view.update(ctx, |terminal, ctx| terminal.input().update(ctx, |input, ctx| input.set_pending_command(&command, ctx)));
        }
    }

    fn insert_in_input(&mut self, content: &str, replace: bool, ctx: &mut ViewContext<Self>) {
        if let Some(view) = self.active_tab_pane_group().as_ref(ctx).focused_session_view(ctx) {
            view.update(ctx, |terminal, ctx| terminal.input().update(ctx, |input, ctx| {
                if replace { input.replace_buffer_content(content, ctx); } else { input.append_to_buffer(content, ctx); }
                input.focus_input_box(ctx);
            }));
        }
    }

    fn open_settings(&mut self, section: SettingsSection, query: Option<&str>, ctx: &mut ViewContext<Self>) {
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
            FileTarget::ExternalEditor(editor) => crate::util::file::open_file_path_with_editor(line_col, path, Some(editor), ctx),
            FileTarget::EnvEditor => crate::util::file::open_file_path_with_editor(line_col, path, None, ctx),
            FileTarget::SystemDefault | FileTarget::SystemGeneric => ctx.open_path(&path),
            FileTarget::MarkdownViewer(layout) | FileTarget::CodeEditor(layout) => {
                let pane = CodePane::new(source, line_col, ctx);
                match layout {
                    EditorLayout::NewTab => self.add_tab_for_pane(Box::new(pane), ctx),
                    EditorLayout::SplitPane => self.active_tab_pane_group().update(ctx, |group, ctx| group.add_pane_with_direction(Direction::Right, pane, true, ctx)),
                }
            }
        }
    }

    #[cfg(not(feature = "local_fs"))]
    pub fn open_file_with_target(&mut self, _path: PathBuf, _target: FileTarget, _line_col: Option<warp_util::path::LineAndColumnArg>, _source: CodeSource, _ctx: &mut ViewContext<Self>) {}

    pub fn open_notebook(&mut self, source: &NotebookSource, ctx: &mut ViewContext<Self>, new_pane: bool) {
        let pane = NotebookManager::handle(ctx).update(ctx, |manager, ctx| manager.create_pane(source, ctx.window_id(), ctx));
        if new_pane {
            self.active_tab_pane_group().update(ctx, |group, ctx| group.add_pane_with_direction(Direction::Right, pane, true, ctx));
        } else {
            self.add_tab_for_pane(Box::new(pane), ctx);
        }
    }

    pub fn open_workflow_in_pane(&mut self, source: &WorkflowOpenSource, mode: WorkflowViewMode, ctx: &mut ViewContext<Self>) {
        let pane = WorkflowManager::handle(ctx).update(ctx, |manager, ctx| manager.create_pane(source, mode, ctx.window_id(), ctx));
        self.add_tab_for_pane(Box::new(pane), ctx);
    }

    pub fn snapshot(&self, window_id: WindowId, quake_mode: bool, app: &AppContext) -> WindowSnapshot {
        let tabs = self.tabs.iter().map(|tab| TabSnapshot {
            custom_title: tab.pane_group.as_ref(app).custom_title(app),
            root: tab.pane_group.as_ref(app).snapshot(app),
            default_directory_color: tab.default_directory_color,
            selected_color: tab.selected_color,
            left_panel: None::<LeftPanelSnapshot>,
            group_id: tab.group_id,
            pinned: tab.pinned,
        }).collect();
        let tab_groups = self.tab_groups.values().map(|group| TabGroupSnapshot {
            id: group.id, name: group.name.clone(), color: group.color, collapsed: group.collapsed, pinned: group.pinned,
        }).collect();
        WindowSnapshot {
            tabs,
            active_tab_index: self.active_tab_index,
            bounds: app.window_bounds(&window_id),
            fullscreen_state: app.windows().platform_window(window_id).map(|window| window.fullscreen_state()).unwrap_or_default(),
            quake_mode,
            universal_search_width: None,
            voltron_width: None,
            left_panel_open: self.left_panel_open,
            vertical_tabs_panel_open: self.vertical_tabs_panel_open,
            left_panel_width: None,
            tab_groups,
        }
    }

    pub fn set_tab_color(&mut self, index: usize, color: SelectedTabColor, ctx: &mut ViewContext<Self>) {
        if let Some(tab) = self.tabs.get_mut(index) { tab.selected_color = color; ctx.notify(); }
    }

    pub fn close_tabs(
        &mut self,
        indices: impl Iterator<Item = usize>,
        _source: OpenDialogSource,
        _force: bool,
        _allow_window_close: bool,
        ctx: &mut ViewContext<Self>,
    ) -> bool {
        let mut indices = indices.collect::<Vec<_>>();
        indices.sort_unstable_by(|a, b| b.cmp(a));
        for index in indices { self.close_tab(index, ctx); }
        true
    }

    pub fn restore_closed_tab(&mut self, _index: usize, tab: TabData, ctx: &mut ViewContext<Self>) {
        self.subscribe_to_pane_group(&tab.pane_group, ctx);
        self.tabs.push(tab);
        self.active_tab_index = self.tabs.len() - 1;
        ctx.notify();
    }
}

impl Entity for Workspace { type Event = (); }

impl TypedActionView for Workspace {
    type Action = WorkspaceAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            WorkspaceAction::ActivateTab(index) | WorkspaceAction::ActivateTabByNumber(index) => {
                if *index < self.tabs.len() { self.active_tab_index = *index; ctx.notify(); }
            }
            WorkspaceAction::ActivatePrevTab | WorkspaceAction::CyclePrevSession => {
                self.active_tab_index = self.active_tab_index.checked_sub(1).unwrap_or(self.tabs.len() - 1); ctx.notify();
            }
            WorkspaceAction::ActivateNextTab | WorkspaceAction::CycleNextSession => {
                self.active_tab_index = (self.active_tab_index + 1) % self.tabs.len(); ctx.notify();
            }
            WorkspaceAction::ActivateLastTab => { self.active_tab_index = self.tabs.len() - 1; ctx.notify(); }
            WorkspaceAction::MoveTabLeft(index) if *index > 0 && *index < self.tabs.len() => { self.tabs.swap(*index, *index - 1); self.active_tab_index = *index - 1; ctx.notify(); }
            WorkspaceAction::MoveTabRight(index) if *index + 1 < self.tabs.len() => { self.tabs.swap(*index, *index + 1); self.active_tab_index = *index + 1; ctx.notify(); }
            WorkspaceAction::CloseTab(index) => self.close_tab(*index, ctx),
            WorkspaceAction::CloseActiveTab => self.close_tab(self.active_tab_index, ctx),
            WorkspaceAction::AddDefaultTab | WorkspaceAction::AddTerminalTab { .. } => self.add_terminal_tab(false, ctx),
            WorkspaceAction::AddTabWithShell { shell, .. } => self.add_tab_with_pane_layout(PanesLayout::SingleTerminal(Box::new(NewTerminalOptions { shell: Some(shell.clone()), ..Default::default() })), Arc::new(HashMap::new()), None, ctx),
            WorkspaceAction::AddWindowWithShell { shell } => ctx.dispatch_global_action("root_view:open_new_with_shell", &Some(shell.clone())),
            WorkspaceAction::ShowSettings => self.open_settings(SettingsSection::default(), None, ctx),
            WorkspaceAction::ShowSettingsPage(section) => self.open_settings(*section, None, ctx),
            WorkspaceAction::ShowSettingsPageWithSearch { search_query, section } => self.open_settings(section.unwrap_or_default(), Some(search_query), ctx),
            WorkspaceAction::ScrollToSettingsWidget { page, .. } => self.open_settings(*page, None, ctx),
            WorkspaceAction::CopyVersion(version) => ctx.clipboard().write(ClipboardContent::plain_text(*version)),
            WorkspaceAction::CopyTextToClipboard(text) => ctx.clipboard().write(ClipboardContent::plain_text(text.clone())),
            WorkspaceAction::SendFeedback => ctx.dispatch_global_action("root_view:send_feedback", &()),
            WorkspaceAction::OpenInExplorer { path } => ctx.open_path(path),
            WorkspaceAction::RunCommand(command) => self.run_command(command.clone(), ctx),
            WorkspaceAction::InsertInInput { content, replace_buffer } => self.insert_in_input(content, *replace_buffer, ctx),
            WorkspaceAction::RunWorkflow { workflow, argument_override, .. } => {
                let mut command = workflow.as_workflow().content().to_owned();
                if let Some(values) = argument_override { for (name, value) in values { command = command.replace(&format!("{{{{{name}}}}}"), value); } }
                self.run_command(command, ctx);
            }
            WorkspaceAction::FocusPane(locator) => self.focus_pane(*locator, ctx),
            WorkspaceAction::FocusTerminalViewInWorkspace { terminal_view_id } => {
                for tab in &self.tabs { tab.pane_group.update(ctx, |group, ctx| group.focus_terminal_view(*terminal_view_id, ctx)); }
            }
            WorkspaceAction::DisableTerminalInputSync => SyncedInputState::handle(ctx).update(ctx, |state, _| state.disable_sync_terminal_inputs(ctx.window_id())),
            WorkspaceAction::ToggleSyncTerminalInputsInTab => SyncedInputState::handle(ctx).update(ctx, |state, _| state.toggle_sync_terminal_inputs_in_tab(ctx.window_id(), self.active_tab_pane_group().id())),
            WorkspaceAction::ToggleSyncAllTerminalInputsInAllTabs => SyncedInputState::handle(ctx).update(ctx, |state, _| state.toggle_sync_all_terminal_inputs_in_all_tabs(ctx.window_id())),
            WorkspaceAction::FileDeleted { .. }
            | WorkspaceAction::FileRenamed { .. }
            | WorkspaceAction::ConfigureKeybindingSettings { .. }
            | WorkspaceAction::OpenSettingsFile
            | WorkspaceAction::ShowThemeChooser(_)
            | WorkspaceAction::ShowThemeChooserForActiveTheme
            | WorkspaceAction::OpenPalette { .. }
            | WorkspaceAction::TogglePalette { .. }
            | WorkspaceAction::ShowCommandSearch(_)
            | WorkspaceAction::ToggleResourceCenter
            | WorkspaceAction::OpenRepository { .. }
            | WorkspaceAction::OpenPromptEditor { .. }
            | WorkspaceAction::ReopenClosedSession
            | WorkspaceAction::ToggleRecordingMode
            | WorkspaceAction::ToggleInBandGenerators
            | WorkspaceAction::ToggleDebugNetworkStatus
            | WorkspaceAction::ToggleShowMemoryStats
            | WorkspaceAction::OpenProjectExplorer
            | WorkspaceAction::OpenGlobalSearch
            | WorkspaceAction::ToggleLeftPanel
            | WorkspaceAction::ToggleVerticalTabsPanel
            | WorkspaceAction::OpenVerticalTabsPanel
            | WorkspaceAction::OpenCodeReviewPanel(_)
            | WorkspaceAction::OpenFileInNewTab { .. }
            | WorkspaceAction::UndoRevertInCodeReviewPane { .. }
            | WorkspaceAction::MoveTabLeft(_)
            | WorkspaceAction::MoveTabRight(_) => {}
        }
    }
}

impl View for Workspace {
    fn ui_name() -> &'static str { "Workspace" }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let mut tabs = Flex::row().with_spacing(4.);
        for (index, tab) in self.tabs.iter().enumerate() {
            let title = tab.pane_group.as_ref(app).display_title(app);
            tabs.add_child(
                EventHandler::new(Container::new(Text::new(title, appearance.ui_font_family(), appearance.ui_font_size()).finish()).with_uniform_padding(8.).finish())
                    .on_left_mouse_up(move |ctx, _, _| { ctx.dispatch_typed_action(WorkspaceAction::ActivateTab(index)); warpui::elements::DispatchEventResult::StopPropagation })
                    .finish(),
            );
        }
        Flex::column()
            .child(Container::new(tabs.finish()).with_height(TAB_BAR_HEIGHT).finish())
            .child(Shrinkable::new(1., ChildView::new(self.active_tab_pane_group()).finish()).finish())
            .finish()
    }

    fn on_focus(&mut self, focus: &FocusContext, ctx: &mut ViewContext<Self>) {
        if focus.is_self_focused() { self.active_tab_pane_group().update(ctx, |group, ctx| group.focus(ctx)); }
    }
}
