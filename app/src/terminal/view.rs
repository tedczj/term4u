mod action;
mod context_menu;
pub mod init;
mod link_detection;
mod local_io;
mod scroll;
use std::borrow::Cow;
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub use action::TerminalAction;
use async_channel::{Receiver, Sender};
use instant::Instant;
use local_io::NotificationKind;
use parking_lot::FairMutex;
use pathfinder_geometry::vector::Vector2F;
use sum_tree::SeekBias;
use vec1::vec1;
use warp_completer::meta::Span;
use warp_core::semantic_selection::SemanticSelection;
use warp_util::path::ShellFamily;
use warpui::clipboard::ClipboardContent;
use warpui::elements::{
    Align, ChildAnchor, ChildView, ClippedScrollStateHandle, ClippedScrollable, ConstrainedBox,
    Container, CrossAxisAlignment, DispatchEventResult, Empty, EventHandler, Expanded, Fill, Flex,
    MainAxisSize, OffsetPositioning, ParentAnchor, ParentElement, ParentOffsetBounds, SavePosition,
    ScrollbarWidth, Stack,
};
use warpui::ui_components::components::{UiComponent, UiComponentStyles};
use warpui::units::{IntoLines, IntoPixels, Lines};
use warpui::windowing::WindowManager;
use warpui::{
    AppContext, Element, Entity, EntityId, FocusContext, ModelHandle, SingletonEntity,
    TypedActionView, View, ViewContext, ViewHandle,
};

use super::alt_screen::alt_screen_element::AltScreenElement;
use super::alt_screen::{should_intercept_mouse, should_intercept_scroll};
use super::blockgrid_element::BlockGridElement;
use super::find::{BlockGridMatch, BlockListMatch, FindOptions, TerminalFindModel};
use super::grid_size_util::grid_cell_dimensions;
use super::input::{CommandExecutionSource, Input};
use super::model::ObfuscateSecrets;
use super::model::completions::ShellCompletion;
use super::model::escape_sequences::{ToEscapeSequence, alt_screen_scroll_to_pty_bytes};
use super::model::grid::grid_handler::Link;
use super::model::selection::SelectAction;
use super::model::session::{Session, SessionId, Sessions};
use super::model::terminal_model::TerminalInputState;
use super::model_events::{AnsiHandlerEvent, ModelEvent, ModelEventDispatcher};
use super::safe_mode_settings::get_secret_obfuscation_mode;
use super::terminal_size_element::TerminalSizeElement;
use super::{
    PtyIntent, PtyIntentEvent, ShellLaunchData, SizeInfo, SizeUpdate, SizeUpdateReason,
    TerminalModel, TerminalSurface,
};
use crate::appearance::Appearance;
use crate::code::buffer_location::LocalOrRemotePath;
use crate::code::editor_management::CodeSource;
use crate::menu::{Menu, MenuItem, MenuItemFields};
use crate::pane_group::focus_state::PaneFocusHandle;
use crate::pane_group::pane::view;
use crate::pane_group::{BackingView, PaneConfiguration, PaneEvent, SplitPaneState};
use crate::session_management::{CommandContext, SessionNavigationPromptElements};
use crate::settings::{AppEditorSettings, EnforceMinimumContrast};
use crate::terminal::event::BlockCompletedEvent;
use crate::terminal::input::Event as InputEvent;
use crate::terminal::model::block::SerializedBlock;
use crate::terminal::model::blocks::{
    BlockHeight, BlockHeightItem, BlockHeightSummary, BlockListPoint,
};
use crate::terminal::model::index::Point;
use crate::terminal::model::terminal_model::{BlockIndex, WithinBlock};
use crate::terminal::session_settings::SessionSettings;
use crate::terminal::shell::ShellType;
use crate::terminal::{GridType, context_menu_offset, should_right_click_paste};
use crate::throttle::throttle;
use crate::util::openable_file_type::{EditorLayout, FileTarget};
use crate::view_components::find::{Event as FindViewEvent, Find, FindWithinBlockState};
use crate::workspace::CommandSearchOptions;

pub const WAKEUP_THROTTLE_PERIOD: std::time::Duration = std::time::Duration::from_millis(16);

#[derive(Clone)]
pub struct ExecuteCommandEvent {
    pub command: String,
    pub session_id: SessionId,
    pub workflow_command: Option<String>,
    pub should_add_command_to_history: bool,
    pub source: CommandExecutionSource,
}

pub enum Event {
    AppStateChanged,
    Exited,
    BlockListCleared,
    BlockCompleted {
        block: Arc<SerializedBlock>,
        is_local: bool,
    },
    Pane(PaneEvent),
    SyncInput(SyncEvent),
    ShowCommandSearch(CommandSearchOptions),
    CtrlD,
    InterruptPty,
    ShutdownPty,
    WriteBytesToPty {
        bytes: Cow<'static, [u8]>,
    },
    ClipboardResponse(warp_terminal::writeable_pty::ClipboardResponse),
    Resize {
        size_update: SizeUpdate,
    },
    ExecuteCommand(ExecuteCommandEvent),
    BlockStarted {
        is_for_in_band_command: bool,
    },
    FocusSession,
    SessionBootstrapped,
    ShellSpawned(ShellType),
    PtySpawnFailed {
        reason: String,
    },
    OpenFileInWarp {
        path: std::path::PathBuf,
        session: Arc<Session>,
    },
    #[cfg(feature = "local_fs")]
    OpenCodeInWarp {
        source: CodeSource,
        layout: EditorLayout,
    },
    #[cfg(feature = "local_fs")]
    OpenFileWithTarget {
        path: std::path::PathBuf,
        target: FileTarget,
        line_col: Option<warp_util::path::LineAndColumnArg>,
    },
    RunNativeShellCompletions {
        buffer_text: String,
        results_tx: async_channel::Sender<(Vec<ShellCompletion>, Option<Span>)>,
    },
}

impl PtyIntentEvent for Event {
    fn pty_intent(&self) -> Option<PtyIntent> {
        match self {
            Event::CtrlD => Some(PtyIntent::CtrlD),
            #[cfg(not(target_family = "wasm"))]
            Event::InterruptPty => Some(PtyIntent::Interrupt),
            #[cfg(target_family = "wasm")]
            Event::InterruptPty => None,
            Event::ShutdownPty => Some(PtyIntent::ShutdownPty),
            Event::WriteBytesToPty { bytes } => Some(PtyIntent::WriteBytes(bytes.clone())),
            Event::ClipboardResponse(response) => {
                Some(PtyIntent::ClipboardResponse(response.clone()))
            }
            Event::Resize { size_update } => Some(PtyIntent::Resize(*size_update)),
            Event::ExecuteCommand(event) => Some(PtyIntent::ExecuteCommand(event.clone())),
            Event::RunNativeShellCompletions {
                buffer_text,
                results_tx,
            } => Some(PtyIntent::RunNativeShellCompletions {
                buffer_text: buffer_text.clone(),
                results_tx: results_tx.clone(),
            }),
            Event::AppStateChanged
            | Event::Exited
            | Event::BlockListCleared
            | Event::BlockCompleted { .. }
            | Event::Pane(_)
            | Event::SyncInput(_)
            | Event::ShowCommandSearch(_)
            | Event::BlockStarted { .. }
            | Event::FocusSession
            | Event::SessionBootstrapped
            | Event::ShellSpawned(_)
            | Event::PtySpawnFailed { .. }
            | Event::OpenFileInWarp { .. }
            | Event::OpenFileWithTarget { .. }
            | Event::OpenCodeInWarp { .. } => None,
        }
    }
}

#[derive(Clone)]
pub struct SyncEvent {
    pub source_view_id: EntityId,
    pub data: SyncInputType,
}

#[derive(Clone)]
pub enum SyncInputType {
    InputEditorContentsChanged { contents: Arc<String> },
    NonEditorTyped { chars: Arc<Vec<u8>> },
    RanCommand,
    StartSyncing,
    StopSyncing,
}

#[derive(Debug, Clone, Copy)]
pub enum TerminalEditor {
    Yes,
    No,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveSessionState {
    Active,
    Inactive,
}

pub struct TerminalViewRenderContext {
    pub size_info: SizeInfo,
    pub highlighted_url: Option<Link>,
    pub link_tool_tip: Option<Link>,
    pub is_terminal_focused: bool,
    pub cursor_blink_epoch: Option<Instant>,
    pub selection_dragging: Arc<AtomicBool>,
    pub pane_state: SplitPaneState,
    pub active_session_state: ActiveSessionState,
    pub terminal_view_id: EntityId,
    pub hovered_secret: Option<super::model::SecretHandle>,
    pub obfuscate_secrets: ObfuscateSecrets,
}

pub struct TerminalView {
    view_id: EntityId,
    pub model: Arc<FairMutex<TerminalModel>>,
    input: ViewHandle<Input>,
    size_info: SizeInfo,
    resize_tx: Sender<Vector2F>,
    transcript_scroll: ClippedScrollStateHandle,
    transcript_height: f32,
    reading_position: RefCell<Option<scroll::ScrollSnapshot>>,
    scroll_after_clear: bool,
    find_model: ModelHandle<TerminalFindModel>,
    find_bar: ViewHandle<Find<TerminalFindModel>>,
    find_bar_open: bool,
    find_options: FindOptions,
    find_selected_blocks: Vec<BlockIndex>,
    pane_configuration: ModelHandle<PaneConfiguration>,
    focus_handle: Option<PaneFocusHandle>,
    sessions: ModelHandle<Sessions>,
    model_events: ModelHandle<ModelEventDispatcher>,
    active_shell_launch_data: Option<ShellLaunchData>,
    current_repo_path: Option<LocalOrRemotePath>,
    pty_spawn_error: Option<String>,
    is_selecting: bool,
    output_dragging: Arc<AtomicBool>,
    pending_commands: Vec<String>,
    is_bootstrapped: bool,
    was_ever_visible: bool,
    context_menu: ViewHandle<Menu<TerminalAction>>,
    context_menu_state: Option<ContextMenuState>,
    context_menu_generation: u64,
    context_menu_return_to_find: bool,
    position_id: String,
    local_io: local_io::LocalIoState,
    file_drop_active: bool,
    links: link_detection::LinkState,
}

/// Which part of a block the transcript context menu copies.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum BlockTextPart {
    Command,
    Output,
    Both,
}

/// Tracks the open transcript/alt-screen context menu. `position` is relative to the
/// terminal view's own bounds, so the menu follows the click instead of the window.
struct ContextMenuState {
    position: Vector2F,
}

impl TerminalView {
    pub fn new(
        wakeups_rx: Receiver<()>,
        model_events: ModelHandle<ModelEventDispatcher>,
        model: Arc<FairMutex<TerminalModel>>,
        sessions: ModelHandle<Sessions>,
        size_info: SizeInfo,
        ctx: &mut ViewContext<Self>,
    ) -> Self {
        model.lock().block_list_mut().set_next_gap_height_in_lines(
            (size_info.pane_height_px as f64 / size_info.cell_height_px().as_f32() as f64)
                .into_lines(),
        );
        let mut clipboard_access =
            *super::settings::TerminalSettings::as_ref(ctx).osc52_clipboard_access;
        ctx.subscribe_to_model(
            &super::settings::TerminalSettings::handle(ctx),
            move |view, _, _, ctx| {
                let access = *super::settings::TerminalSettings::as_ref(ctx).osc52_clipboard_access;
                if clipboard_access != access {
                    clipboard_access = access;
                    view.model
                        .lock()
                        .event_proxy
                        .invalidate_clipboard_requests();
                }
            },
        );
        let input = ctx.add_typed_action_view(Input::new);
        ctx.subscribe_to_view(&input, |view, _, event, ctx| {
            view.handle_input_event(event, ctx)
        });
        let find_model = ctx.add_model(|ctx| TerminalFindModel::new(model.clone(), ctx));
        let find_bar = ctx.add_typed_action_view(|ctx| Find::new(find_model.clone(), ctx));
        ctx.subscribe_to_view(&find_bar, |view, _, event, ctx| {
            view.handle_find_event(event, ctx)
        });
        ctx.subscribe_to_model(&find_model, |view, _, _, ctx| {
            let matched = match view.find_model.as_ref(ctx).focused_block_list_match() {
                Some(BlockListMatch::CommandBlock(matched)) => Some(matched),
                Some(BlockListMatch::RichContent { .. }) | None => None,
            };
            view.handle_wakeup(matched, ctx);
        });
        let context_menu = ctx.add_typed_action_view(|_| {
            Menu::new()
                .prevent_interaction_with_other_elements()
                .with_drop_shadow()
        });
        ctx.subscribe_to_view(&context_menu, |view, _, event, ctx| {
            if let crate::menu::Event::Close { .. } = event {
                view.close_context_menu(ctx);
            }
            ctx.notify();
        });
        let pane_configuration = ctx.add_model(|_| PaneConfiguration::new("Terminal"));
        ctx.subscribe_to_model(&model_events, |view, _, event, ctx| {
            view.handle_model_event(event, ctx)
        });
        ctx.spawn_stream_local(
            throttle(WAKEUP_THROTTLE_PERIOD, wakeups_rx),
            |view, _, ctx| view.handle_wakeup(None, ctx),
            |_, _| {},
        );
        ctx.subscribe_to_model(&AppEditorSettings::handle(ctx), |view, _, _, ctx| {
            view.local_io.reset_cursor_blink();
            ctx.notify();
        });
        ctx.subscribe_to_model(&WindowManager::handle(ctx), |view, _, _, ctx| {
            view.local_io.reset_cursor_blink();
            ctx.notify();
        });
        let (resize_tx, resize_rx) = async_channel::unbounded();
        ctx.spawn_stream_local(resize_rx, Self::after_layout, |_, _| {});
        Self {
            view_id: ctx.view_id(),
            model,
            input,
            size_info,
            resize_tx,
            transcript_scroll: ClippedScrollStateHandle::default(),
            transcript_height: 0.,
            reading_position: RefCell::new(None),
            scroll_after_clear: false,
            find_model,
            find_bar,
            find_bar_open: false,
            find_options: FindOptions::default(),
            find_selected_blocks: Vec::new(),
            pane_configuration,
            focus_handle: None,
            sessions,
            model_events,
            active_shell_launch_data: None,
            current_repo_path: None,
            pty_spawn_error: None,
            is_selecting: false,
            output_dragging: Arc::new(AtomicBool::new(false)),
            pending_commands: Vec::new(),
            is_bootstrapped: false,
            was_ever_visible: false,
            context_menu,
            context_menu_state: None,
            context_menu_generation: 0,
            context_menu_return_to_find: false,
            position_id: format!("terminal_content_{}", ctx.view_id()),
            local_io: local_io::LocalIoState::default(),
            file_drop_active: false,
            links: link_detection::LinkState::default(),
        }
    }

    pub fn input(&self) -> &ViewHandle<Input> {
        &self.input
    }

    pub fn id(&self) -> EntityId {
        self.input.id()
    }

    pub fn pane_configuration(&self) -> &ModelHandle<PaneConfiguration> {
        &self.pane_configuration
    }

    pub fn current_repo_path(&self) -> Option<&LocalOrRemotePath> {
        self.current_repo_path.as_ref()
    }

    pub fn current_local_repo_path(&self) -> Option<&std::path::Path> {
        self.current_repo_path
            .as_ref()
            .and_then(LocalOrRemotePath::to_local_path)
    }

    pub fn shell_family(&self, ctx: &AppContext) -> ShellFamily {
        self.active_session(ctx)
            .map(|session| ShellFamily::from(session.shell().shell_type()))
            .unwrap_or(ShellFamily::Posix)
    }

    pub fn active_session(&self, ctx: &AppContext) -> Option<Arc<Session>> {
        let id = self.model_events.as_ref(ctx).active_session_id()?;
        self.sessions.as_ref(ctx).get(id)
    }

    pub fn sessions_model(&self) -> &ModelHandle<Sessions> {
        &self.sessions
    }

    pub fn active_block_session_id(&self) -> Option<SessionId> {
        self.model.lock().block_list().active_block().session_id()
    }

    pub fn active_session_is_local(&self, ctx: &AppContext) -> Option<bool> {
        self.active_session(ctx).map(|session| session.is_local())
    }

    pub fn active_session_wsl_distro(&self, ctx: &AppContext) -> Option<String> {
        self.active_session(ctx)
            .and_then(|session| session.wsl_distro_name().map(str::to_owned))
    }

    pub fn active_session_path_if_local(&self, ctx: &AppContext) -> Option<std::path::PathBuf> {
        if self.active_session_is_local(ctx) != Some(true) {
            return None;
        }
        let path = std::path::PathBuf::from(self.current_working_directory(ctx)?);
        path.is_dir().then_some(path)
    }

    pub fn canonical_session_pwd_if_local(&self, ctx: &AppContext) -> Option<std::path::PathBuf> {
        let path = self.active_session_path_if_local(ctx)?;
        dunce::canonicalize(path).ok()
    }

    pub fn pwd_as_local_or_remote(&self, ctx: &AppContext) -> Option<LocalOrRemotePath> {
        self.canonical_session_pwd_if_local(ctx)
            .map(LocalOrRemotePath::Local)
    }

    pub fn current_working_directory(&self, _app: &AppContext) -> Option<String> {
        self.model.lock().block_list().active_block().pwd().cloned()
    }

    pub fn active_shell_launch_data(&self) -> Option<ShellLaunchData> {
        self.active_shell_launch_data.clone()
    }

    pub fn pwd(&self) -> Option<String> {
        self.model.lock().block_list().active_block().pwd().cloned()
    }

    pub fn display_working_directory(&self, app: &AppContext) -> Option<String> {
        self.current_working_directory(app)
    }

    /// Keeps the block's origin attached to snapshots after its live session has gone away.
    pub(crate) fn serialize_block(
        &self,
        block: &super::model::block::Block,
        app: &AppContext,
    ) -> SerializedBlock {
        let mut serialized = SerializedBlock::from(block);
        serialized.is_local = block
            .session_id()
            .and_then(|id| self.sessions.as_ref(app).get(id))
            .map(|session| session.is_local())
            .or(block.restored_block_was_local());
        serialized
    }

    pub fn selected_text_from_input(&self, app: &AppContext) -> Option<String> {
        let text = self
            .input
            .as_ref(app)
            .editor()
            .as_ref(app)
            .selected_text(app);
        (!text.is_empty()).then_some(text)
    }

    pub fn selected_text(&self, app: &AppContext) -> Option<String> {
        self.model
            .lock()
            .selection_to_string(SemanticSelection::as_ref(app), false)
    }

    pub fn is_long_running(&self) -> bool {
        self.model
            .lock()
            .block_list()
            .active_block()
            .is_active_and_long_running()
    }

    pub fn set_pending_command_queue(
        &mut self,
        commands: Vec<String>,
        ctx: &mut ViewContext<Self>,
    ) {
        self.pending_commands = commands;
        self.run_pending_command(ctx);
    }

    fn run_pending_command(&mut self, ctx: &mut ViewContext<Self>) {
        if self.is_bootstrapped
            && let Some(command) = self.pending_commands.first().cloned()
        {
            self.input
                .update(ctx, |input, ctx| input.set_pending_command(&command, ctx));
            self.pending_commands.remove(0);
        }
    }

    pub fn clear_orchestration_split_off(&mut self, _: &mut ViewContext<Self>) {}

    pub fn has_highlighted_link(&self) -> bool {
        self.links.highlighted.is_some()
    }

    pub fn mark_as_visible(&mut self) {
        self.was_ever_visible = true;
    }

    pub fn was_ever_visible(&self) -> bool {
        self.was_ever_visible
    }

    pub fn size_info(&self) -> SizeInfo {
        self.size_info
    }

    pub fn dismiss_tooltips(&mut self, ctx: &mut ViewContext<Self>) {
        self.clear_link(ctx);
    }

    pub fn shell_indicator_type(&self) -> Option<crate::shell_indicator::ShellIndicatorType> {
        self.active_shell_launch_data
            .as_ref()
            .and_then(|launch_data| launch_data.try_into().ok())
    }

    pub fn full_prompt(&self, app: &AppContext) -> String {
        let model = self.model.lock();
        let Some(block) = model.prompt_block() else {
            return String::new();
        };
        let session = block
            .session_id()
            .and_then(|id| self.sessions.as_ref(app).get(id));
        let directory = super::prompt::display_path_string(
            block.pwd(),
            session.as_ref().and_then(|session| session.home_dir()),
        );
        let identity = session
            .as_ref()
            .map(|session| {
                format!(
                    "{}@{} ",
                    session.user(),
                    session
                        .hostname()
                        .split('.')
                        .next()
                        .unwrap_or(session.hostname())
                )
            })
            .unwrap_or_default();
        let branch = block
            .git_branch()
            .filter(|branch| !branch.is_empty())
            .map(|branch| format!(" ({branch})"))
            .unwrap_or_default();
        format!("{identity}{directory}{branch} %")
    }

    pub fn prompt_elements(&self, _app: &AppContext) -> SessionNavigationPromptElements {
        let model = self.model.lock();
        SessionNavigationPromptElements {
            ps1_prompt_grid: model
                .prompt_block()
                .filter(|block| block.honor_ps1())
                .map(|block| block.prompt_grid().clone()),
        }
    }

    fn render_input_prompt(&self, app: &AppContext) -> Box<dyn Element> {
        if let Some(grid) = self.prompt_elements(app).ps1_prompt_grid
            && !grid.is_empty()
        {
            return BlockGridElement::new(
                &grid,
                Appearance::as_ref(app),
                EnforceMinimumContrast::default(),
                get_secret_obfuscation_mode(app),
                self.size_info,
            )
            .finish();
        }
        let appearance = Appearance::as_ref(app);
        Container::new(
            appearance
                .ui_builder()
                .paragraph(self.full_prompt(app))
                .with_style(UiComponentStyles {
                    font_family_id: Some(appearance.monospace_font_family()),
                    font_size: Some(appearance.monospace_font_size()),
                    ..Default::default()
                })
                .build()
                .finish(),
        )
        .with_padding_left(8.)
        .with_padding_right(8.)
        .finish()
    }

    pub fn session_command_context(&self, _app: &AppContext) -> CommandContext {
        let model = self.model.lock();
        let active = model.block_list().active_block();
        if active.is_active_and_long_running() {
            return CommandContext::RunningCommand {
                running_command: active.command_to_string(),
            };
        }
        model
            .block_list()
            .blocks()
            .iter()
            .rev()
            .find(|block| block.finished() && !block.command_to_string().is_empty())
            .map_or(CommandContext::None, |block| {
                CommandContext::LastRunCommand {
                    last_run_command: block.command_to_string(),
                    mins_since_completion: None,
                }
            })
    }

    pub fn last_focus_ts(&self) -> Option<chrono::NaiveDateTime> {
        None
    }

    pub fn create_sync_event_based_on_terminal_state(&self, app: &AppContext) -> SyncEvent {
        SyncEvent {
            source_view_id: self.input.id(),
            data: SyncInputType::InputEditorContentsChanged {
                contents: Arc::new(self.input.as_ref(app).buffer_text(app)),
            },
        }
    }

    pub fn receive_sync_input_event(&mut self, event: &SyncEvent, ctx: &mut ViewContext<Self>) {
        if event.source_view_id == self.input.id() {
            return;
        }
        match &event.data {
            SyncInputType::InputEditorContentsChanged { contents } => {
                self.input.update(ctx, |input, ctx| {
                    input.send_input_buffer_to_terminal_editor(contents.clone(), ctx)
                });
            }
            SyncInputType::NonEditorTyped { chars } => {
                ctx.emit(Event::WriteBytesToPty {
                    bytes: Cow::Owned(chars.as_ref().clone()),
                });
            }
            SyncInputType::RanCommand => {
                self.input.update(ctx, |input, ctx| {
                    input.run_command_in_synced_terminal_input(ctx)
                });
            }
            SyncInputType::StartSyncing | SyncInputType::StopSyncing => {}
        }
    }

    pub fn focus(&mut self, ctx: &mut ViewContext<Self>) {
        self.local_io.reset_cursor_blink();
        if self.context_menu_state.is_some() {
            ctx.focus(&self.context_menu);
            return;
        }
        let model = self.model.lock();
        let focus_input = matches!(
            model.terminal_input_state(),
            TerminalInputState::InputEditor | TerminalInputState::NotBootstrapped
        ) && model.block_list().selection().is_none();
        drop(model);
        if focus_input {
            self.input
                .update(ctx, |input, ctx| input.focus_input_box(ctx));
        } else {
            ctx.focus_self();
        }
        ctx.emit(Event::FocusSession);
        ctx.notify();
    }

    pub fn clear_buffer(&mut self, ctx: &mut ViewContext<Self>) {
        self.input.update(ctx, |input, ctx| {
            input.clear_buffer_and_reset_undo_stack(ctx)
        });
    }

    pub fn install_focus_handle(
        &mut self,
        focus_handle: PaneFocusHandle,
        ctx: &mut ViewContext<Self>,
    ) {
        self.focus_handle = Some(focus_handle);
        ctx.notify();
    }

    fn input_is_visible(&self) -> bool {
        matches!(
            self.model.lock().terminal_input_state(),
            TerminalInputState::InputEditor | TerminalInputState::NotBootstrapped
        )
    }

    fn handle_input_event(&mut self, event: &InputEvent, ctx: &mut ViewContext<Self>) {
        match event {
            InputEvent::ExecuteCommand(command) => {
                if !self.is_bootstrapped {
                    self.pending_commands.push(command.clone());
                    return;
                }
                self.transcript_scroll
                    .scroll_to(self.transcript_height.into_pixels());
                let session_id = self
                    .model_events
                    .as_ref(ctx)
                    .active_session_id()
                    .unwrap_or_default();
                ctx.emit(Event::ExecuteCommand(ExecuteCommandEvent {
                    command: command.clone(),
                    session_id,
                    workflow_command: None,
                    should_add_command_to_history: true,
                    source: CommandExecutionSource::User,
                }));
                ctx.emit(Event::SyncInput(SyncEvent {
                    source_view_id: self.input.id(),
                    data: SyncInputType::RanCommand,
                }));
            }
            InputEvent::Complete => {
                let session = self.active_session(ctx);
                let directory = self
                    .model
                    .lock()
                    .prompt_block()
                    .and_then(|block| block.pwd().cloned());
                if let (Some(session), Some(directory)) = (session, directory) {
                    let completion_context = crate::completer::SessionContext::for_terminal(
                        session,
                        typed_path::TypedPathBuf::from(directory),
                    );
                    self.input
                        .update(ctx, |input, ctx| input.complete(completion_context, ctx));
                }
            }
            InputEvent::NavigateHistory { previous } => {
                if let Some(session) = self.active_session(ctx) {
                    let commands = super::History::as_ref(ctx)
                        .commands(session.id())
                        .unwrap_or_default()
                        .into_iter()
                        .map(|entry| entry.command.clone())
                        .collect();
                    self.input.update(ctx, |input, ctx| {
                        input.navigate_history(session.id(), commands, *previous, ctx);
                    });
                }
            }
            InputEvent::CtrlC { .. } => ctx.emit(Event::InterruptPty),
            InputEvent::CtrlD => ctx.emit(Event::CtrlD),
            InputEvent::EditorFocused => {
                self.model.lock().block_list_mut().clear_selection();
                ctx.notify();
            }
        }
    }

    fn handle_model_event(&mut self, event: &ModelEvent, ctx: &mut ViewContext<Self>) {
        self.invalidate_changed_link(ctx);
        match event {
            ModelEvent::Handler(AnsiHandlerEvent::Bootstrapped { .. }) => {
                self.input
                    .update(ctx, |input, _| input.invalidate_async_state());
                self.invalidate_pending_paste();
                self.is_bootstrapped = true;
                self.run_pending_command(ctx);
                ctx.emit(Event::SessionBootstrapped);
            }
            ModelEvent::Title(title) => {
                self.pane_configuration.update(ctx, |configuration, ctx| {
                    configuration.set_title(title.clone(), ctx)
                });
            }
            ModelEvent::BlockCompleted(BlockCompletedEvent {
                block_id,
                block_type,
                ..
            }) => {
                let completed = {
                    let model = self.model.lock();
                    model.block_list().block_with_id(block_id).map(|block| {
                        (
                            Arc::new(self.serialize_block(block, ctx)),
                            block.duration().and_then(|duration| duration.to_std().ok()),
                        )
                    })
                };
                if let Some((block, duration)) = completed {
                    if matches!(block_type, super::event::BlockType::User(_))
                        && duration.is_some_and(|duration| {
                            duration
                                >= SessionSettings::as_ref(ctx)
                                    .notifications
                                    .long_running_threshold
                        })
                    {
                        self.send_local_notification(
                            Some("Command completed"),
                            "A background command has finished.",
                            NotificationKind::CommandCompleted,
                            ctx,
                        );
                    }
                    ctx.emit(Event::BlockCompleted {
                        is_local: block.is_local.unwrap_or(false),
                        block,
                    });
                }
                if self.context_menu_state.is_none()
                    && !self.find_bar_open
                    && ctx.is_self_or_child_focused()
                {
                    self.focus(ctx);
                }
                self.run_pending_command(ctx);
            }
            ModelEvent::AfterBlockStarted {
                is_for_in_band_command,
                ..
            } => ctx.emit(Event::BlockStarted {
                is_for_in_band_command: *is_for_in_band_command,
            }),
            ModelEvent::TerminalClear => {
                self.scroll_after_clear = true;
                self.handle_wakeup(None, ctx);
                self.transcript_scroll
                    .scroll_to(self.transcript_height.into_pixels());
                ctx.emit(Event::BlockListCleared);
            }
            ModelEvent::Exit { .. } => {
                self.input
                    .update(ctx, |input, _| input.invalidate_async_state());
                self.invalidate_pending_paste();
                self.model.lock().clear_marked_text();
                ctx.emit(Event::Exited);
            }
            ModelEvent::ClipboardStore(selection, text, request) => {
                let current = self.model.lock().clipboard_request_is_current(request);
                if current {
                    self.store_terminal_clipboard(*selection, text, ctx);
                }
            }
            ModelEvent::ClipboardLoad(selection, encode, request) => {
                let current = self.model.lock().clipboard_request_is_current(request);
                if current {
                    self.load_terminal_clipboard(*selection, encode.as_ref(), request, ctx);
                }
            }
            ModelEvent::CursorBlinkingChange(_) => {
                self.local_io.reset_cursor_blink();
                ctx.notify();
            }
            ModelEvent::Bell => self.ring_local_bell(ctx),
            ModelEvent::PluggableNotification { title, body } => {
                self.send_local_notification(
                    title.as_deref(),
                    body,
                    NotificationKind::Attention,
                    ctx,
                );
            }
            ModelEvent::BlockMetadataReceived(_)
            | ModelEvent::BlockWorkingDirectoryUpdated(_)
            | ModelEvent::BootstrapPrecmdDone => {
                ctx.emit(Event::AppStateChanged);
                ctx.notify();
            }
            ModelEvent::ShellSpawned(_) | ModelEvent::ExitShell { .. } => {
                self.input
                    .update(ctx, |input, _| input.invalidate_async_state());
                self.invalidate_pending_paste();
                self.model.lock().clear_marked_text();
                ctx.notify();
            }
            ModelEvent::Handler(
                AnsiHandlerEvent::SetBracketedPaste | AnsiHandlerEvent::UnsetBracketedPaste,
            )
            | ModelEvent::TerminalModeSwapped(_) => {
                self.invalidate_pending_paste();
                self.model.lock().clear_marked_text();
                ctx.notify();
            }
            ModelEvent::Handler(_)
            | ModelEvent::AfterBlockCompleted(_)
            | ModelEvent::BackgroundBlockStarted
            | ModelEvent::VisibleBootstrapBlock
            | ModelEvent::PromptUpdated
            | ModelEvent::HonorPS1OutOfSync
            | ModelEvent::SelectedTextChanged
            | ModelEvent::ImageReceived { .. }
            | ModelEvent::AgentTaggedInChanged { .. }
            | ModelEvent::FinishUpdate(_)
            | ModelEvent::Typeahead
            | ModelEvent::CompletionsFinished(_, _)
            | ModelEvent::MouseCursorDirty
            | ModelEvent::ExecutedInBandCommand(_)
            | ModelEvent::DetectedEndOfSshLogin(_)
            | ModelEvent::InitSubshell(_)
            | ModelEvent::SourcedRcFileInSubshell(_)
            | ModelEvent::PreInteractiveSSHSession
            | ModelEvent::SSH(_)
            | ModelEvent::SSHControlMasterError => ctx.notify(),
        }
    }

    fn handle_find_event(&mut self, event: &FindViewEvent, ctx: &mut ViewContext<Self>) {
        match event {
            FindViewEvent::CloseFindBar => {
                self.find_bar_open = false;
                self.find_model
                    .update(ctx, |model, ctx| model.clear_matches(ctx));
                self.focus(ctx);
                ctx.notify();
                return;
            }
            FindViewEvent::NextMatch { direction } => {
                self.find_model.update(ctx, |model, ctx| {
                    model.focus_next_find_match(*direction, ctx)
                });
                return;
            }
            FindViewEvent::Update { query } => {
                self.find_options.query = query.clone().map(Arc::new)
            }
            FindViewEvent::ToggleCaseSensitivity { is_case_sensitive } => {
                self.find_options.is_case_sensitive = *is_case_sensitive
            }
            FindViewEvent::ToggleRegexSearch { is_regex_enabled } => {
                self.find_options.is_regex_enabled = *is_regex_enabled
            }
            FindViewEvent::ToggleFindInBlock { value } => {
                self.find_options.blocks_to_include_in_results =
                    value.then(|| self.find_selected_blocks.clone())
            }
        }
        self.find_model.update(ctx, |model, ctx| {
            model.run_find(self.find_options.clone(), ctx)
        });
    }

    fn handle_wakeup(&mut self, find_match: Option<BlockGridMatch>, ctx: &mut ViewContext<Self>) {
        self.invalidate_changed_link(ctx);
        let model_handle = self.model.clone();
        let mut model = model_handle.lock();
        let show_input = matches!(
            model.terminal_input_state(),
            TerminalInputState::InputEditor | TerminalInputState::NotBootstrapped
        );
        let has_output_selection = model.block_list().selection().is_some();
        if !self.find_bar_open {
            self.find_selected_blocks = model
                .block_list()
                .text_selection_range(SemanticSelection::as_ref(ctx), false)
                .map(|(start, end, _)| {
                    (start.block_index.0.min(end.block_index.0)
                        ..=start.block_index.0.max(end.block_index.0))
                        .map(BlockIndex)
                        .collect()
                })
                .unwrap_or_default();
        }
        if !model.is_alt_screen_active() {
            let scroll = self.capture_scroll(&model);
            model.block_list_mut().update_background_block_height();
            model.block_list_mut().update_active_block_height();
            self.restore_scroll(scroll, &model);

            let find_row = find_match.as_ref().and_then(|matched| {
                let block_list = model.block_list();
                let block = block_list.block_at(matched.block_index)?;
                let grid = match matched.grid_type {
                    GridType::Output => block.output_grid(),
                    GridType::PromptAndCommand => block.prompt_and_command_grid(),
                    GridType::Prompt | GridType::Rprompt => return None,
                };
                let point = grid
                    .grid_handler()
                    .maybe_translate_point_from_original_to_displayed(*matched.range.start());
                Some(
                    BlockListPoint::from_within_block_point(
                        &WithinBlock::new(point, matched.block_index, matched.grid_type),
                        block_list,
                    )
                    .row,
                )
            });
            if let Some(row) = find_row {
                let offset = (row.as_f64() as f32 * self.size_info.cell_height_px().as_f32()
                    - self.size_info.pane_height_px / 2.)
                    .max(0.);
                self.transcript_scroll.scroll_to(offset.into_pixels());
                self.remember_scroll(&model);
            }
        }
        drop(model);
        if !show_input && self.input.as_ref(ctx).editor().is_focused(ctx) {
            log::debug!("Moving keyboard focus from command editor to terminal");
            ctx.focus_self();
        } else if show_input && !has_output_selection && ctx.is_self_focused() {
            self.input
                .update(ctx, |input, ctx| input.focus_input_box(ctx));
        }
        ctx.notify();
    }

    fn after_layout(&mut self, size: Vector2F, ctx: &mut ViewContext<Self>) {
        if self.scroll_after_clear {
            self.scroll_after_clear = false;
            self.transcript_scroll
                .scroll_to(self.transcript_height.into_pixels());
            ctx.notify();
        }
        let cell_size = if ctx.is_headless() {
            Vector2F::new(
                self.size_info.cell_width_px().as_f32(),
                self.size_info.cell_height_px().as_f32(),
            )
        } else {
            let appearance = Appearance::as_ref(ctx);
            grid_cell_dimensions(
                ctx.font_cache(),
                appearance.monospace_font_family(),
                appearance.monospace_font_size(),
                appearance.ui_builder().line_height_ratio(),
            )
        };
        let new_size = SizeInfo::new(
            size,
            cell_size.x().into_pixels(),
            cell_size.y().into_pixels(),
            self.size_info.padding_x_px(),
            self.size_info.padding_y_px(),
        );
        if new_size == self.size_info {
            return;
        }
        self.clear_link(ctx);
        let update = SizeUpdate {
            update_reason: SizeUpdateReason::AfterLayout,
            last_size: self.size_info,
            new_size,
            new_gap_height: Some(
                (new_size.pane_height_px as f64 / cell_size.y() as f64).into_lines(),
            ),
            natural_rows: new_size.rows(),
            natural_cols: new_size.columns(),
        };
        {
            let model_handle = self.model.clone();
            let mut model = model_handle.lock();
            let mut scroll = self.capture_scroll(&model);
            scroll.prepare_resize(&mut model);
            model
                .block_list_mut()
                .set_next_gap_height_in_lines(update.new_gap_height.unwrap());
            model.resize(update);
            scroll.finish_resize(&mut model);
            self.size_info = new_size;
            self.restore_scroll(scroll, &model);
        }
        ctx.emit(Event::Resize {
            size_update: update,
        });
        ctx.notify();
    }

    fn render_blocks(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let model = self.model.lock();
        if self
            .reading_position
            .borrow()
            .as_ref()
            .is_none_or(|snapshot| !snapshot.matches_scroll(self))
        {
            self.remember_scroll(&model);
        }
        let receives_native_input = !matches!(
            model.terminal_input_state(),
            TerminalInputState::InputEditor | TerminalInputState::NotBootstrapped
        ) && app.focused_view_id(self.input.window_id(app))
            == Some(self.view_id);

        let ranges = model
            .block_list()
            .renderable_selection(SemanticSelection::as_ref(app), false)
            .map(|ranges| ranges.into_iter().collect::<Vec<_>>())
            .unwrap_or_default();
        let mut column = Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);
        let cell_height = self.size_info.cell_height_px().as_f32();
        let tree = model.block_list().block_heights();
        let total_rows = tree.summary().height.as_f64();
        let viewport_rows = self.size_info.pane_height_px as f64 / cell_height as f64;
        let scroll_row = (self.transcript_scroll.scroll_start().as_f32() as f64
            / cell_height as f64)
            .min((total_rows - viewport_rows).max(0.));
        let start_row = (scroll_row - 3.).max(0.);
        let end_row = scroll_row + viewport_rows + 3.;
        let mut cursor = tree.cursor::<BlockHeight, BlockHeightSummary>();
        cursor.seek(&start_row.into(), SeekBias::Right);
        let mut rendered_to = 0.;
        while let Some(item) = cursor.item() {
            let top = cursor.start().height.as_f64();
            if top >= end_row {
                break;
            }
            let bottom = top + item.height().as_f64();
            let index = cursor.start().block_count;
            #[cfg(test)]
            viewport_tests::WORK.with(|work| work.set((work.get().0 + 1, work.get().1)));
            // Seeking by height skips arbitrarily long runs of hidden (zero-height) blocks.
            cursor.seek(&bottom.into(), SeekBias::Right);
            if !matches!(item, BlockHeightItem::Block(_)) {
                continue;
            }
            let block = &model.block_list().blocks()[index];
            let find = self.find_model.as_ref(app).find_render_data_for_block(
                BlockIndex(index),
                Some(block.prompt_and_command_grid().grid_handler()),
                Some(block.output_grid().grid_handler()),
            );
            for (hidden, grid, grid_type, grid_offset, grid_height) in [
                (
                    block.should_hide_command_grid(),
                    block.prompt_and_command_grid(),
                    GridType::PromptAndCommand,
                    block.prompt_and_command_grid_offset(),
                    block.prompt_and_command_grid().len_displayed().into_lines(),
                ),
                (
                    block.should_hide_output_grid(),
                    block.output_grid(),
                    GridType::Output,
                    block.output_grid_offset(),
                    block.output_grid_displayed_height(),
                ),
            ] {
                let grid_top = top + grid_offset.as_f64();
                let grid_rows = grid_height.as_f64().min((bottom - grid_top).max(0.));
                if !hidden
                    && grid_rows > 0.
                    && grid_top < end_row
                    && grid_top + grid_rows > start_row
                {
                    if grid_top > rendered_to {
                        column.add_child(
                            ConstrainedBox::new(Empty::new().finish())
                                .with_height((grid_top - rendered_to) as f32 * cell_height)
                                .finish(),
                        );
                    }
                    rendered_to = grid_top + grid_rows;
                    #[cfg(test)]
                    viewport_tests::WORK.with(|work| work.set((work.get().0, work.get().1 + 1)));
                    let first_row = BlockListPoint::from_within_block_point(
                        &WithinBlock::new(Point { row: 0, col: 0 }, BlockIndex(index), grid_type),
                        model.block_list(),
                    )
                    .row;
                    column.add_child(
                        ConstrainedBox::new(
                            BlockGridElement::new(
                                grid,
                                appearance,
                                EnforceMinimumContrast::default(),
                                get_secret_obfuscation_mode(app),
                                self.size_info,
                            )
                            .with_find_matches(
                                find.as_ref()
                                    .and_then(|find| {
                                        if grid_type == GridType::Output {
                                            find.output_grid_matches()
                                        } else {
                                            find.command_grid_matches()
                                        }
                                    })
                                    .into_iter()
                                    .flatten()
                                    .cloned()
                                    .collect(),
                                find.as_ref()
                                    .and_then(|find| find.focused_range_for_grid(grid_type)),
                            )
                            .with_link(
                                BlockIndex(index),
                                grid_type,
                                self.highlighted_grid_link().and_then(|link| match link {
                                    super::model::terminal_model::WithinModel::BlockList(link)
                                        if link.block_index == BlockIndex(index)
                                            && link.grid == grid_type =>
                                    {
                                        Some(link.inner)
                                    }
                                    _ => None,
                                }),
                            )
                            .with_cursor(
                                (receives_native_input
                                    && BlockIndex(index)
                                        == model.block_list().active_block_index()
                                    && grid_type == block.active_grid_type())
                                .then_some(self.view_id),
                                self.native_cursor_blink_epoch(app),
                            )
                            .with_selection(first_row, &ranges, self.output_dragging.clone())
                            .with_context_menu(self.position_id.clone(), BlockIndex(index))
                            .finish(),
                        )
                        .with_height(grid_rows as f32 * cell_height)
                        .finish(),
                    );
                }
            }
        }
        if total_rows > rendered_to {
            column.add_child(
                ConstrainedBox::new(Empty::new().finish())
                    .with_height((total_rows - rendered_to) as f32 * cell_height)
                    .finish(),
            );
        }
        drop(cursor);
        drop(model);
        ClippedScrollable::vertical_centered(
            self.transcript_scroll.clone(),
            Align::new(column.finish()).bottom_left().finish(),
            ScrollbarWidth::Auto,
            appearance.theme().nonactive_ui_detail().into(),
            appearance.theme().active_ui_detail().into(),
            Fill::None,
        )
        .with_overlayed_scrollbar()
        .finish()
    }

    fn render_alt_screen(&self, app: &AppContext) -> Box<dyn Element> {
        let semantic_selection = SemanticSelection::as_ref(app);
        let model = self.model.lock();
        let selection = model
            .alt_screen()
            .selection_range(semantic_selection)
            .map(|selection| match selection {
                crate::terminal::model::selection::ExpandedSelectionRange::Regular {
                    start,
                    end,
                    ..
                } => vec1![start..end],
                crate::terminal::model::selection::ExpandedSelectionRange::Rect { rows } => {
                    rows.mapped(|(start, end)| start..end)
                }
            });
        drop(model);
        AltScreenElement::new(
            self.model.clone(),
            TerminalViewRenderContext {
                size_info: self.size_info,
                highlighted_url: self.highlighted_grid_link().and_then(|link| match link {
                    super::model::terminal_model::WithinModel::AltScreen(link) => Some(link),
                    super::model::terminal_model::WithinModel::BlockList(_) => None,
                }),
                link_tool_tip: None,
                is_terminal_focused: app.focused_view_id(self.input.window_id(app))
                    == Some(self.view_id),
                cursor_blink_epoch: self.native_cursor_blink_epoch(app),
                selection_dragging: self.output_dragging.clone(),
                pane_state: self
                    .focus_handle
                    .as_ref()
                    .map(|handle| handle.split_pane_state(app))
                    .unwrap_or(SplitPaneState::NotInSplitPane),
                active_session_state: if self
                    .focus_handle
                    .as_ref()
                    .is_none_or(|handle| handle.is_active_session(app))
                {
                    ActiveSessionState::Active
                } else {
                    ActiveSessionState::Inactive
                },
                terminal_view_id: self.view_id,
                hovered_secret: None,
                obfuscate_secrets: get_secret_obfuscation_mode(app),
            },
            self.find_model.clone(),
            EnforceMinimumContrast::default(),
            selection,
            Appearance::as_ref(app),
            Lines::zero(),
            None,
            None,
        )
        .with_context_menu_anchor(self.position_id.clone())
        .finish()
    }

    /// Copies part of `block_index` to the clipboard. Used by the transcript context menu.
    fn copy_block_text(
        &self,
        block_index: BlockIndex,
        part: BlockTextPart,
        ctx: &mut ViewContext<Self>,
    ) {
        let text = {
            let model = self.model.lock();
            let Some(block) = model.block_list().block_at(block_index) else {
                return;
            };
            match part {
                BlockTextPart::Command => block.command_to_string(),
                BlockTextPart::Output => block.output_to_string(),
                BlockTextPart::Both => {
                    let command = block.command_to_string();
                    let output = block.output_to_string();
                    if command.trim().is_empty() {
                        output
                    } else if output.trim().is_empty() {
                        command
                    } else {
                        format!("{command}\n{output}")
                    }
                }
            }
        };
        if !text.is_empty() {
            ctx.clipboard().write(ClipboardContent::plain_text(text));
        }
    }

    fn native_cursor_blink_epoch(&self, app: &AppContext) -> Option<Instant> {
        (AppEditorSettings::as_ref(app).cursor_blink_enabled()
            && app.windows().state().active_window == Some(self.input.window_id(app))
            && app.focused_view_id(self.input.window_id(app)) == Some(self.view_id))
        .then(|| self.local_io.cursor_blink_epoch())
    }

    fn write_bytes(&self, bytes: Vec<u8>, ctx: &mut ViewContext<Self>) {
        self.local_io.reset_cursor_blink();
        ctx.notify();
        ctx.emit(Event::WriteBytesToPty {
            bytes: Cow::Owned(bytes),
        });
    }
}

impl Entity for TerminalView {
    type Event = Event;
}

impl TerminalSurface for TerminalView {
    fn on_shell_determined(&mut self, ctx: &mut ViewContext<Self>) {
        if ctx.is_self_or_child_focused() {
            self.focus(ctx);
        }
    }

    fn on_active_shell_launch_data_updated(
        &mut self,
        shell_launch_data: Option<ShellLaunchData>,
        ctx: &mut ViewContext<Self>,
    ) {
        self.active_shell_launch_data = shell_launch_data;
        ctx.notify();
    }

    fn on_pty_spawn_failed(&mut self, error: anyhow::Error, ctx: &mut ViewContext<Self>) {
        self.pty_spawn_error = Some(error.to_string());
        ctx.emit(Event::PtySpawnFailed {
            reason: error.to_string(),
        });
        ctx.notify();
    }
}

impl TypedActionView for TerminalView {
    type Action = TerminalAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        if matches!(
            action,
            TerminalAction::UserInputSequence(_)
                | TerminalAction::ControlSequence(_)
                | TerminalAction::Up
                | TerminalAction::Down
                | TerminalAction::Home
                | TerminalAction::End
                | TerminalAction::PageUp
                | TerminalAction::PageDown
                | TerminalAction::CtrlC
                | TerminalAction::ClearBuffer
        ) {
            self.invalidate_pending_paste();
        }
        match action {
            TerminalAction::CtrlC => {
                if self.input_is_visible() && !self.input.as_ref(ctx).buffer_text(ctx).is_empty() {
                    let len = self.input.as_ref(ctx).buffer_text(ctx).len();
                    self.clear_buffer(ctx);
                    self.input.update(ctx, |_, ctx| {
                        ctx.emit(InputEvent::CtrlC {
                            cleared_buffer_len: len,
                        })
                    });
                } else {
                    ctx.emit(Event::InterruptPty);
                }
            }
            TerminalAction::TypedCharacters(text) if self.input_is_visible() => {
                self.input.update(ctx, |input, ctx| {
                    input.insert_text(text, ctx);
                    input.focus_input_box(ctx);
                });
            }
            TerminalAction::TypedCharacters(text) | TerminalAction::KeyDown(text) => {
                self.invalidate_pending_paste();
                self.model.lock().clear_marked_text();
                self.write_bytes(text.as_bytes().to_vec(), ctx);
                ctx.notify();
            }
            TerminalAction::SelectOutput(action) => {
                match action {
                    SelectAction::Begin {
                        point,
                        side,
                        selection_type,
                        ..
                    } => {
                        self.model.lock().block_list_mut().start_selection(
                            *point,
                            *selection_type,
                            *side,
                        );
                        self.links.dragged = false;
                        self.is_selecting = true;
                        ctx.focus_self();
                    }
                    SelectAction::Update { point, side, .. } => {
                        self.links.dragged = true;
                        self.clear_link(ctx);
                        self.model
                            .lock()
                            .block_list_mut()
                            .update_selection(*point, *side);
                    }
                    SelectAction::End => self.handle_action(&TerminalAction::FinishSelection, ctx),
                }
                ctx.notify();
            }
            TerminalAction::Copy => {
                if let Some(text) = self.selected_text(ctx) {
                    ctx.clipboard().write(ClipboardContent::plain_text(text));
                }
            }
            TerminalAction::UserInputSequence(bytes) | TerminalAction::ControlSequence(bytes) => {
                self.write_bytes(bytes.clone(), ctx)
            }
            TerminalAction::Up => self.write_bytes(b"\x1b[A".to_vec(), ctx),
            TerminalAction::Down => self.write_bytes(b"\x1b[B".to_vec(), ctx),
            TerminalAction::Home => self.write_bytes(b"\x1b[H".to_vec(), ctx),
            TerminalAction::End => self.write_bytes(b"\x1b[F".to_vec(), ctx),
            TerminalAction::PageUp if self.input_is_visible() => {
                self.transcript_scroll
                    .scroll_by((-self.size_info.pane_height_px).into_pixels());
                ctx.notify();
            }
            TerminalAction::PageDown if self.input_is_visible() => {
                self.transcript_scroll
                    .scroll_by(self.size_info.pane_height_px.into_pixels());
                ctx.notify();
            }
            TerminalAction::PageUp => self.write_bytes(b"\x1b[5~".to_vec(), ctx),
            TerminalAction::PageDown => self.write_bytes(b"\x1b[6~".to_vec(), ctx),
            TerminalAction::Paste => {
                let content = ctx.clipboard().read();
                match content.paths {
                    Some(paths) => {
                        let paths = paths.into_iter().map(PathBuf::from).collect::<Vec<_>>();
                        self.drop_paths(&paths, ctx);
                    }
                    None => self.paste_text(content.plain_text, ctx),
                }
            }
            TerminalAction::ClearBuffer => self.clear_buffer(ctx),
            TerminalAction::Focus => self.focus(ctx),
            TerminalAction::FinishSelection => {
                self.is_selecting = false;
                self.output_dragging
                    .store(false, std::sync::atomic::Ordering::Relaxed);
                if self.selected_text(ctx).is_none_or(|text| text.is_empty()) {
                    self.model.lock().block_list_mut().clear_selection();
                }
                self.focus(ctx);
            }
            TerminalAction::FocusInputAndClearSelection => {
                self.model.lock().block_list_mut().clear_selection();
                self.focus(ctx);
            }
            TerminalAction::ShowFindBar => {
                if !self.find_bar_open {
                    self.handle_wakeup(None, ctx);
                    self.find_options.blocks_to_include_in_results = None;
                }
                self.find_bar_open = true;
                self.find_bar.update(ctx, |bar, ctx| {
                    bar.display_find_within_block = if self.find_selected_blocks.is_empty() {
                        FindWithinBlockState::Hidden
                    } else if self.find_options.blocks_to_include_in_results.is_some() {
                        FindWithinBlockState::Enabled
                    } else {
                        FindWithinBlockState::Disabled
                    };
                    ctx.notify();
                });
                self.find_model.update(ctx, |model, ctx| {
                    model.run_find(self.find_options.clone(), ctx)
                });
                ctx.focus(&self.find_bar);
                ctx.notify();
            }
            TerminalAction::Close => ctx.emit(Event::Pane(PaneEvent::Close)),
            TerminalAction::ToggleMaximizePane => ctx.emit(Event::Pane(PaneEvent::ToggleMaximized)),
            TerminalAction::SplitRight(shell) => {
                ctx.emit(Event::Pane(PaneEvent::SplitRight(shell.clone())))
            }
            TerminalAction::SplitLeft(shell) => {
                ctx.emit(Event::Pane(PaneEvent::SplitLeft(shell.clone())))
            }
            TerminalAction::SplitDown(shell) => {
                ctx.emit(Event::Pane(PaneEvent::SplitDown(shell.clone())))
            }
            TerminalAction::SplitUp(shell) => {
                ctx.emit(Event::Pane(PaneEvent::SplitUp(shell.clone())))
            }
            TerminalAction::MaybeClearAltSelect
            | TerminalAction::AltMouseAction(_)
            | TerminalAction::AltScroll { .. } => {
                let bytes = {
                    let mut model = self.model.lock();
                    if let TerminalAction::AltMouseAction(mouse) = action {
                        if should_intercept_mouse(&model, mouse.modifiers().shift, ctx) {
                            None
                        } else {
                            mouse.to_escape_sequence(&*model)
                        }
                    } else if let TerminalAction::AltScroll { delta, point } = action {
                        alt_screen_scroll_to_pty_bytes(
                            *delta,
                            *point,
                            !should_intercept_scroll(&model, ctx),
                            &*model,
                        )
                    } else {
                        model.alt_screen_mut().clear_selection();
                        None
                    }
                };
                if let Some(bytes) = bytes {
                    self.write_bytes(bytes, ctx);
                }
                ctx.notify();
            }
            TerminalAction::AltSelect(action) => match action {
                SelectAction::Begin {
                    point,
                    side,
                    selection_type,
                    ..
                } => {
                    self.model.lock().alt_screen_mut().start_selection(
                        *point,
                        *selection_type,
                        *side,
                    );
                    self.links.dragged = false;
                    self.is_selecting = true;
                    ctx.notify();
                }
                SelectAction::Update { point, side, .. } => {
                    self.links.dragged = true;
                    self.clear_link(ctx);
                    self.model
                        .lock()
                        .alt_screen_mut()
                        .update_selection(*point, *side);
                    ctx.notify();
                }
                SelectAction::End => self.handle_action(&TerminalAction::FinishSelection, ctx),
            },
            TerminalAction::AltScreenContextMenu { position } => {
                let items = self.alt_screen_context_menu_items(ctx);
                self.show_context_menu(*position, items, ctx);
            }
            TerminalAction::BlockContextMenu {
                position,
                block_index,
            } => {
                let items = self.block_context_menu_items(*block_index, ctx);
                self.show_context_menu(*position, items, ctx);
            }
            TerminalAction::CloseContextMenu => self.close_context_menu(ctx),
            TerminalAction::RestoreContextMenuFocus { generation } => {
                if *generation == self.context_menu_generation
                    && self.context_menu_state.is_none()
                    && self.context_menu.is_focused(ctx)
                {
                    if self.context_menu_return_to_find && self.find_bar_open {
                        ctx.focus(&self.find_bar);
                    } else {
                        self.focus(ctx);
                    }
                }
            }
            TerminalAction::CopyBlockCommand(block_index) => {
                self.copy_block_text(*block_index, BlockTextPart::Command, ctx);
            }
            TerminalAction::CopyBlockOutput(block_index) => {
                self.copy_block_text(*block_index, BlockTextPart::Output, ctx);
            }
            TerminalAction::CopyBlock(block_index) => {
                self.copy_block_text(*block_index, BlockTextPart::Both, ctx);
            }
            TerminalAction::InsertSelectedTextIntoInput => {
                if let Some(text) = self.selected_text(ctx).filter(|text| !text.is_empty()) {
                    self.model.lock().block_list_mut().clear_selection();
                    self.input.update(ctx, |input, ctx| {
                        input.insert_text(&text, ctx);
                        input.focus_input_box(ctx);
                    });
                }
            }
            TerminalAction::DragAndDropFiles(paths) => self.drop_paths(paths, ctx),
            TerminalAction::StartFileDropTarget => {
                if !self.file_drop_active {
                    self.file_drop_active = true;
                    ctx.notify();
                }
            }
            TerminalAction::StopFileDropTarget => {
                if self.file_drop_active {
                    self.file_drop_active = false;
                    ctx.notify();
                }
            }
            TerminalAction::SetMarkedText {
                text,
                selected_range,
            } => {
                self.invalidate_pending_paste();
                self.model.lock().set_marked_text(text, selected_range);
                ctx.notify();
            }
            TerminalAction::ClearMarkedText => {
                self.local_io.reset_cursor_blink();
                self.model.lock().clear_marked_text();
                ctx.notify();
            }
            TerminalAction::MaybeLinkHover { position } => {
                if !self.is_selecting {
                    self.links.dragged = false;
                }
                self.hover_link(*position, None, ctx);
            }
            TerminalAction::ClickOnGrid {
                position,
                modifiers,
            } => {
                if self.selected_text(ctx).is_none_or(|text| text.is_empty()) {
                    self.hover_link(Some(*position), Some(*modifiers), ctx);
                }
            }
            TerminalAction::OpenGridLink { generation } => self.open_grid_link(*generation, ctx),
            TerminalAction::MaybeDismissToolTip { .. } => self.dismiss_tooltips(ctx),
            TerminalAction::Scroll { .. }
            | TerminalAction::MiddleClickOnGrid { .. }
            | TerminalAction::MaybeHoverSecret => {}
        }
    }
}

impl View for TerminalView {
    fn ui_name() -> &'static str {
        "Terminal"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let output = if let Some(error) = &self.pty_spawn_error {
            Appearance::as_ref(app)
                .ui_builder()
                .paragraph(format!("Unable to start local shell: {error}"))
                .build()
                .finish()
        } else if self.model.lock().is_alt_screen_active() {
            self.render_alt_screen(app)
        } else {
            self.render_blocks(app)
        };
        let output = EventHandler::new(output)
            .with_always_handle()
            .on_left_mouse_up(|ctx, _, _| {
                ctx.dispatch_typed_action(TerminalAction::FinishSelection);
                DispatchEventResult::StopPropagation
            })
            .finish();
        // Fallback for right-clicks that no block grid claimed: the gaps between blocks, the
        // prompt row, and the empty space below the transcript. This wrapper deliberately does
        // not set `always_handle`, so a click a block already handled stops here and keeps its
        // block-specific menu entries.
        let position_id = self.position_id.clone();
        let output = EventHandler::new(output)
            .on_right_mouse_down(move |ctx, app, position, modifiers| {
                let action = if should_right_click_paste(modifiers.shift, app) {
                    TerminalAction::Paste
                } else {
                    TerminalAction::BlockContextMenu {
                        position: context_menu_offset(ctx, Some(&position_id), position),
                        block_index: None,
                    }
                };
                ctx.dispatch_typed_action(action);
                DispatchEventResult::StopPropagation
            })
            .finish();
        let receives_input = app.focused_view_id(self.input.window_id(app)) == Some(self.view_id);
        let output =
            TerminalSizeElement::new(self.resize_tx.clone(), output, receives_input).finish();
        let mut column = Flex::column()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch);
        if self.find_bar_open {
            column.add_child(ChildView::new(&self.find_bar).finish());
        }
        column.add_child(Expanded::new(1., output).finish());
        if self.input_is_visible() {
            column.add_child(self.render_input_prompt(app));
            column = column.with_child(ChildView::new(&self.input).finish());
        }

        let content = TerminalSizeElement::file_drop_target(column.finish()).finish();
        let mut stack = Stack::new();
        stack.add_child(SavePosition::new(content, &self.position_id).finish());
        if self.file_drop_active {
            stack.add_positioned_overlay_child(
                Appearance::as_ref(app)
                    .ui_builder()
                    .paragraph("Drop files to insert their paths.")
                    .build()
                    .finish(),
                OffsetPositioning::offset_from_parent(
                    Vector2F::new(8., 8.),
                    ParentOffsetBounds::ParentBySize,
                    ParentAnchor::TopLeft,
                    ChildAnchor::TopLeft,
                ),
            );
        }
        if let Some(hint) = self.render_link_hint(app) {
            stack.add_positioned_overlay_child(
                hint,
                OffsetPositioning::offset_from_parent(
                    Vector2F::new(8., -8.),
                    ParentOffsetBounds::ParentBySize,
                    ParentAnchor::BottomLeft,
                    ChildAnchor::BottomLeft,
                ),
            );
        }
        if let Some(context_menu_state) = &self.context_menu_state {
            stack.add_positioned_overlay_child(
                ChildView::new(&self.context_menu).finish(),
                OffsetPositioning::offset_from_parent(
                    context_menu_state.position,
                    ParentOffsetBounds::WindowByPosition,
                    ParentAnchor::TopLeft,
                    ChildAnchor::TopLeft,
                ),
            );
        }
        stack.finish()
    }

    fn active_cursor_position(&self, ctx: &ViewContext<Self>) -> Option<warpui::CursorInfo> {
        if self.input_is_visible() || !ctx.is_self_focused() {
            return None;
        }
        ctx.element_position_by_id(format!("terminal_view:cursor_{}", self.view_id))
            .map(|position| warpui::CursorInfo {
                position,
                font_size: Appearance::as_ref(ctx).monospace_font_size(),
            })
    }

    fn on_focus(&mut self, focus: &FocusContext, ctx: &mut ViewContext<Self>) {
        if focus.is_self_focused() {
            self.focus(ctx);
        }
    }

    fn on_blur(&mut self, blur: &warpui::BlurContext, ctx: &mut ViewContext<Self>) {
        // A plain output click returns focus to this pane's input while file detection
        // may still be running. Only leaving the pane invalidates that user action.
        if !ctx.is_self_or_child_focused() {
            self.clear_link(ctx);
        }
        if blur.is_self_blurred() {
            self.invalidate_pending_paste();
            self.model.lock().clear_marked_text();
            ctx.notify();
        }
    }

    fn keymap_context(&self, _app: &AppContext) -> warpui::keymap::Context {
        let mut context = Self::default_keymap_context();
        if self.input_is_visible() {
            context.set.insert(init::INPUT_BOX_VISIBLE_KEY);
        }
        let model = self.model.lock();
        if model.is_alt_screen_active() {
            context.set.insert("AltScreen");
        }
        if model.block_list().selection().is_some() {
            context.set.insert("OutputSelected");
        }
        context
    }
}

impl BackingView for TerminalView {
    type PaneHeaderOverflowMenuAction = TerminalAction;
    type CustomAction = ();
    type AssociatedData = ();

    fn handle_pane_header_overflow_menu_action(
        &mut self,
        action: &Self::PaneHeaderOverflowMenuAction,
        ctx: &mut ViewContext<Self>,
    ) {
        self.handle_action(action, ctx);
    }

    fn pane_header_overflow_menu_items(&self, app: &AppContext) -> Vec<MenuItem<TerminalAction>> {
        let is_maximized = self
            .focus_handle
            .as_ref()
            .is_some_and(|handle| handle.is_maximized(app));
        vec![
            MenuItemFields::toggle_pane_action(is_maximized)
                .with_on_select_action(TerminalAction::ToggleMaximizePane)
                .into_item(),
        ]
    }

    fn close(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.emit(Event::Pane(PaneEvent::Close));
    }

    fn focus_contents(&mut self, ctx: &mut ViewContext<Self>) {
        self.focus(ctx);
    }

    fn render_header_content(
        &self,
        _ctx: &view::HeaderRenderContext<'_>,
        app: &AppContext,
    ) -> view::HeaderContent {
        view::HeaderContent::Standard(view::StandardHeader {
            title: self.pane_configuration.as_ref(app).title().to_owned(),
            title_secondary: None,
            title_style: None,
            title_clip_config: warpui::text_layout::ClipConfig::start(),
            title_max_width: None,
            left_of_title: None,
            right_of_title: None,
            left_of_overflow: None,
            options: Default::default(),
        })
    }

    fn set_focus_handle(&mut self, focus_handle: PaneFocusHandle, ctx: &mut ViewContext<Self>) {
        self.install_focus_handle(focus_handle, ctx);
    }
}

#[cfg(test)]
#[path = "local_view_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "local_interaction_tests.rs"]
mod local_interaction_tests;

#[cfg(test)]
#[path = "view/viewport_tests.rs"]
mod viewport_tests;
