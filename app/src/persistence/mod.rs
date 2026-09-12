#![cfg_attr(not(feature = "local_fs"), allow(dead_code))]

cfg_if::cfg_if! {
    if #[cfg(feature = "local_fs")] {
        mod block_list;
        mod sqlite;
        mod local_snapshot;
        #[cfg(target_os = "macos")]
        mod legacy_snapshot;
    }
}

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::SyncSender;
use std::thread::JoinHandle;

use ai::project_context::model::ProjectRulePath;
use ai::workspace::WorkspaceMetadata as CodeWorkspaceMetadata;
use chrono::{DateTime, Local};
use instant::Instant;
use lsp::supported_servers::LSPServerType;
pub use persistence::model;
#[cfg_attr(not(feature = "local_fs"), expect(unused_imports))]
pub use persistence::schema;
use warp_core::command::ExitCode;
use warp_errors::report_error;
use warpui::{AppContext, Entity, SingletonEntity};

use crate::ai::persisted_workspace::EnablementState;
use crate::app_state::AppState;
use crate::terminal::history::PersistedCommand;
use crate::terminal::model::block::SerializedBlock;
use crate::terminal::model::session::SessionId;

#[derive(Clone)]
pub enum PersistenceScope {
    /// The GUI app (and other launch modes that share its database).
    App,
    /// The `warp-tui` front-end, which keeps its own database so GUI/TUI
    /// version skew can never migrate a shared database out from under the
    /// older binary. Each front-end restores only its own local state.
    Tui,
}

/// Which subsets of [`PersistedData`] a launch mode actually consumes.
///
/// Loading everything unconditionally is expensive (GUI session-restore
/// payloads dominate startup on large databases), so headless launch modes
/// opt out of the data they never read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistedDataScope {
    /// The GUI app: everything, including window/tab/block session
    /// restoration and command history.
    Full,
    /// The headless front-end, without GUI session restoration.
    TuiFrontend,
}

impl PersistedDataScope {
    /// Window/tab/pane snapshots and restored blocks.
    fn session_restoration(self) -> bool {
        matches!(self, PersistedDataScope::Full)
    }
}

/// Initializes the persistence "subsystem".
///
/// Returns the previously-persisted data, if any, and handles for
/// writing updated data to persist, if the persistence subsystem is
/// available.
#[tracing::instrument(name = "persistence::initialize", skip_all, fields(tags.cloud_agent = true))]
#[cfg_attr(not(feature = "local_fs"), allow(unused_variables))]
pub fn initialize(
    ctx: &mut AppContext,
    scope: PersistenceScope,
    data_scope: PersistedDataScope,
) -> (Option<Box<PersistedData>>, Option<WriterHandles>) {
    // Record the scope for ad-hoc read-only connections; keep the first value
    // if this is ever called more than once in a process (e.g. tests).
    cfg_if::cfg_if! {
        if #[cfg(feature = "local_fs")] {
            sqlite::initialize(ctx, scope, data_scope)
        } else {
            (None, None)
        }
    }
}

/// Holds interfaces to the writer thread.
pub struct WriterHandles {
    pub handle: JoinHandle<()>,
    pub sender: SyncSender<ModelEvent>,
}

/// Model for interacting with the writer thread.
pub struct PersistenceWriter {
    thread_handle: Option<JoinHandle<()>>,
    model_event_sender: Option<SyncSender<ModelEvent>>,
}

impl PersistenceWriter {
    pub fn new(handle: Option<WriterHandles>) -> Self {
        let (thread_handle, model_event_sender) = match handle {
            Some(handle) => (Some(handle.handle), Some(handle.sender)),
            None => (None, None),
        };
        Self {
            thread_handle,
            model_event_sender,
        }
    }

    /// Sending half for sending model updates to the persistence writer thread.
    pub fn sender(&self) -> Option<SyncSender<ModelEvent>> {
        self.model_event_sender.clone()
    }

    /// Synchronously terminate the SQLite writer thread.
    pub fn terminate(&mut self) {
        if let Some(handle) = self.thread_handle.take() {
            let start = Instant::now();
            let Some(sender) = self.sender() else {
                report_error!("Model event sender should exist if thread handle is set");
                return;
            };
            if let Err(err) = sender.send(ModelEvent::Terminate) {
                report_error!(
                    anyhow::Error::new(err).context("Could not terminate SQLite writer thread")
                );
            }
            if handle.join().is_err() {
                // If crash reporting is enabled, Sentry will have already handled the panic.
                report_error!("SQLite writer thread panicked");
            }
            log::info!("Shut down SQLite writer in {:?}", start.elapsed());
        }
    }
}

impl Drop for PersistenceWriter {
    fn drop(&mut self) {
        self.terminate();
    }
}

impl Entity for PersistenceWriter {
    type Event = ();
}

impl SingletonEntity for PersistenceWriter {}

/// Local data loaded for startup and session restoration.
pub struct PersistedData {
    pub app_state: Option<AppState>,
    pub command_history: Vec<PersistedCommand>,
    pub legacy_notebooks: Vec<(i32, Option<String>, Option<String>)>,
    pub codebase_indices: Vec<CodeWorkspaceMetadata>,
    pub workspace_language_servers: HashMap<PathBuf, HashMap<LSPServerType, EnablementState>>,
    pub project_rules: Vec<ProjectRulePath>,
}

#[derive(Clone, Debug)]
pub struct BlockCompleted {
    pub pane_id: Vec<u8>,
    /// Indicates if the block was created locally (e.g. not in a remote session)
    pub is_local: bool,
    pub block: Arc<SerializedBlock>,
}

#[derive(Debug)]
pub struct StartedCommandMetadata {
    pub command: String,
    pub start_ts: Option<DateTime<Local>>,
    pub pwd: Option<String>,
    pub shell: Option<String>,
    pub username: Option<String>,
    pub hostname: Option<String>,
    pub session_id: Option<SessionId>,
    pub git_branch: Option<String>,
    pub workflow_command: Option<String>,
}

#[derive(Debug)]
pub struct FinishedCommandMetadata {
    pub exit_code: ExitCode,
    pub start_ts: DateTime<Local>,
    pub completed_ts: DateTime<Local>,
    pub session_id: SessionId,
}

#[derive(Debug)]
pub enum ModelEvent {
    SaveBlock(BlockCompleted),
    DeleteBlocks(Vec<u8>),
    Snapshot(AppState),
    InsertCommand {
        metadata: StartedCommandMetadata,
    },
    UpdateFinishedCommand {
        metadata: FinishedCommandMetadata,
    },
    Terminate,
    UpsertCodebaseIndexMetadata {
        index_metadata: Box<CodeWorkspaceMetadata>,
    },
    DeleteCodebaseIndexMetadata {
        repo_path: PathBuf,
    },
    UpsertProjectRules {
        project_rule_paths: Vec<ProjectRulePath>,
    },
    DeleteProjectRules {
        path: Vec<PathBuf>,
    },
    UpsertWorkspaceLanguageServer {
        workspace_path: PathBuf,
        lsp_type: LSPServerType,
        enabled: EnablementState,
    },
}

#[cfg(feature = "local_fs")]
pub(crate) fn save_app_snapshot(ctx: &AppContext) {
    let snapshot = crate::app_state::get_app_state(ctx);
    if let Some(sender) = PersistenceWriter::as_ref(ctx).sender()
        && sender.send(ModelEvent::Snapshot(snapshot)).is_err()
    {
        log::warn!("Could not queue local session snapshot");
    }
}
