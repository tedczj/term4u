use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::thread;

use ai::project_context::model::ProjectRulePath;
use anyhow::{Context, Result, anyhow};
use diesel::connection::{DefaultLoadingMode, SimpleConnection};
use diesel::prelude::*;
use diesel::result::Error;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::MigrationHarness;
use itertools::Itertools as _;
use lsp::supported_servers::LSPServerType;
use warp_errors::report_error;
use warpui::AppContext;

use super::block_list::{delete_blocks, save_block};
use super::model::{
    self, NewCommand, NewWorkspaceMetadata, Project, ProjectRules,
    WorkspaceMetadata as WorkspaceMetadataModel,
};
use super::{
    BlockCompleted, FinishedCommandMetadata, ModelEvent, PersistedData, PersistedDataScope,
    PersistenceScope, StartedCommandMetadata, WriterHandles, schema,
};
use crate::ai::persisted_workspace::EnablementState;
use crate::suggestions::ignored_suggestions_model::SuggestionType;
use crate::terminal::history::PersistedCommand;

const CHANNEL_SIZE: usize = 1024;
const COMMANDS_COUNT_LIMIT: i64 = 10_000;
const WARP_SQLITE_FILE_NAME: &str = "warp.sqlite";

pub fn initialize(
    ctx: &mut AppContext,
    scope: PersistenceScope,
    data_scope: PersistedDataScope,
) -> (Option<Box<PersistedData>>, Option<WriterHandles>) {
    let database_path = database_file_path_for_scope(&scope);
    match init_db(&scope) {
        Ok(mut connection) => {
            let persisted_data = read_persisted_data(&mut connection, data_scope);
            let writer = match start_writer(connection, database_path.clone()) {
                Ok(writer) => Some(writer),
                Err(error) => {
                    report_error!(error.context("Failed to start SQLite writer"));
                    None
                }
            };
            (persisted_data, writer)
        }
        Err(error) => {
            report_error!(error.context("Failed to initialize SQLite persistence"));
            let _ = ctx;
            (None, None)
        }
    }
}

fn read_persisted_data(
    connection: &mut SqliteConnection,
    data_scope: PersistedDataScope,
) -> Option<Box<PersistedData>> {
    match read_sqlite_data(connection, data_scope) {
        Ok(data) => Some(Box::new(data)),
        Err(error) => {
            report_error!(anyhow::Error::new(error).context("Failed to read persisted data"));
            None
        }
    }
}

pub fn establish_ro_connection(database_url: &str) -> Result<SqliteConnection> {
    establish_connection(database_url, true)
}

fn establish_connection(database_url: &str, read_only: bool) -> Result<SqliteConnection> {
    let read_only_url;
    let database_url = if read_only {
        read_only_url = format!("file:{database_url}?mode=ro");
        &read_only_url
    } else {
        database_url
    };
    let mut connection = SqliteConnection::establish(database_url)?;
    connection.batch_execute(
        r#"
        PRAGMA foreign_keys = ON;
        PRAGMA busy_timeout = 1000;
        PRAGMA journal_mode = WAL;
        PRAGMA wal_autocheckpoint = 500;
        "#,
    )?;
    Ok(connection)
}

pub(super) fn init_db(scope: &PersistenceScope) -> Result<SqliteConnection> {
    let database_path = database_file_path_for_scope(scope);
    let parent = database_path
        .parent()
        .expect("database file path should be absolute");
    std::fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create SQLite directory {}", parent.display()))?;
    if matches!(scope, PersistenceScope::App) {
        migrate_old_sqlite_into_secure_container_if_needed(&database_path);
    }
    setup_database(&database_path)
}

fn migrate_old_sqlite_into_secure_container_if_needed(database_path: &Path) {
    let old_database_path = warp_core::paths::state_dir().join(WARP_SQLITE_FILE_NAME);
    if old_database_path == database_path || !old_database_path.exists() || database_path.exists() {
        return;
    }

    if let Err(error) = std::fs::rename(&old_database_path, database_path) {
        report_error!(anyhow::Error::new(error).context("Failed to migrate SQLite database"));
        return;
    }

    for suffix in ["sqlite-wal", "sqlite-shm"] {
        let old_path = old_database_path.with_extension(suffix);
        let new_path = database_path.with_extension(suffix);
        if let Err(error) = std::fs::rename(old_path, new_path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            report_error!(
                anyhow::Error::new(error).context("Failed to migrate SQLite sidecar file")
            );
        }
    }
}

fn setup_database(database_path: &Path) -> Result<SqliteConnection> {
    let database_url = database_path
        .to_str()
        .ok_or_else(|| anyhow!("Failed to convert SQLite path to a string"))?;
    let mut connection = establish_connection(database_url, false)?;
    connection
        .run_pending_migrations(persistence::MIGRATIONS)
        .map_err(|error| anyhow!(error))
        .context("Failed to perform SQLite migrations")?;
    Ok(connection)
}

pub fn database_file_path_for_scope(scope: &PersistenceScope) -> PathBuf {
    match scope {
        PersistenceScope::App => warp_core::paths::secure_state_dir()
            .unwrap_or_else(warp_core::paths::state_dir)
            .join(WARP_SQLITE_FILE_NAME),
        PersistenceScope::Tui => warp_core::paths::tui_state_dir().join(WARP_SQLITE_FILE_NAME),
    }
}

pub fn database_file_path_for_current_scope() -> PathBuf {
    database_file_path_for_scope(&super::current_scope())
}

fn start_writer(connection: SqliteConnection, database_path: PathBuf) -> Result<WriterHandles> {
    let (sender, receiver) = std::sync::mpsc::sync_channel(CHANNEL_SIZE);
    let handle = thread::Builder::new()
        .name("SQLite Writer".into())
        .spawn(move || {
            let mut connection = connection;
            loop {
                let first = match receiver.recv() {
                    Ok(event) => event,
                    Err(_) => break,
                };
                let mut events = vec![first];
                events.extend(receiver.try_iter());
                for event in deduplicate_events(events) {
                    if matches!(event, ModelEvent::Terminate) {
                        return;
                    }
                    if let Err(error) = handle_model_event(event, &mut connection) {
                        report_error!(
                            error.context("Failed to persist local model event"),
                            extra: { "database_path" => %database_path.display() }
                        );
                    }
                }
            }
        })?;
    Ok(WriterHandles { handle, sender })
}

fn deduplicate_events(events: Vec<ModelEvent>) -> Vec<ModelEvent> {
    let last_snapshot = events
        .iter()
        .rposition(|event| matches!(event, ModelEvent::Snapshot(_)));
    events
        .into_iter()
        .enumerate()
        .filter_map(|(index, event)| match event {
            ModelEvent::Snapshot(_) if last_snapshot.is_some_and(|last| index < last) => None,
            event => Some(event),
        })
        .collect()
}

fn handle_model_event(event: ModelEvent, connection: &mut SqliteConnection) -> Result<()> {
    match event {
        ModelEvent::SaveBlock(BlockCompleted {
            pane_id,
            is_local,
            block,
        }) => save_block(connection, pane_id, &block, is_local)?,
        ModelEvent::DeleteBlocks(pane_id) => delete_blocks(connection, pane_id)?,
        ModelEvent::Snapshot(_) => {}
        ModelEvent::InsertCommand { metadata } => insert_command(connection, metadata)?,
        ModelEvent::UpdateFinishedCommand { metadata } => {
            update_finished_command(connection, metadata)?;
        }
        ModelEvent::Terminate => unreachable!("terminate is handled by the writer loop"),
        ModelEvent::UpsertCodebaseIndexMetadata { index_metadata } => {
            save_codebase_index_metadata(connection, *index_metadata)?;
        }
        ModelEvent::DeleteCodebaseIndexMetadata { repo_path } => {
            delete_codebase_index_metadata(connection, &repo_path)?;
        }
        ModelEvent::UpsertProject { project } => save_project(connection, project)?,
        ModelEvent::DeleteProject { path } => delete_project(connection, &path)?,
        ModelEvent::UpsertProjectRules { project_rule_paths } => {
            upsert_project_rules(connection, project_rule_paths)?;
        }
        ModelEvent::DeleteProjectRules { path } => delete_project_rules(connection, path)?,
        ModelEvent::AddIgnoredSuggestion {
            suggestion,
            suggestion_type,
        } => add_ignored_suggestion(connection, suggestion, suggestion_type)?,
        ModelEvent::RemoveIgnoredSuggestion {
            suggestion,
            suggestion_type,
        } => remove_ignored_suggestion(connection, suggestion, suggestion_type)?,
        ModelEvent::UpsertWorkspaceLanguageServer {
            workspace_path,
            lsp_type,
            enabled,
        } => upsert_workspace_language_server(connection, &workspace_path, lsp_type, enabled)?,
    }
    Ok(())
}

fn read_sqlite_data(
    connection: &mut SqliteConnection,
    data_scope: PersistedDataScope,
) -> Result<PersistedData, Error> {
    let codebase_indices = get_all_codebase_index_metadata(connection)?;
    if matches!(data_scope, PersistedDataScope::CodebaseIndicesOnly) {
        return Ok(PersistedData {
            app_state: None,
            command_history: Vec::new(),
            legacy_notebooks: Vec::new(),
            codebase_indices,
            workspace_language_servers: HashMap::new(),
            projects: Vec::new(),
            project_rules: Vec::new(),
            ignored_suggestions: Vec::new(),
        });
    }

    let command_history = if data_scope.command_history() {
        schema::commands::table
            .order(schema::commands::id.desc())
            .load_iter::<model::Command, DefaultLoadingMode>(connection)?
            .filter_map(Result::ok)
            .map(PersistedCommand::from)
            .collect()
    } else {
        Vec::new()
    };
    let legacy_notebooks = schema::notebooks::table
        .select((
            schema::notebooks::id,
            schema::notebooks::title,
            schema::notebooks::data,
        ))
        .load(connection)?;

    Ok(PersistedData {
        app_state: None,
        command_history,
        legacy_notebooks,
        codebase_indices,
        workspace_language_servers: get_all_workspace_language_servers_by_workspace(connection)?,
        projects: get_all_projects(connection)?,
        project_rules: get_all_project_rules(connection)?,
        ignored_suggestions: get_all_ignored_suggestions(connection)?,
    })
}

fn save_codebase_index_metadata(
    connection: &mut SqliteConnection,
    index_metadata: ai::workspace::WorkspaceMetadata,
) -> Result<()> {
    use schema::workspace_metadata::dsl::*;

    let metadata: NewWorkspaceMetadata = index_metadata.into();
    diesel::insert_into(workspace_metadata)
        .values(metadata.clone())
        .on_conflict(repo_path)
        .do_update()
        .set(&metadata)
        .execute(connection)?;
    Ok(())
}

fn get_all_codebase_index_metadata(
    connection: &mut SqliteConnection,
) -> Result<Vec<ai::workspace::WorkspaceMetadata>, Error> {
    Ok(schema::workspace_metadata::table
        .load_iter::<WorkspaceMetadataModel, DefaultLoadingMode>(connection)?
        .filter_map(|row| row.ok().map(Into::into))
        .collect_vec())
}

fn delete_codebase_index_metadata(
    connection: &mut SqliteConnection,
    index_path: &Path,
) -> Result<()> {
    let index_path = index_path.to_string_lossy().into_owned();
    diesel::delete(
        schema::workspace_metadata::table
            .filter(schema::workspace_metadata::repo_path.eq(index_path)),
    )
    .execute(connection)?;
    Ok(())
}

fn get_all_workspace_language_servers_by_workspace(
    connection: &mut SqliteConnection,
) -> Result<HashMap<PathBuf, HashMap<LSPServerType, EnablementState>>, Error> {
    let rows = schema::workspace_language_server::table
        .inner_join(schema::workspace_metadata::table)
        .select((
            schema::workspace_metadata::repo_path,
            schema::workspace_language_server::language_server_name,
            schema::workspace_language_server::enabled,
        ))
        .load::<(String, String, String)>(connection)?;
    let mut grouped = HashMap::new();
    for (path, server, enabled) in rows {
        let (Ok(server), Ok(enabled)) = (
            serde_json::from_str(&server),
            serde_json::from_str(&enabled),
        ) else {
            continue;
        };
        grouped
            .entry(PathBuf::from(path))
            .or_insert_with(HashMap::new)
            .insert(server, enabled);
    }
    Ok(grouped)
}

fn upsert_workspace_language_server(
    connection: &mut SqliteConnection,
    workspace_path: &Path,
    server_type: LSPServerType,
    enablement: EnablementState,
) -> Result<()> {
    let path = workspace_path.to_string_lossy().into_owned();
    let workspace = schema::workspace_metadata::table
        .filter(schema::workspace_metadata::repo_path.eq(path))
        .first::<WorkspaceMetadataModel>(connection)
        .optional()?
        .ok_or_else(|| anyhow!("Cannot persist a language server for an unknown workspace"))?;
    let server = serde_json::to_string(&server_type)?;
    let enabled = serde_json::to_string(&enablement)?;
    let existing = schema::workspace_language_server::table
        .filter(schema::workspace_language_server::workspace_id.eq(workspace.id))
        .filter(schema::workspace_language_server::language_server_name.eq(&server))
        .first::<model::WorkspaceLanguageServer>(connection)
        .optional()?;
    if let Some(existing) = existing {
        diesel::update(schema::workspace_language_server::table.find(existing.id))
            .set(schema::workspace_language_server::enabled.eq(enabled))
            .execute(connection)?;
    } else {
        diesel::insert_into(schema::workspace_language_server::table)
            .values(model::NewWorkspaceLanguageServer {
                workspace_id: workspace.id,
                language_server_name: server,
                enabled,
            })
            .execute(connection)?;
    }
    Ok(())
}

fn save_project(connection: &mut SqliteConnection, project: Project) -> Result<()> {
    diesel::insert_into(schema::projects::table)
        .values(project.clone())
        .on_conflict(schema::projects::path)
        .do_update()
        .set(project)
        .execute(connection)?;
    Ok(())
}

fn get_all_projects(connection: &mut SqliteConnection) -> Result<Vec<Project>, Error> {
    Ok(schema::projects::table
        .load_iter::<Project, DefaultLoadingMode>(connection)?
        .filter_map(Result::ok)
        .collect())
}

fn delete_project(connection: &mut SqliteConnection, project_path: &str) -> Result<()> {
    diesel::delete(schema::projects::table.filter(schema::projects::path.eq(project_path)))
        .execute(connection)?;
    Ok(())
}

fn get_all_project_rules(connection: &mut SqliteConnection) -> Result<Vec<ProjectRulePath>, Error> {
    Ok(schema::project_rules::table
        .load_iter::<ProjectRules, DefaultLoadingMode>(connection)?
        .filter_map(|row| {
            row.ok().map(|row| ProjectRulePath {
                path: PathBuf::from(row.path),
                project_root: PathBuf::from(row.project_root),
            })
        })
        .collect())
}

fn upsert_project_rules(
    connection: &mut SqliteConnection,
    rules: Vec<ProjectRulePath>,
) -> Result<()> {
    for rule in rules {
        let row = model::NewProjectRules {
            path: rule.path.to_string_lossy().into_owned(),
            project_root: rule.project_root.to_string_lossy().into_owned(),
        };
        diesel::insert_into(schema::project_rules::table)
            .values(&row)
            .on_conflict(schema::project_rules::path)
            .do_update()
            .set(&row)
            .execute(connection)?;
    }
    Ok(())
}

fn delete_project_rules(connection: &mut SqliteConnection, paths: Vec<PathBuf>) -> Result<()> {
    let paths = paths
        .into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    diesel::delete(schema::project_rules::table.filter(schema::project_rules::path.eq_any(paths)))
        .execute(connection)?;
    Ok(())
}

fn get_all_ignored_suggestions(
    connection: &mut SqliteConnection,
) -> Result<Vec<(String, SuggestionType)>, Error> {
    Ok(schema::ignored_suggestions::table
        .select((
            schema::ignored_suggestions::suggestion,
            schema::ignored_suggestions::suggestion_type,
        ))
        .load::<(String, String)>(connection)?
        .into_iter()
        .filter_map(|(suggestion, kind)| {
            SuggestionType::from_str(&kind).map(|kind| (suggestion, kind))
        })
        .collect())
}

fn add_ignored_suggestion(
    connection: &mut SqliteConnection,
    suggestion: String,
    suggestion_type: SuggestionType,
) -> Result<()> {
    let row = model::NewIgnoredSuggestion {
        suggestion,
        suggestion_type: suggestion_type.as_str().to_owned(),
    };
    diesel::insert_into(schema::ignored_suggestions::table)
        .values(row)
        .on_conflict((
            schema::ignored_suggestions::suggestion,
            schema::ignored_suggestions::suggestion_type,
        ))
        .do_nothing()
        .execute(connection)?;
    Ok(())
}

fn remove_ignored_suggestion(
    connection: &mut SqliteConnection,
    suggestion: String,
    suggestion_type: SuggestionType,
) -> Result<()> {
    diesel::delete(
        schema::ignored_suggestions::table
            .filter(schema::ignored_suggestions::suggestion.eq(suggestion))
            .filter(schema::ignored_suggestions::suggestion_type.eq(suggestion_type.as_str())),
    )
    .execute(connection)?;
    Ok(())
}

impl From<StartedCommandMetadata> for NewCommand {
    fn from(metadata: StartedCommandMetadata) -> Self {
        Self {
            command: metadata.command,
            exit_code: None,
            start_ts: metadata.start_ts.map(|timestamp| timestamp.naive_utc()),
            completed_ts: None,
            pwd: metadata.pwd,
            shell: metadata.shell,
            username: metadata.username,
            hostname: metadata.hostname,
            session_id: metadata
                .session_id
                .and_then(|id| id.as_u64().try_into().ok()),
            git_branch: metadata.git_branch,
            cloud_workflow_id: None,
            workflow_command: metadata.workflow_command,
            is_agent_executed: Some(false),
        }
    }
}

fn insert_command(
    connection: &mut SqliteConnection,
    metadata: StartedCommandMetadata,
) -> Result<(), Error> {
    connection.transaction(|connection| {
        let command_count = schema::commands::table.count().first::<i64>(connection)?;
        if command_count >= COMMANDS_COUNT_LIMIT {
            let oldest_id = schema::commands::table
                .select(schema::commands::id)
                .order(schema::commands::id.asc())
                .first::<i32>(connection)?;
            diesel::delete(schema::commands::table.filter(schema::commands::id.eq(oldest_id)))
                .execute(connection)?;
        }
        diesel::insert_into(schema::commands::table)
            .values(NewCommand::from(metadata))
            .execute(connection)?;
        Ok(())
    })
}

fn update_finished_command(
    connection: &mut SqliteConnection,
    metadata: FinishedCommandMetadata,
) -> Result<(), Error> {
    let session_id: Option<i64> = metadata.session_id.as_u64().try_into().ok();
    diesel::update(schema::commands::table)
        .filter(schema::commands::start_ts.eq(Some(metadata.start_ts.naive_utc())))
        .filter(schema::commands::session_id.eq(session_id))
        .set((
            schema::commands::exit_code.eq(metadata.exit_code.value()),
            schema::commands::completed_ts.eq(metadata.completed_ts.naive_utc()),
        ))
        .execute(connection)?;
    Ok(())
}
