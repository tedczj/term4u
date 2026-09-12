use std::collections::HashMap;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use diesel::prelude::*;
use num_traits::FromPrimitive;
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::vector::vec2f;
use warpui::platform::FullscreenState;
use warpui::windowing::{MIN_WINDOW_HEIGHT, MIN_WINDOW_WIDTH};

use super::{block_list, model, schema};
use crate::app_state::{
    AppState, BranchSnapshot, CodePaneSnapShot, CodePaneTabSnapshot, CodeReviewPaneSnapshot,
    LeafContents, LeafSnapshot, LeftPanelSnapshot, NotebookPaneSnapshot, PaneFlex,
    PaneNodeSnapshot, SettingsPaneSnapshot, SplitDirection, TabGroupSnapshot, TabSnapshot,
    TerminalPaneSnapshot, WindowSnapshot, WorkflowPaneSnapshot,
};
use crate::notebooks::NotebookId;
use crate::settings_view::SettingsSection;
use crate::tab::SelectedTabColor;
use crate::themes::theme::AnsiColorIdentifier;
use crate::workflows::{Workflow, WorkflowId};
use crate::workspace::tab_group::TabGroupId;

pub(super) fn load(connection: &mut SqliteConnection) -> QueryResult<Option<AppState>> {
    let active_window = schema::app::table
        .select(schema::app::active_window_id)
        .first::<Option<i32>>(connection)
        .optional()?
        .flatten();
    let old_windows = schema::windows::table
        .order(schema::windows::id.asc())
        .load::<model::Window>(connection)?;
    let mut windows = Vec::new();
    let mut active_window_index = None;
    for window in old_windows {
        let old_tabs = schema::tabs::table
            .filter(schema::tabs::window_id.eq(window.id))
            .order(schema::tabs::id.asc())
            .load::<model::Tab>(connection)?;
        let old_groups = schema::tab_groups::table
            .filter(schema::tab_groups::window_id.eq(window.id))
            .order(schema::tab_groups::id.asc())
            .load::<model::TabGroup>(connection)?;
        let mut group_ids = HashMap::new();
        let tab_groups = old_groups
            .into_iter()
            .map(|group| {
                let id = TabGroupId::new();
                group_ids.insert(group.id, id);
                TabGroupSnapshot {
                    id,
                    name: group.name,
                    color: decode_color(group.color.as_deref()),
                    collapsed: group.collapsed,
                    pinned: group.pinned,
                }
            })
            .collect();
        let mut tabs = Vec::new();
        let mut active_tab_index = 0;
        for (index, tab) in old_tabs.into_iter().enumerate() {
            let root = schema::pane_nodes::table
                .filter(schema::pane_nodes::tab_id.eq(tab.id))
                .filter(schema::pane_nodes::parent_pane_node_id.is_null())
                .order(schema::pane_nodes::id.asc())
                .first::<model::PaneNode>(connection)
                .optional()?;
            let Some(root) = root else { continue };
            let Some(root) = read_node(connection, root) else {
                continue;
            };
            let panel = schema::panels::table
                .filter(schema::panels::tab_id.eq(tab.id))
                .first::<model::Panel>(connection)
                .optional()?;
            let left_panel = panel
                .and_then(|panel| panel.left_panel)
                .and_then(|json| serde_json::from_str::<LeftPanelSnapshot>(&json).ok());
            if usize::try_from(window.active_tab_index).ok() == Some(index) {
                active_tab_index = tabs.len();
            }
            tabs.push(TabSnapshot {
                root,
                custom_title: tab.custom_title,
                default_directory_color: None,
                selected_color: decode_color(tab.color.as_deref()),
                left_panel,
                group_id: tab.tab_group_id.and_then(|id| group_ids.get(&id).copied()),
                pinned: tab.pinned,
            });
        }
        if tabs.is_empty() {
            continue;
        }
        let left_panel = tabs[active_tab_index].left_panel.as_ref();
        let left_panel_width = left_panel.map(|panel| panel.width as f32);
        let left_panel_open = window.left_panel_open.unwrap_or(left_panel.is_some());
        if active_window == Some(window.id) {
            active_window_index = Some(windows.len());
        }
        let bounds = match (
            window.window_width,
            window.window_height,
            window.origin_x,
            window.origin_y,
        ) {
            (Some(width), Some(height), Some(x), Some(y))
                if width >= MIN_WINDOW_WIDTH
                    && height >= MIN_WINDOW_HEIGHT
                    && width.is_finite()
                    && height.is_finite()
                    && x.is_finite()
                    && y.is_finite() =>
            {
                Some(RectF::new(vec2f(x, y), vec2f(width, height)))
            }
            _ => None,
        };
        windows.push(WindowSnapshot {
            tabs,
            active_tab_index,
            bounds,
            fullscreen_state: FullscreenState::from_i32(window.fullscreen_state)
                .unwrap_or_default(),
            quake_mode: window.quake_mode,
            universal_search_width: window.universal_search_width,
            voltron_width: window.voltron_width,
            left_panel_open,
            vertical_tabs_panel_open: window.vertical_tabs_panel_open.unwrap_or(false),
            left_panel_width,
            tab_groups,
        });
    }
    if windows.is_empty() {
        return Ok(None);
    }
    Ok(Some(AppState {
        windows,
        active_window_index,
        block_lists: Arc::new(block_list::get_all_restored_blocks(connection)?),
    }))
}

fn decode_color(value: Option<&str>) -> SelectedTabColor {
    value
        .and_then(|value| {
            serde_yaml::from_str(value).ok().or_else(|| {
                serde_yaml::from_str::<AnsiColorIdentifier>(value)
                    .ok()
                    .map(SelectedTabColor::Color)
            })
        })
        .unwrap_or_default()
}

fn read_node(connection: &mut SqliteConnection, node: model::PaneNode) -> Option<PaneNodeSnapshot> {
    match read_node_inner(connection, node) {
        Ok(node) => node,
        Err(_) => {
            log::warn!("Skipping unreadable legacy pane");
            None
        }
    }
}

fn read_node_inner(
    connection: &mut SqliteConnection,
    node: model::PaneNode,
) -> Result<Option<PaneNodeSnapshot>> {
    if node.is_leaf {
        let leaf = schema::pane_leaves::table
            .filter(schema::pane_leaves::pane_node_id.eq(node.id))
            .first::<model::PaneLeaf>(connection)?;
        let Some(contents) = read_leaf(connection, node.id, &leaf.kind)? else {
            return Ok(None);
        };
        return Ok(Some(PaneNodeSnapshot::Leaf(Box::new(LeafSnapshot {
            is_focused: leaf.is_focused,
            custom_vertical_tabs_title: leaf.custom_vertical_tabs_title,
            contents,
        }))));
    }
    let branch = schema::pane_branches::table
        .filter(schema::pane_branches::pane_node_id.eq(node.id))
        .first::<model::PaneBranch>(connection)?;
    let nodes = schema::pane_nodes::table
        .filter(schema::pane_nodes::tab_id.eq(node.tab_id))
        .filter(schema::pane_nodes::parent_pane_node_id.eq(node.id))
        .order(schema::pane_nodes::id.asc())
        .load::<model::PaneNode>(connection)?;
    let children: Vec<_> = nodes
        .into_iter()
        .filter_map(|node| {
            let flex = node
                .flex
                .filter(|flex| flex.is_finite() && *flex > 0.)
                .unwrap_or(1.);
            read_node(connection, node).map(|node| (PaneFlex(flex), node))
        })
        .collect();
    if children.is_empty() {
        return Ok(None);
    }
    Ok(Some(PaneNodeSnapshot::Branch(BranchSnapshot {
        direction: if branch.horizontal {
            SplitDirection::Horizontal
        } else {
            SplitDirection::Vertical
        },
        children,
    })))
}

fn read_leaf(
    connection: &mut SqliteConnection,
    id: i32,
    kind: &str,
) -> Result<Option<LeafContents>> {
    Ok(Some(match kind {
        model::TERMINAL_PANE_KIND => {
            let pane = schema::terminal_panes::table
                .find(id)
                .select(model::TerminalPane::as_select())
                .first::<model::TerminalPane>(connection)?;
            LeafContents::Terminal(TerminalPaneSnapshot {
                uuid: pane.uuid,
                cwd: pane.cwd,
                is_active: pane.is_active,
                shell_launch_data: pane
                    .shell_launch_data
                    .and_then(|json| serde_json::from_str(&json).ok()),
            })
        }
        model::NOTEBOOK_PANE_KIND => {
            let pane = schema::notebook_panes::table
                .find(id)
                .select(model::NotebookPane::as_select())
                .first::<model::NotebookPane>(connection)?;
            let notebook = if let Some(path) = pane.local_path {
                NotebookPaneSnapshot::LocalFileNotebook {
                    path: Some(OsString::from_vec(path).into()),
                }
            } else {
                let Some(id) = legacy_object_id(connection, "NOTEBOOK", pane.notebook_id)? else {
                    return Ok(None);
                };
                if schema::notebooks::table
                    .find(id)
                    .filter(schema::notebooks::data.is_not_null())
                    .select(schema::notebooks::id)
                    .first::<i32>(connection)
                    .optional()?
                    .is_none()
                {
                    return Ok(None);
                }
                NotebookPaneSnapshot::LocalNotebook {
                    notebook_id: Some(NotebookId::from_legacy_id(id)),
                }
            };
            LeafContents::Notebook(notebook)
        }
        model::WORKFLOW_PANE_KIND => {
            let pane = schema::workflow_panes::table
                .find(id)
                .select(model::WorkflowPane::as_select())
                .first::<model::WorkflowPane>(connection)?;
            let Some(id) = legacy_object_id(connection, "WORKFLOW", pane.workflow_id)? else {
                return Ok(None);
            };
            let data = schema::workflows::table
                .find(id)
                .select(schema::workflows::data)
                .first::<String>(connection)?;
            LeafContents::Workflow(WorkflowPaneSnapshot::LocalWorkflow {
                workflow_id: WorkflowId::from_legacy_id(id),
                workflow: serde_json::from_str::<Workflow>(&data)?,
            })
        }
        model::CODE_PANE_KIND => {
            let pane = schema::code_panes::table
                .find(id)
                .select(model::CodePane::as_select())
                .first::<model::CodePane>(connection)?;
            let rows = schema::code_pane_tabs::table
                .filter(schema::code_pane_tabs::code_pane_id.eq(id))
                .order(schema::code_pane_tabs::tab_index.asc())
                .select(model::CodePaneTab::as_select())
                .load::<model::CodePaneTab>(connection)?;
            LeafContents::Code(CodePaneSnapShot::Local {
                active_tab_index: usize::try_from(pane.active_tab_index)
                    .unwrap_or(0)
                    .min(rows.len().saturating_sub(1)),
                tabs: rows
                    .into_iter()
                    .map(|row| CodePaneTabSnapshot {
                        path: row.local_path.map(|path| OsString::from_vec(path).into()),
                    })
                    .collect(),
                source: pane
                    .source_data
                    .and_then(|json| serde_json::from_str(&json).ok()),
            })
        }
        model::SETTINGS_PANE_KIND => {
            let page = schema::settings_panes::table
                .find(id)
                .select(schema::settings_panes::current_page)
                .first::<String>(connection)?;
            let Some(current_page) = SettingsSection::from_slug(&page) else {
                return Ok(None);
            };
            LeafContents::Settings(SettingsPaneSnapshot::Local {
                current_page,
                search_query: None,
            })
        }
        model::CODE_REVIEW_PANE_KIND => {
            let pane = schema::code_review_panes::table
                .find(id)
                .select(model::CodeReviewPane::as_select())
                .first::<model::CodeReviewPane>(connection)?;
            LeafContents::CodeReview(CodeReviewPaneSnapshot::Local {
                terminal_uuid: pane.terminal_uuid,
                repo_path: PathBuf::from(pane.repo_path),
            })
        }
        model::GET_STARTED_PANE_KIND => LeafContents::GetStarted,
        _ => return Ok(None),
    }))
}

fn legacy_object_id(
    connection: &mut SqliteConnection,
    kind: &str,
    id: Option<String>,
) -> QueryResult<Option<i32>> {
    let Some(id) = id else { return Ok(None) };
    schema::object_metadata::table
        .filter(schema::object_metadata::object_type.eq(kind))
        .filter(schema::object_metadata::trashed_ts.is_null())
        .filter(
            schema::object_metadata::server_id
                .eq(&id)
                .or(schema::object_metadata::client_id.eq(&id)),
        )
        .select(schema::object_metadata::shareable_object_id)
        .first(connection)
        .optional()
}

#[cfg(test)]
#[path = "legacy_snapshot_tests.rs"]
mod tests;
