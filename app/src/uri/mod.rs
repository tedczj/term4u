pub mod parse_url_paths;
pub mod web_intent_parser;

use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{Result, anyhow};
use url::Url;
use warpui::{AppContext, SingletonEntity as _, WindowId};

use crate::code::editor_management::CodeSource;
use crate::launch_configs::launch_config::LaunchConfig;
use crate::root_view::{OpenLaunchConfigArg, open_new_window_get_handles};
use crate::server::telemetry::LaunchConfigUiLocation;
use crate::settings_view::{SettingsSection, settings_widget_deeplink_target};
use crate::user_config::load_launch_configs;
use crate::util::openable_file_type::{EditorLayout, FileTarget};
use crate::workspace::{PaneViewLocator, Workspace, WorkspaceAction, WorkspaceRegistry};

pub enum OpenSettingsArgs {
    Default,
    Search { query: String },
    Widget { page: SettingsSection, widget_id: &'static str },
}

#[derive(Debug, PartialEq, Eq)]
pub enum UriHost {
    Action,
    Launch,
    Settings,
    Session,
}

impl FromStr for UriHost {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "action" => Ok(Self::Action),
            "launch" => Ok(Self::Launch),
            "settings" => Ok(Self::Settings),
            "session" => Ok(Self::Session),
            other => Err(anyhow!("Unsupported local URI host: {other}")),
        }
    }
}

impl UriHost {
    fn handle(&self, primary_window_id: Option<WindowId>, url: &Url, ctx: &mut AppContext) {
        match self {
            Self::Action => handle_action(primary_window_id, url, ctx),
            Self::Launch => handle_launch(url, ctx),
            Self::Settings => handle_settings(primary_window_id, url, ctx),
            Self::Session => handle_session(url, ctx),
        }
    }
}

fn handle_action(primary_window_id: Option<WindowId>, url: &Url, ctx: &mut AppContext) {
    let action = url.path_segments().into_iter().flatten().next().unwrap_or("");
    match action {
        "new_tab" => dispatch_workspace_action(primary_window_id, WorkspaceAction::AddDefaultTab, ctx),
        "new_window" => { open_new_window_get_handles(None, ctx); }
        "open_file" => {
            if let Some(path) = url.query_pairs().find_map(|(key, value)| (key == "path").then(|| PathBuf::from(value.into_owned()))) {
                open_file(primary_window_id, path, ctx);
            }
        }
        _ => log::warn!("Unsupported local URI action: {action}"),
    }
}

fn handle_launch(url: &Url, ctx: &mut AppContext) {
    let target = url.path().trim_matches('/');
    let configs = load_launch_configs(&crate::user_config::launch_configs_dir());
    if let Some(config) = configs.iter().find(|config| {
        config.name.as_deref() == Some(target)
            || config.source_path.as_ref().is_some_and(|path| path.file_stem().and_then(|stem| stem.to_str()) == Some(target))
    }) {
        ctx.dispatch_global_action("root_view:open_launch_config", &OpenLaunchConfigArg {
            launch_config: config.clone(),
            ui_location: LaunchConfigUiLocation::Uri,
            open_in_active_window: false,
        });
    } else {
        log::warn!("Local launch configuration not found: {target}");
    }
}

fn handle_settings(primary_window_id: Option<WindowId>, url: &Url, ctx: &mut AppContext) {
    let query = url.query_pairs().find_map(|(key, value)| (key == "q").then(|| value.into_owned()));
    let widget = url.query_pairs().find_map(|(key, value)| (key == "widget").then(|| value.into_owned()));
    let action = if let Some(widget) = widget.and_then(|widget| settings_widget_deeplink_target(&widget)) {
        WorkspaceAction::ScrollToSettingsWidget { page: widget.0, widget_id: widget.1 }
    } else if let Some(query) = query.filter(|query| !query.is_empty()) {
        WorkspaceAction::ShowSettingsPageWithSearch { search_query: query, section: None }
    } else {
        WorkspaceAction::ShowSettings
    };
    dispatch_workspace_action(primary_window_id, action, ctx);
}

fn handle_session(url: &Url, ctx: &mut AppContext) {
    let encoded = url.path_segments().into_iter().flatten().last().unwrap_or("");
    let Ok(uuid) = hex::decode(encoded) else {
        log::warn!("Invalid local session URI");
        return;
    };
    let target = WorkspaceRegistry::as_ref(ctx).all_workspaces(ctx).into_iter().find_map(|(window_id, workspace)| {
        workspace.as_ref(ctx).tab_views().find_map(|group| {
            group.as_ref(ctx).find_terminal_pane_by_session_uuid(&uuid).map(|pane_id| (window_id, PaneViewLocator { pane_group_id: group.id(), pane_id }))
        })
    });
    if let Some((window_id, locator)) = target {
        ctx.windows().show_window_and_focus_app(window_id);
        if let Some(root) = ctx.root_view_id(window_id) {
            ctx.dispatch_action_for_view(window_id, root, "root_view:handle_pane_navigation_event", &locator);
        }
    }
}

fn dispatch_workspace_action(window_id: Option<WindowId>, action: WorkspaceAction, ctx: &mut AppContext) {
    let workspace = window_id
        .and_then(|window_id| WorkspaceRegistry::as_ref(ctx).get(window_id, ctx))
        .or_else(|| ctx.windows().active_window().and_then(|window_id| WorkspaceRegistry::as_ref(ctx).get(window_id, ctx)));
    if let Some(workspace) = workspace {
        workspace.update(ctx, |workspace, ctx| workspace.handle_action(&action, ctx));
    } else {
        let (_, root) = open_new_window_get_handles(None, ctx);
        root.update(ctx, |root, ctx| root.handle_action(&action, ctx));
    }
}

fn open_file(window_id: Option<WindowId>, path: PathBuf, ctx: &mut AppContext) {
    let workspace = window_id
        .and_then(|window_id| WorkspaceRegistry::as_ref(ctx).get(window_id, ctx))
        .or_else(|| ctx.windows().active_window().and_then(|window_id| WorkspaceRegistry::as_ref(ctx).get(window_id, ctx)));
    let Some(workspace) = workspace else { return; };
    let source = CodeSource::Finder { path: path.clone() };
    workspace.update(ctx, |workspace, ctx| {
        workspace.open_file_with_target(path, FileTarget::CodeEditor(EditorLayout::NewTab), None, source, ctx)
    });
}

pub fn handle_incoming_uri(url: &Url, ctx: &mut AppContext) {
    if url.scheme() == "file" {
        if let Ok(path) = url.to_file_path() { open_file(ctx.windows().active_window(), path, ctx); }
        return;
    }
    if url.scheme() != "warp" && url.scheme() != "term4u" {
        log::warn!("Ignoring URI with unsupported scheme");
        return;
    }
    match url.host_str().and_then(|host| UriHost::from_str(host).ok()) {
        Some(host) => host.handle(ctx.windows().active_window(), url, ctx),
        None => log::warn!("Ignoring unsupported URI"),
    }
}
