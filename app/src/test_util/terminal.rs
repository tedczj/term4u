use warp_core::ui::appearance::Appearance;
use warpui::{App, ViewHandle, WindowId};

use super::settings::initialize_history_persistence_for_tests;
use crate::changelog_model::ChangelogModel;
use crate::settings::PrivacySettings;
use crate::settings_view::keybindings::KeybindingChangedNotifier;
use crate::system::SystemStats;
use crate::terminal::mock_terminal_manager::MockTerminalManager;
use crate::terminal::model::SerializedBlockListItem;
use crate::terminal::resizable_data::ResizableData;
use crate::terminal::{History, TerminalView};
use crate::undo_close::UndoCloseStack;
use crate::vim_registers::VimRegisters;
use crate::workflows::local_workflows::LocalWorkflows;
use crate::workspace::sync_inputs::SyncedInputState;
use crate::workspace::{ActiveSession, WorkspaceRegistry};

/// Initializes the local models needed by terminal view tests.
pub fn initialize_app_for_terminal_view(app: &mut App) {
    initialize_history_persistence_for_tests(app);
    app.add_singleton_model(|_| Appearance::mock());
    app.update(PrivacySettings::register_singleton);

    app.add_singleton_model(|_| ChangelogModel::new(()));
    app.add_singleton_model(|_| SystemStats::new());
    app.add_singleton_model(|_| SyncedInputState::new());
    app.add_singleton_model(|_| ResizableData::default());
    app.add_singleton_model(LocalWorkflows::new);
    app.add_singleton_model(|_| History::default());
    app.add_singleton_model(UndoCloseStack::new);
    app.add_singleton_model(|_| KeybindingChangedNotifier::new());
    app.add_singleton_model(|_| ActiveSession::default());
    app.add_singleton_model(|_| WorkspaceRegistry::new());
    app.add_singleton_model(|_| crate::workspace::ToastStack);
    app.add_singleton_model(|_| VimRegisters::new());
    app.update(crate::terminal::init);
    app.update(crate::editor::init);
    app.update(crate::terminal::input::Input::init);
}

pub fn add_window_with_id_and_terminal(
    app: &mut App,
    restored_blocks: Option<&[SerializedBlockListItem]>,
) -> (WindowId, ViewHandle<TerminalView>) {
    let view = MockTerminalManager::create_new_terminal_view_window_for_test(app, restored_blocks);
    let window_id = app.read(|ctx| view.window_id(ctx));
    (window_id, view)
}

pub fn initialize_app_for_pane_group(app: &mut App) {
    initialize_app_for_terminal_view(app);
    app.add_singleton_model(|_| crate::terminal::local_tty::spawner::PtySpawner::new_for_test());
    app.add_singleton_model(crate::notebooks::manager::NotebookManager::new_local);
    app.add_singleton_model(crate::workflows::manager::WorkflowManager::new);
    app.add_singleton_model(repo_metadata::watcher::DirectoryWatcher::new);
    app.add_singleton_model(|_| repo_metadata::repositories::DetectedRepositories::default());
    app.add_singleton_model(watcher::HomeDirectoryWatcher::new_for_test);
    app.add_singleton_model(
        crate::warp_managed_paths_watcher::WarpManagedPathsWatcher::new_for_testing,
    );
    #[cfg(feature = "local_fs")]
    {
        app.add_singleton_model(crate::ImportedConfigModel::new);
        app.add_singleton_model(repo_metadata::RepoMetadataModel::new);
    }
    app.add_singleton_model(crate::search::files::model::FileSearchModel::new);
    app.add_singleton_model(|_| crate::code_review::git_repo_model::GitRepoModels::new());
    app.add_singleton_model(|_| crate::ai::persisted_workspace::PersistedWorkspace::new_for_test());
    app.add_singleton_model(|_| ai::project_context::model::ProjectContextModel::default());
    crate::terminal::available_shells::register(app);
}
