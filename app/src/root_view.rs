use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use lazy_static::lazy_static;
use parking_lot::Mutex;
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::vector::{Vector2F, vec2f};
use serde::{Deserialize, Serialize};
use warpui::elements::{ChildView, Element};
use warpui::platform::{WindowBounds, WindowStyle};
use warpui::{
    AddWindowOptions, AppContext, DisplayId, Entity, FocusContext, SingletonEntity,
    TypedActionView, View, ViewContext, ViewHandle, WindowId,
};

use crate::app_state::{AppState, PaneUuid, WindowSnapshot};
use crate::launch_configs::launch_config;
use crate::pane_group::{NewTerminalOptions, PanesLayout};
use crate::settings::{QuakeModeSettings, ThemeSettings};
use crate::settings_view::SettingsSection;
use crate::terminal::available_shells::AvailableShell;
use crate::terminal::model::SerializedBlockListItem;
use crate::terminal::shell::ShellType;
use crate::themes::theme::AnsiColorIdentifier;
use crate::uri::OpenSettingsArgs;
use crate::window_settings::WindowSettings;
use crate::workspace::{PaneViewLocator, Workspace, WorkspaceAction, WorkspaceRegistry};
use crate::{GlobalResourceHandles, GlobalResourceHandlesProvider, UpdateQuakeModeEventArg};

const WINDOW_TITLE: &str = "Term4u";

lazy_static! {
    static ref FALLBACK_WINDOW_SIZE: Vector2F = vec2f(800., 600.);
    static ref QUAKE_STATE: Arc<Mutex<Option<QuakeModeState>>> = Arc::new(Mutex::new(None));
}

#[derive(Debug, Clone)]
pub struct QuakeModeState {
    window_id: WindowId,
    active_display_id: DisplayId,
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Deserialize, Serialize, Default, schemars::JsonSchema, settings_value::SettingsValue)]
#[schemars(description = "Screen edge to pin the hotkey window to.", rename_all = "snake_case")]
pub enum QuakeModePinPosition {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
}

pub struct OpenFromRestoredArg {
    pub app_state: Option<AppState>,
}

pub struct OpenLaunchConfigArg {
    pub launch_config: launch_config::LaunchConfig,
    pub ui_location: crate::server::telemetry::LaunchConfigUiLocation,
    pub open_in_active_window: bool,
}

pub struct OpenPath {
    pub path: PathBuf,
}

pub struct SubshellCommandArg {
    pub command: String,
    pub shell_type: Option<ShellType>,
}

#[derive(Clone)]
pub enum NewWorkspaceSource {
    Empty {
        previous_active_window: Option<WindowId>,
        shell: Option<AvailableShell>,
    },
    FromTemplate {
        window_template: launch_config::WindowTemplate,
    },
    Restored {
        window_snapshot: WindowSnapshot,
        block_lists: Arc<HashMap<PaneUuid, Vec<SerializedBlockListItem>>>,
    },
    Session {
        options: Box<NewTerminalOptions>,
    },
    NotebookFromFilePath {
        file_path: Option<PathBuf>,
    },
    TransferredTab {
        source_window_id: WindowId,
        tab_color: Option<AnsiColorIdentifier>,
        custom_title: Option<String>,
        left_panel_open: bool,
        vertical_tabs_panel_open: bool,
        is_tab_drag_preview: bool,
    },
}

impl NewWorkspaceSource {
    pub fn has_horizontal_split(&self) -> bool {
        match self {
            Self::Restored { window_snapshot, .. } => window_snapshot
                .tabs
                .get(window_snapshot.active_tab_index)
                .or_else(|| window_snapshot.tabs.first())
                .is_some_and(|tab| tab.root.has_horizontal_split()),
            Self::Empty { .. }
            | Self::FromTemplate { .. }
            | Self::Session { .. }
            | Self::NotebookFromFilePath { .. }
            | Self::TransferredTab { .. } => false,
        }
    }
}

pub struct RootView {
    workspace: ViewHandle<Workspace>,
    window_id: WindowId,
}

impl RootView {
    pub fn new(
        resources: GlobalResourceHandles,
        source: NewWorkspaceSource,
        ctx: &mut ViewContext<Self>,
    ) -> Self {
        let workspace = ctx.add_typed_action_view(|ctx| Workspace::new(resources, source, ctx));
        Self {
            workspace,
            window_id: ctx.window_id(),
        }
    }

    pub fn workspace_view(&self) -> Option<&ViewHandle<Workspace>> {
        Some(&self.workspace)
    }

    pub fn focus(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.focus(&self.workspace);
    }

    fn close_window(&mut self, _: &(), ctx: &mut ViewContext<Self>) -> bool {
        ctx.close_window();
        true
    }

    fn minimize_window(&mut self, _: &(), ctx: &mut ViewContext<Self>) -> bool {
        ctx.minimize_window();
        true
    }

    fn toggle_maximize_window(&mut self, _: &(), ctx: &mut ViewContext<Self>) -> bool {
        ctx.toggle_maximize_window();
        true
    }

    fn toggle_fullscreen(&mut self, _: &(), ctx: &mut ViewContext<Self>) -> bool {
        ctx.windows().toggle_fullscreen(self.window_id, ctx);
        true
    }

    fn focus_pane(&mut self, locator: &PaneViewLocator, ctx: &mut ViewContext<Self>) -> bool {
        self.workspace.update(ctx, |workspace, ctx| workspace.focus_pane(*locator, ctx));
        true
    }

    fn activate_tab_by_pane_group_id(&mut self, id: &warpui::EntityId, ctx: &mut ViewContext<Self>) -> bool {
        let index = self.workspace.as_ref(ctx).tab_views().position(|group| group.id() == *id);
        if let Some(index) = index {
            self.workspace.update(ctx, |workspace, ctx| workspace.handle_action(&WorkspaceAction::ActivateTab(index), ctx));
        }
        true
    }

    pub fn open_settings_page_in_existing_window(&mut self, section: &SettingsSection, ctx: &mut ViewContext<Self>) -> bool {
        self.workspace.update(ctx, |workspace, ctx| workspace.handle_action(&WorkspaceAction::ShowSettingsPage(*section), ctx));
        true
    }

    pub fn open_settings_in_existing_window(&mut self, args: &OpenSettingsArgs, ctx: &mut ViewContext<Self>) -> bool {
        let action = workspace_action_for_open_settings(args);
        self.workspace.update(ctx, |workspace, ctx| workspace.handle_action(&action, ctx));
        true
    }

    fn add_session_at_path(&mut self, path: &PathBuf, ctx: &mut ViewContext<Self>) -> bool {
        self.workspace.update(ctx, |workspace, ctx| {
            workspace.add_tab_with_pane_layout(
                PanesLayout::SingleTerminal(Box::new(NewTerminalOptions::default().with_initial_directory(path))),
                Arc::new(HashMap::new()),
                None,
                ctx,
            );
        });
        true
    }

    fn insert_subshell_command(&mut self, arg: &SubshellCommandArg, ctx: &mut ViewContext<Self>) {
        self.workspace.update(ctx, |workspace, ctx| {
            workspace.handle_action(&WorkspaceAction::RunCommand(arg.command.clone()), ctx)
        });
    }
}

impl Entity for RootView { type Event = (); }

impl TypedActionView for RootView {
    type Action = WorkspaceAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        self.workspace.update(ctx, |workspace, ctx| workspace.handle_action(action, ctx));
    }
}

impl View for RootView {
    fn ui_name() -> &'static str { "RootView" }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        ChildView::new(&self.workspace).finish()
    }

    fn on_focus(&mut self, focus: &FocusContext, ctx: &mut ViewContext<Self>) {
        if focus.is_self_focused() { self.focus(ctx); }
    }
}

impl Drop for RootView {
    fn drop(&mut self) {}
}

pub fn init(app: &mut AppContext) {
    app.add_global_action("root_view:open_from_restored", open_from_restored);
    app.add_global_action("root_view:open_new", open_new);
    app.add_global_action("root_view:open_new_with_shell", open_new_with_shell);
    app.add_global_action("root_view:open_new_from_path", |path, ctx| { let _ = open_new_from_path(path, ctx); });
    app.add_global_action("root_view:open_launch_config", open_launch_config);
    app.add_global_action("root_view:send_feedback", send_feedback);
    app.add_global_action("root_view:toggle_quake_mode_window", toggle_quake_mode_window);
    app.add_action("root_view:handle_pane_navigation_event", RootView::focus_pane);
    app.add_action("root_view:activate_tab_by_pane_group_id", RootView::activate_tab_by_pane_group_id);
    app.add_action("root_view:add_session_at_path", RootView::add_session_at_path);
    app.add_action("root_view:open_settings_page_in_existing_window", RootView::open_settings_page_in_existing_window);
    app.add_action("root_view:open_settings_in_existing_window", RootView::open_settings_in_existing_window);
    app.add_action("root_view:close_window", RootView::close_window);
    app.add_action("root_view:minimize_window", RootView::minimize_window);
    app.add_action("root_view:toggle_maximize_window", RootView::toggle_maximize_window);
    app.add_action("root_view:toggle_fullscreen", RootView::toggle_fullscreen);
}

fn open_from_restored(arg: &OpenFromRestoredArg, ctx: &mut AppContext) {
    let Some(state) = &arg.app_state else {
        open_new(&(), ctx);
        return;
    };
    if state.windows.is_empty() {
        open_new(&(), ctx);
        return;
    }
    for window in &state.windows {
        open_new_with_workspace_source(
            NewWorkspaceSource::Restored {
                window_snapshot: window.clone(),
                block_lists: state.block_lists.clone(),
            },
            ctx,
        );
    }
}

fn open_launch_config(arg: &OpenLaunchConfigArg, ctx: &mut AppContext) {
    for window_template in arg.launch_config.windows.clone() {
        open_new_with_workspace_source(NewWorkspaceSource::FromTemplate { window_template }, ctx);
    }
}

fn send_feedback(_: &(), ctx: &mut AppContext) {
    ctx.open_url("mailto:feedback@term4u.local");
}

pub(crate) fn open_new_with_workspace_source(
    source: NewWorkspaceSource,
    ctx: &mut AppContext,
) -> (WindowId, ViewHandle<RootView>) {
    let resources = GlobalResourceHandlesProvider::as_ref(ctx).get().clone();
    ctx.add_window(default_window_options(WindowSettings::as_ref(ctx), ctx), |ctx| {
        let mut root = RootView::new(resources, source, ctx);
        root.focus(ctx);
        root
    })
}

pub(crate) fn open_new_from_path(arg: &OpenPath, ctx: &mut AppContext) -> (WindowId, ViewHandle<RootView>) {
    open_new_with_workspace_source(
        NewWorkspaceSource::Session {
            options: Box::new(NewTerminalOptions::default().with_initial_directory_opt(path_if_directory(&arg.path).map(Path::to_path_buf))),
        },
        ctx,
    )
}

pub(crate) fn open_new_window_get_handles(
    shell: Option<AvailableShell>,
    ctx: &mut AppContext,
) -> (WindowId, ViewHandle<RootView>) {
    open_new_with_workspace_source(
        NewWorkspaceSource::Empty { previous_active_window: ctx.windows().active_window(), shell },
        ctx,
    )
}

fn open_new(_: &(), ctx: &mut AppContext) { open_new_window_get_handles(None, ctx); }
fn open_new_with_shell(shell: &Option<AvailableShell>, ctx: &mut AppContext) { open_new_window_get_handles(shell.clone(), ctx); }

fn path_if_directory(path: &Path) -> Option<&Path> { path.is_dir().then_some(path) }

fn workspace_action_for_open_settings(args: &OpenSettingsArgs) -> WorkspaceAction {
    match args {
        OpenSettingsArgs::Default => WorkspaceAction::ShowSettings,
        OpenSettingsArgs::Search { query } => WorkspaceAction::ShowSettingsPageWithSearch { search_query: query.clone(), section: None },
        OpenSettingsArgs::Widget { page, widget_id } => WorkspaceAction::ScrollToSettingsWidget { page: *page, widget_id },
    }
}

fn default_window_options(settings: &WindowSettings, _ctx: &AppContext) -> AddWindowOptions {
    AddWindowOptions {
        window_style: WindowStyle::Normal,
        window_bounds: WindowBounds::new(settings.window_size.value().map(|size| RectF::new(vec2f(100., 100.), vec2f(size.width as f32, size.height as f32)))),
        title: Some(WINDOW_TITLE.to_owned()),
        ..Default::default()
    }
}

pub fn quake_mode_window_is_open() -> bool { QUAKE_STATE.lock().is_some() }
pub fn quake_mode_window_id() -> Option<WindowId> { QUAKE_STATE.lock().as_ref().map(|state| state.window_id) }
pub fn set_quake_mode(state: Option<QuakeModeState>) { *QUAKE_STATE.lock() = state; }

fn toggle_quake_mode_window(_: &GlobalResourceHandles, ctx: &mut AppContext) {
    if let Some(window_id) = quake_mode_window_id() {
        if ctx.is_window_open(window_id) {
            ctx.windows().show_window_and_focus_app(window_id);
            return;
        }
        set_quake_mode(None);
    }
    let (window_id, _) = open_new_window_get_handles(None, ctx);
    let display_id = ctx.windows().active_display().map(|display| display.id()).unwrap_or_default();
    set_quake_mode(Some(QuakeModeState { window_id, active_display_id: display_id }));
}

pub fn update_quake_window_bounds(_settings: &QuakeModeSettings, _ctx: &mut AppContext) {}
