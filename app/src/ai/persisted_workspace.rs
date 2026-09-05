use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::SyncSender;

use ai::workspace::WorkspaceMetadata;
use chrono::Utc;
use lsp::LanguageId;
use lsp::supported_servers::LSPServerType;
use serde::{Deserialize, Serialize};
use warpui::{Entity, ModelContext, SingletonEntity};

#[cfg(feature = "local_fs")]
use lsp::{LspManagerModel, LspServerConfig};
#[cfg(feature = "local_fs")]
use warp_core::channel::ChannelState;

use crate::persistence::ModelEvent;
#[cfg(feature = "local_fs")]
use crate::terminal::local_shell::LocalShellState;
#[cfg(feature = "local_fs")]
use crate::view_components::DismissibleToast;
#[cfg(feature = "local_fs")]
use crate::workspace::ToastStack;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnablementState {
    Yes,
    No,
    Suggested,
}

#[cfg(feature = "local_fs")]
pub enum LspTask {
    Install { server_type: LSPServerType },
    Spawn { file_path: PathBuf },
}

pub enum LSPEnablementResultForFile {
    Enabled,
    UnsupportedLanguage,
    LSPNotEnabled { root_name: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LspRepoStatus {
    Ready,
    Enabled,
    CheckingForInstallation,
    DisabledAndInstalled { server_type: LSPServerType },
    DisabledAndNotInstalled { server_type: LSPServerType },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LSPInstallationStatus {
    Installed,
    NotInstalled,
    Checking,
}

impl LspRepoStatus {
    pub fn from_installation_status(
        status: &LSPInstallationStatus,
        server_type: LSPServerType,
    ) -> Self {
        match status {
            LSPInstallationStatus::Installed => Self::DisabledAndInstalled { server_type },
            LSPInstallationStatus::NotInstalled => Self::DisabledAndNotInstalled { server_type },
            LSPInstallationStatus::Checking => Self::CheckingForInstallation,
        }
    }
}

pub struct Workspace {
    pub metadata: WorkspaceMetadata,
    pub language_servers: HashMap<LSPServerType, EnablementState>,
}

pub struct PersistedWorkspace {
    workspaces: HashMap<PathBuf, Workspace>,
    model_event_sender: Option<SyncSender<ModelEvent>>,
    #[cfg(feature = "local_fs")]
    lsp_installation_status: HashMap<LSPServerType, LSPInstallationStatus>,
}

#[derive(Debug, Clone)]
pub enum PersistedWorkspaceEvent {
    InstallStatusUpdate {
        server_type: LSPServerType,
        status: LSPInstallationStatus,
    },
    InstallationFailed,
    AvailableServersDetected {
        workspace_path: PathBuf,
        servers: Vec<LSPServerType>,
    },
    WorkspaceAdded { path: PathBuf },
}

impl Entity for PersistedWorkspace {
    type Event = PersistedWorkspaceEvent;
}

impl SingletonEntity for PersistedWorkspace {}

impl PersistedWorkspace {
    pub fn new_local(_ctx: &mut ModelContext<Self>) -> Self {
        Self {
            workspaces: HashMap::new(),
            model_event_sender: None,
            #[cfg(feature = "local_fs")]
            lsp_installation_status: HashMap::new(),
        }
    }

    #[cfg(any(test, feature = "test-util"))]
    pub fn new_for_test(ctx: &mut ModelContext<Self>) -> Self {
        Self::new_local(ctx)
    }

    pub fn enable_lsp_server_for_path(&mut self, path: &Path, server_type: LSPServerType) {
        self.set_lsp_server_for_path(path, server_type, EnablementState::Yes);
    }

    pub fn disable_lsp_server_for_path(&mut self, path: &Path, server_type: LSPServerType) {
        self.set_lsp_server_for_path(path, server_type, EnablementState::No);
    }

    fn set_lsp_server_for_path(
        &mut self,
        path: &Path,
        server_type: LSPServerType,
        state: EnablementState,
    ) {
        let workspace = self
            .workspaces
            .entry(path.to_path_buf())
            .or_insert_with(|| Workspace {
                metadata: WorkspaceMetadata {
                    path: path.to_path_buf(),
                    modified_ts: Some(Utc::now()),
                    ..Default::default()
                },
                language_servers: HashMap::new(),
            });
        workspace.language_servers.insert(server_type, state);
        self.save_to_db([
            ModelEvent::UpsertCodebaseIndexMetadata {
                index_metadata: Box::new(workspace.metadata.clone()),
            },
            ModelEvent::UpsertWorkspaceLanguageServer {
                workspace_path: path.to_path_buf(),
                lsp_type: server_type,
                enabled: state,
            },
        ]);
    }

    pub fn root_for_workspace<'a>(&self, path: &'a Path) -> Option<&'a Path> {
        path.ancestors()
            .find(|ancestor| self.workspaces.contains_key(*ancestor))
    }

    pub fn has_enabled_lsp_server_for_file_path(&self, path: &Path) -> LSPEnablementResultForFile {
        let Some(language) = LanguageId::from_path(path) else {
            return LSPEnablementResultForFile::UnsupportedLanguage;
        };
        let Some(root) = self.root_for_workspace(path) else {
            return LSPEnablementResultForFile::LSPNotEnabled { root_name: None };
        };
        let workspace = &self.workspaces[root];
        if workspace.language_servers.iter().any(|(server, state)| {
            *state == EnablementState::Yes && server.languages().contains(&language)
        }) {
            LSPEnablementResultForFile::Enabled
        } else {
            LSPEnablementResultForFile::LSPNotEnabled {
                root_name: root
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(ToOwned::to_owned),
            }
        }
    }

    pub fn enabled_lsp_servers(
        &self,
        path: &Path,
    ) -> Option<impl Iterator<Item = LSPServerType> + use<'_>> {
        let root = self.root_for_workspace(path)?;
        self.workspaces.get(root).map(|workspace| {
            workspace
                .language_servers
                .iter()
                .filter_map(|(server, state)| (*state == EnablementState::Yes).then_some(*server))
        })
    }

    pub fn all_lsp_servers(
        &self,
        path: &Path,
        include_suggested: bool,
    ) -> Option<impl Iterator<Item = (LSPServerType, EnablementState)> + use<'_>> {
        let root = self.root_for_workspace(path)?;
        self.workspaces.get(root).map(move |workspace| {
            workspace
                .language_servers
                .iter()
                .filter(move |(_, state)| {
                    include_suggested || **state != EnablementState::Suggested
                })
                .map(|(server, state)| (*server, *state))
        })
    }

    pub fn total_lsp_server_count(&self, include_suggested: bool) -> usize {
        self.workspaces
            .values()
            .flat_map(|workspace| workspace.language_servers.values())
            .filter(|state| include_suggested || **state != EnablementState::Suggested)
            .count()
    }

    pub fn user_added_workspace(&mut self, path: PathBuf, ctx: &mut ModelContext<Self>) {
        let metadata = WorkspaceMetadata {
            path: path.clone(),
            navigated_ts: Some(Utc::now()),
            ..Default::default()
        };
        self.workspaces.entry(path.clone()).or_insert(Workspace {
            metadata: metadata.clone(),
            language_servers: HashMap::new(),
        });
        self.save_to_db([ModelEvent::UpsertCodebaseIndexMetadata {
            index_metadata: Box::new(metadata),
        }]);
        ctx.emit(PersistedWorkspaceEvent::WorkspaceAdded { path });
    }

    pub fn workspaces(&self) -> impl Iterator<Item = WorkspaceMetadata> + use<'_> {
        let mut workspaces = self
            .workspaces
            .values()
            .map(|workspace| workspace.metadata.clone())
            .collect::<Vec<_>>();
        workspaces.sort_by(WorkspaceMetadata::most_recently_touched);
        workspaces.into_iter()
    }

    pub fn navigated_to_path(&mut self, directory: &PathBuf) {
        if let Some(root) = self.root_for_workspace(directory).map(Path::to_path_buf)
            && let Some(workspace) = self.workspaces.get_mut(&root)
        {
            workspace.metadata.navigated_ts = Some(Utc::now());
        }
    }

    pub fn workspace_for_path(&self, root_path: &Path) -> Option<WorkspaceMetadata> {
        self.root_for_workspace(root_path)
            .and_then(|root| self.workspaces.get(root))
            .map(|workspace| workspace.metadata.clone())
    }

    fn save_to_db(&self, events: impl IntoIterator<Item = ModelEvent>) {
        if let Some(sender) = &self.model_event_sender {
            for event in events {
                if let Err(error) = sender.send(event) {
                    log::warn!("Unable to save local workspace metadata: {error}");
                }
            }
        }
    }

    #[cfg(feature = "local_fs")]
    pub fn detect_available_servers_for_workspaces(
        &mut self,
        paths: Vec<PathBuf>,
        rescan: bool,
        ctx: &mut ModelContext<Self>,
    ) {
        let mut pending = Vec::new();
        for path in paths {
            if !rescan
                && let Some(workspace) = self.workspaces.get(&path)
                && !workspace.language_servers.is_empty()
            {
                ctx.emit(PersistedWorkspaceEvent::AvailableServersDetected {
                    workspace_path: path,
                    servers: workspace.language_servers.keys().copied().collect(),
                });
            } else {
                pending.push(path);
            }
        }
        if pending.is_empty() {
            return;
        }
        let path_future = LocalShellState::handle(ctx)
            .update(ctx, |shell, ctx| shell.get_interactive_path_env_var(ctx));
        ctx.spawn(
            async move {
                let executor = lsp::CommandBuilder::new(path_future.await);
                let mut results = Vec::new();
                for path in pending {
                    let mut servers = Vec::new();
                    for server in LSPServerType::all() {
                        if server
                            .candidate()
                            .should_suggest_for_repo(&path, &executor)
                            .await
                        {
                            servers.push(server);
                        }
                    }
                    results.push((path, servers));
                }
                results
            },
            |model, results, ctx| {
                for (path, servers) in results {
                    let workspace = model.workspaces.entry(path.clone()).or_insert(Workspace {
                        metadata: WorkspaceMetadata {
                            path: path.clone(),
                            ..Default::default()
                        },
                        language_servers: HashMap::new(),
                    });
                    for server in &servers {
                        workspace
                            .language_servers
                            .entry(*server)
                            .or_insert(EnablementState::Suggested);
                    }
                    ctx.emit(PersistedWorkspaceEvent::AvailableServersDetected {
                        workspace_path: path,
                        servers,
                    });
                }
            },
        );
    }

    #[cfg(feature = "local_fs")]
    pub fn detect_lsp_workspace_status(
        &mut self,
        root: PathBuf,
        server_type: LSPServerType,
        ctx: &mut ModelContext<Self>,
    ) -> LspRepoStatus {
        if self
            .workspaces
            .get(&root)
            .and_then(|workspace| workspace.language_servers.get(&server_type))
            == Some(&EnablementState::Yes)
        {
            return LspRepoStatus::Enabled;
        }
        match self.lsp_installation_status.get(&server_type).copied() {
            Some(LSPInstallationStatus::Installed) => {
                LspRepoStatus::DisabledAndInstalled { server_type }
            }
            Some(LSPInstallationStatus::NotInstalled) => {
                LspRepoStatus::DisabledAndNotInstalled { server_type }
            }
            Some(LSPInstallationStatus::Checking) => LspRepoStatus::CheckingForInstallation,
            None => {
                self.lsp_installation_status
                    .insert(server_type, LSPInstallationStatus::Checking);
                let path_future = LocalShellState::handle(ctx)
                    .update(ctx, |shell, ctx| shell.get_interactive_path_env_var(ctx));
                ctx.spawn(
                    async move {
                        let executor = lsp::CommandBuilder::new(path_future.await);
                        server_type.candidate().is_installed(&executor).await
                    },
                    move |model, installed, ctx| {
                        let status = if installed {
                            LSPInstallationStatus::Installed
                        } else {
                            LSPInstallationStatus::NotInstalled
                        };
                        model.lsp_installation_status.insert(server_type, status);
                        ctx.emit(PersistedWorkspaceEvent::InstallStatusUpdate {
                            server_type,
                            status,
                        });
                    },
                );
                LspRepoStatus::CheckingForInstallation
            }
        }
    }

    #[cfg(feature = "local_fs")]
    pub fn execute_lsp_task(&mut self, task: LspTask, ctx: &mut ModelContext<Self>) {
        match task {
            LspTask::Install { server_type } => {
                self.lsp_installation_status
                    .insert(server_type, LSPInstallationStatus::NotInstalled);
                if let Some(window_id) = ctx.windows().active_window() {
                    ToastStack::handle(ctx).update(ctx, |toasts, ctx| {
                        toasts.add_ephemeral_toast(
                            DismissibleToast::error(server_type.manual_install_message()),
                            window_id,
                            ctx,
                        );
                    });
                }
                ctx.emit(PersistedWorkspaceEvent::InstallationFailed);
                ctx.emit(PersistedWorkspaceEvent::InstallStatusUpdate {
                    server_type,
                    status: LSPInstallationStatus::NotInstalled,
                });
            }
            LspTask::Spawn { file_path } => self.spawn_for_file(file_path, ctx),
        }
    }

    #[cfg(feature = "local_fs")]
    fn spawn_for_file(&self, file_path: PathBuf, ctx: &mut ModelContext<Self>) {
        let Some(root) = self.root_for_workspace(&file_path).map(Path::to_path_buf) else {
            return;
        };
        let Some(servers) = self.enabled_lsp_servers(&root) else {
            return;
        };
        let servers = servers.collect::<Vec<_>>();
        let path_future = LocalShellState::handle(ctx)
            .update(ctx, |shell, ctx| shell.get_interactive_path_env_var(ctx));
        ctx.spawn(
            async move { path_future.await },
            move |_, path_env, ctx| {
                for server in servers {
                    let config = LspServerConfig::new(
                        server,
                        root.clone(),
                        path_env.clone(),
                        ChannelState::app_id().application_name().to_owned(),
                    );
                    LspManagerModel::handle(ctx).update(ctx, |manager, ctx| {
                        manager.register(root.clone(), config, ctx);
                    });
                }
                LspManagerModel::handle(ctx).update(ctx, |manager, ctx| {
                    manager.spawn_servers_for_path(file_path, ctx);
                });
            },
        );
    }
}
