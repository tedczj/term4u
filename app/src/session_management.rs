use chrono::NaiveDateTime;
use warpui::{AppContext, Entity, EntityId, WindowId};

use crate::themes::theme::AnsiColorIdentifier;

use crate::pane_group::{PaneGroup, PaneId};
use crate::terminal::model::blockgrid::BlockGrid;
use crate::workspace::{PaneViewLocator, Workspace};

#[derive(Clone, Debug)]
pub struct TabNavigationData {
    pub title: String,
    pub subtitle: Option<String>,
    pub color: Option<AnsiColorIdentifier>,
    pub pane_group_id: EntityId,
    pub window_id: WindowId,
    pub tab_index: usize,
}

#[derive(Clone)]
pub struct SessionNavigationData {
    prompt: String,
    prompt_elements: SessionNavigationPromptElements,
    command_context: CommandContext,
    pane_view_locator: PaneViewLocator,
    window_id: WindowId,
    last_focus_ts: Option<NaiveDateTime>,
}

impl SessionNavigationData {
    pub fn new(
        prompt: String,
        prompt_elements: SessionNavigationPromptElements,
        command_context: CommandContext,
        pane_view_locator: PaneViewLocator,
        last_focus_ts: Option<NaiveDateTime>,
        window_id: WindowId,
    ) -> Self {
        Self {
            prompt,
            prompt_elements,
            command_context,
            pane_view_locator,
            window_id,
            last_focus_ts,
        }
    }

    pub fn is_for_session(&self, session_id: PaneId) -> bool {
        session_id == self.pane_view_locator.pane_id
    }

    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    pub fn prompt_elements(&self) -> &SessionNavigationPromptElements {
        &self.prompt_elements
    }

    pub fn command_context(&self) -> CommandContext {
        self.command_context.clone()
    }

    pub fn pane_view_locator(&self) -> PaneViewLocator {
        self.pane_view_locator
    }

    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    pub fn last_focus_ts(&self) -> Option<NaiveDateTime> {
        self.last_focus_ts
    }

    pub fn is_read_only(&self) -> bool {
        false
    }

    pub fn all_sessions(app: &AppContext) -> impl Iterator<Item = Self> + '_ {
        app.window_ids().flat_map(move |window_id| {
            app.views_of_type::<Workspace>(window_id)
                .unwrap_or_default()
                .into_iter()
                .flat_map(move |workspace| workspace.as_ref(app).workspace_sessions(window_id, app))
        })
    }
}

#[derive(Clone, Default)]
pub struct SessionNavigationPromptElements {
    pub ps1_prompt_grid: Option<BlockGrid>,
}

#[derive(Clone, Debug)]
pub enum CommandContext {
    LastRunCommand {
        last_run_command: String,
        mins_since_completion: Option<i64>,
    },
    RunningCommand { running_command: String },
    None,
}

impl CommandContext {
    pub fn a11y_description(&self) -> Option<String> {
        match self {
            Self::None => None,
            Self::LastRunCommand {
                last_run_command, ..
            } => Some(format!("Last run command {last_run_command}")),
            Self::RunningCommand { running_command } => {
                Some(format!("Currently running {running_command}"))
            }
        }
    }
}

pub struct RunningSessionSummary<'a> {
    pub long_running_cmds: Vec<&'a SessionNavigationData>,
}

impl<'a> RunningSessionSummary<'a> {
    pub fn new(sessions: &'a [SessionNavigationData]) -> Self {
        Self {
            long_running_cmds: sessions
                .iter()
                .filter(|session| {
                    matches!(session.command_context, CommandContext::RunningCommand { .. })
                })
                .collect(),
        }
    }
}

pub fn num_shared_sessions(_app: &AppContext) -> usize {
    0
}

pub struct ActiveSession {
    pub window_id: WindowId,
    pub pane_group_id: EntityId,
    pub pane_id: PaneId,
}

impl Entity for ActiveSession {
    type Event = ();
}
