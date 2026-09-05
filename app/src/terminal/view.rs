mod action;
pub mod init;

use std::borrow::Cow;
use std::sync::Arc;
use std::sync::mpsc::SyncSender;

use async_channel::{Receiver, Sender};
use parking_lot::FairMutex;
use pathfinder_geometry::vector::Vector2F;
use vec1::Vec1;
use warp_completer::meta::Span;
use warp_core::semantic_selection::SemanticSelection;
use warp_editor::model::CoreEditorModel as _;
use warpui::clipboard::ClipboardContent;
use warpui::elements::{
    ChildView, Clipped, Expanded, Flex, ParentElement, Shrinkable,
};
use warpui::{
    AppContext, Element, Entity, EntityId, FocusContext, ModelHandle, SingletonEntity,
    TypedActionView, View, ViewContext, ViewHandle,
};

pub use action::TerminalAction;

use super::alt_screen::alt_screen_element::AltScreenElement;
use super::blockgrid_element::BlockGridElement;
use super::color::List;
use super::find::TerminalFindModel;
use super::input::{self, CommandExecutionSource, Input};
use super::model::ObfuscateSecrets;
use super::model::completions::ShellCompletion;
use super::model::grid::grid_handler::Link;
use super::model::selection::SelectAction;
use super::model::session::{Session, SessionId, Sessions};
use super::model::terminal_model::{TerminalInputState, WithinModel};
use super::model_events::{ModelEvent, ModelEventDispatcher};
use super::terminal_size_element::TerminalSizeElement;
use super::{
    PtyIntent, PtyIntentEvent, ShellLaunchData, SizeInfo, SizeUpdate, SizeUpdateReason,
    TerminalModel, TerminalSurface,
};
use crate::appearance::Appearance;
use crate::code::buffer_location::LocalOrRemotePath;
use crate::code::editor_management::CodeSource;
use crate::util::openable_file_type::EditorLayout;
use crate::menu::{MenuItem, MenuItemFields};
use crate::pane_group::focus_state::PaneFocusHandle;
use crate::pane_group::pane::view;
use crate::pane_group::{BackingView, PaneConfiguration, PaneEvent, SplitPaneState};
use crate::persistence;
use crate::session_management::{CommandContext, SessionNavigationPromptElements};
use crate::settings::EnforceMinimumContrast;
use crate::terminal::event::BlockCompletedEvent;
use crate::terminal::input::Event as InputEvent;
use crate::terminal::model::block::SerializedBlock;
use crate::terminal::model::index::Point;
use crate::terminal::model::mouse::MouseState;
use crate::terminal::shell::ShellType;
use warp_util::path::ShellFamily;
use crate::throttle::throttle;
use crate::util::openable_file_type::FileTarget;
use crate::view_components::find::Find;
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
    WriteBytesToPty { bytes: Cow<'static, [u8]> },
    Resize { size_update: SizeUpdate },
    ExecuteCommand(ExecuteCommandEvent),
    BlockStarted { is_for_in_band_command: bool },
    FocusSession,
    SessionBootstrapped,
    ShellSpawned(ShellType),
    PtySpawnFailed { reason: String },
    OpenFileInWarp {
        path: std::path::PathBuf,
        session: Arc<Session>,
    },
    #[cfg(feature = "local_fs")]
    OpenCodeInWarp { source: CodeSource, layout: EditorLayout },
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
    pub is_terminal_selecting: bool,
    pub pane_state: SplitPaneState,
    pub active_session_state: ActiveSessionState,
    pub terminal_view_id: EntityId,
    pub hovered_secret: Option<super::model::SecretHandle>,
    pub obfuscate_secrets: ObfuscateSecrets,
}

pub struct TerminalView {
    pub model: Arc<FairMutex<TerminalModel>>,
    input: ViewHandle<Input>,
    size_info: SizeInfo,
    colors: List,
    resize_tx: Sender<Vector2F>,
    find_model: ModelHandle<TerminalFindModel>,
    find_bar: ViewHandle<Find<TerminalFindModel>>,
    pane_configuration: ModelHandle<PaneConfiguration>,
    focus_handle: Option<PaneFocusHandle>,
    sessions: ModelHandle<Sessions>,
    model_events: ModelHandle<ModelEventDispatcher>,
    model_event_sender: Option<SyncSender<persistence::ModelEvent>>,
    active_shell_launch_data: Option<ShellLaunchData>,
    current_repo_path: Option<LocalOrRemotePath>,
    pty_spawn_error: Option<String>,
    is_selecting: bool,
}

impl TerminalView {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        resources: crate::pane_group::TerminalViewResources,
        wakeups_rx: Receiver<()>,
        model_events: ModelHandle<ModelEventDispatcher>,
        model: Arc<FairMutex<TerminalModel>>,
        sessions: ModelHandle<Sessions>,
        size_info: SizeInfo,
        colors: List,
        model_event_sender: Option<SyncSender<persistence::ModelEvent>>,
        ctx: &mut ViewContext<Self>,
    ) -> Self {
        let input = ctx.add_typed_action_view(Input::new);
        ctx.subscribe_to_view(&input, |view, _, event, ctx| view.handle_input_event(event, ctx));
        let find_model = ctx.add_model(|ctx| TerminalFindModel::new(model.clone(), ctx));
        let find_bar = ctx.add_typed_action_view(|ctx| Find::new(find_model.clone(), ctx));
        let pane_configuration = ctx.add_model(|_| PaneConfiguration::new("Terminal"));
        ctx.subscribe_to_model(&model_events, |view, _, event, ctx| {
            view.handle_model_event(event, ctx)
        });
        ctx.spawn_stream_local(
            throttle(WAKEUP_THROTTLE_PERIOD, wakeups_rx),
            |view, _, ctx| view.handle_wakeup(ctx),
            |_, _| {},
        );
        let (resize_tx, resize_rx) = async_channel::unbounded();
        ctx.spawn_stream_local(resize_rx, Self::after_layout, |_, _| {});
        Self {
            model,
            input,
            size_info,
            colors,
            resize_tx,
            find_model,
            find_bar,
            pane_configuration,
            focus_handle: None,
            sessions,
            model_events,
            model_event_sender: model_event_sender.or(resources.model_event_sender),
            active_shell_launch_data: None,
            current_repo_path: None,
            pty_spawn_error: None,
            is_selecting: false,
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
            .map(|session| session.shell().shell_family())
            .unwrap_or(ShellFamily::Posix)
    }

    pub fn active_session(&self, ctx: &AppContext) -> Option<Arc<Session>> {
        let id = self.model_events.as_ref(ctx).active_session_id()?;
        self.sessions.as_ref(ctx).get(id)
    }

    pub fn current_working_directory(&self, _app: &AppContext) -> Option<String> {
        self.model.lock().block_list().active_block().pwd().cloned()
    }

    pub fn active_shell_launch_data(&self) -> Option<ShellLaunchData> {
        self.active_shell_launch_data.clone()
    }

    pub fn full_prompt(&self, _app: &AppContext) -> String {
        String::new()
    }

    pub fn prompt_elements(&self, _app: &AppContext) -> SessionNavigationPromptElements {
        SessionNavigationPromptElements::default()
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
            .map_or(CommandContext::None, |block| CommandContext::LastRunCommand {
                last_run_command: block.command_to_string(),
                mins_since_completion: None,
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
                self.input
                    .update(ctx, |input, ctx| input.run_command_in_synced_terminal_input(ctx));
            }
            SyncInputType::StartSyncing | SyncInputType::StopSyncing => {}
        }
    }

    pub fn focus(&mut self, ctx: &mut ViewContext<Self>) {
        if self.input_is_visible() {
            self.input.update(ctx, |input, ctx| input.focus_input_box(ctx));
        } else {
            ctx.focus_self();
        }
        ctx.emit(Event::FocusSession);
    }

    pub fn clear_buffer(&mut self, ctx: &mut ViewContext<Self>) {
        self.input
            .update(ctx, |input, ctx| input.clear_buffer_and_reset_undo_stack(ctx));
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
            InputEvent::CtrlC { .. } => ctx.emit(Event::InterruptPty),
            InputEvent::CtrlD => ctx.emit(Event::CtrlD),
            InputEvent::EditorFocused => ctx.notify(),
        }
    }

    fn handle_model_event(&mut self, event: &ModelEvent, ctx: &mut ViewContext<Self>) {
        match event {
            ModelEvent::Title(title) => {
                self.pane_configuration.update(ctx, |configuration, ctx| {
                    configuration.set_title(title.clone(), ctx)
                });
            }
            ModelEvent::BlockCompleted(BlockCompletedEvent { block_id, .. }) => {
                if let Some(block) = self
                    .model
                    .lock()
                    .block_list()
                    .block_with_id(block_id)
                    .map(|block| Arc::new(block.serialized()))
                {
                    ctx.emit(Event::BlockCompleted {
                        block,
                        is_local: true,
                    });
                }
                self.input.update(ctx, |input, ctx| input.focus_input_box(ctx));
            }
            ModelEvent::AfterBlockStarted {
                is_for_in_band_command,
                ..
            } => ctx.emit(Event::BlockStarted {
                is_for_in_band_command: *is_for_in_band_command,
            }),
            ModelEvent::TerminalClear => ctx.emit(Event::BlockListCleared),
            ModelEvent::Exit { .. } => ctx.emit(Event::Exited),
            ModelEvent::Handler(_)
            | ModelEvent::AfterBlockCompleted(_)
            | ModelEvent::BlockMetadataReceived(_)
            | ModelEvent::BlockWorkingDirectoryUpdated(_)
            | ModelEvent::BackgroundBlockStarted
            | ModelEvent::ClipboardStore(_, _)
            | ModelEvent::ClipboardLoad(_, _)
            | ModelEvent::CursorBlinkingChange(_)
            | ModelEvent::TerminalModeSwapped(_)
            | ModelEvent::VisibleBootstrapBlock
            | ModelEvent::PromptUpdated
            | ModelEvent::FinishUpdate(_)
            | ModelEvent::Typeahead
            | ModelEvent::CompletionsFinished(_, _)
            | ModelEvent::MouseCursorDirty
            | ModelEvent::ExecutedInBandCommand(_)
            | ModelEvent::DetectedEndOfSshLogin(_)
            | ModelEvent::InitSubshell(_)
            | ModelEvent::SourcedRcFileInSubshell(_)
            | ModelEvent::Bell
            | ModelEvent::PreInteractiveSSHSession
            | ModelEvent::SSH(_)
            | ModelEvent::ExitShell { .. }
            | ModelEvent::SSHControlMasterError => ctx.notify(),
        }
    }

    fn handle_wakeup(&mut self, ctx: &mut ViewContext<Self>) {
        let mut model = self.model.lock();
        if !model.is_alt_screen_active() {
            model.block_list_mut().update_background_block_height();
            model.block_list_mut().update_active_block_height();
        }
        drop(model);
        ctx.notify();
    }

    fn after_layout(&mut self, size: Vector2F, ctx: &mut ViewContext<Self>) {
        let new_size = SizeInfo::new(
            size,
            self.size_info.cell_width_px(),
            self.size_info.cell_height_px(),
            self.size_info.padding_x_px(),
            self.size_info.padding_y_px(),
        );
        if new_size.rows() == self.size_info.rows()
            && new_size.columns() == self.size_info.columns()
        {
            return;
        }
        let update = SizeUpdate {
            update_reason: SizeUpdateReason::AfterLayout,
            last_size: self.size_info,
            new_size,
            new_gap_height: None,
            natural_rows: new_size.rows(),
            natural_cols: new_size.columns(),
        };
        self.model.lock().resize(update);
        self.size_info = new_size;
        ctx.emit(Event::Resize { size_update: update });
        ctx.notify();
    }

    fn render_blocks(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let model = self.model.lock();
        let scope = model.block_list().transcript_scope();
        let mut column = Flex::column().with_reverse_orientation();
        for block in model.block_list().blocks().iter().rev() {
            if !block.is_visible(scope) {
                continue;
            }
            if !block.should_hide_output_grid() {
                column.add_child(
                    BlockGridElement::new(
                        block.output_grid(),
                        appearance,
                        EnforceMinimumContrast::default(),
                        ObfuscateSecrets::No,
                        self.size_info,
                    )
                    .finish(),
                );
            }
            if !block.should_hide_command_grid() {
                column.add_child(
                    BlockGridElement::new(
                        block.prompt_and_command_grid(),
                        appearance,
                        EnforceMinimumContrast::default(),
                        ObfuscateSecrets::No,
                        self.size_info,
                    )
                    .finish(),
                );
            }
        }
        drop(model);
        Clipped::new(column.finish()).finish()
    }

    fn render_alt_screen(&self, app: &AppContext) -> Box<dyn Element> {
        let semantic_selection = SemanticSelection::as_ref(app);
        let model = self.model.lock();
        let selection = model.alt_screen().selection_range(semantic_selection);
        drop(model);
        AltScreenElement::new(
            self.model.clone(),
            TerminalViewRenderContext {
                size_info: self.size_info,
                highlighted_url: None,
                link_tool_tip: None,
                is_terminal_focused: true,
                is_terminal_selecting: self.is_selecting,
                pane_state: SplitPaneState::NotInSplitPane,
                active_session_state: ActiveSessionState::Active,
                terminal_view_id: self.input.id(),
                hovered_secret: None,
                obfuscate_secrets: ObfuscateSecrets::No,
            },
            self.find_model.clone(),
            EnforceMinimumContrast::default(),
            selection,
            Appearance::as_ref(app),
            0.into(),
            None,
            None,
        )
        .finish()
    }

    fn write_bytes(&self, bytes: Vec<u8>, ctx: &mut ViewContext<Self>) {
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
        self.focus(ctx);
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
            TerminalAction::CtrlD => ctx.emit(Event::CtrlD),
            TerminalAction::TypedCharacters(text) | TerminalAction::KeyDown(text) => {
                self.write_bytes(text.as_bytes().to_vec(), ctx)
            }
            TerminalAction::UserInputSequence(bytes)
            | TerminalAction::ControlSequence(bytes) => self.write_bytes(bytes.clone(), ctx),
            TerminalAction::Up => self.write_bytes(b"\x1b[A".to_vec(), ctx),
            TerminalAction::Down => self.write_bytes(b"\x1b[B".to_vec(), ctx),
            TerminalAction::Home => self.write_bytes(b"\x1b[H".to_vec(), ctx),
            TerminalAction::End => self.write_bytes(b"\x1b[F".to_vec(), ctx),
            TerminalAction::PageUp => self.write_bytes(b"\x1b[5~".to_vec(), ctx),
            TerminalAction::PageDown => self.write_bytes(b"\x1b[6~".to_vec(), ctx),
            TerminalAction::Paste => {
                if !self.input_is_visible()
                    && let Some(ClipboardContent::Text(text)) = ctx.clipboard().read()
                {
                    self.write_bytes(text.into_bytes(), ctx);
                }
            }
            TerminalAction::ClearBuffer => self.clear_buffer(ctx),
            TerminalAction::Focus | TerminalAction::FocusInputAndClearSelection => self.focus(ctx),
            TerminalAction::ShowFindBar => {
                self.find_bar.update(ctx, |find, ctx| find.open(ctx));
            }
            TerminalAction::Close => ctx.emit(Event::Pane(PaneEvent::Close)),
            TerminalAction::ToggleMaximizePane => {
                ctx.emit(Event::Pane(PaneEvent::ToggleMaximized))
            }
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
            TerminalAction::MaybeClearAltSelect => {
                self.model.lock().alt_screen_mut().clear_selection();
                ctx.notify();
            }
            TerminalAction::AltMouseAction(mouse) => {
                if let Some(bytes) = mouse.to_escape_sequence(&*self.model.lock()) {
                    self.write_bytes(bytes, ctx);
                }
            }
            TerminalAction::AltSelect(action) => match action {
                SelectAction::Begin {
                    point,
                    side,
                    selection_type,
                    ..
                } => {
                    self.model
                        .lock()
                        .alt_screen_mut()
                        .start_selection(*point, *selection_type, *side);
                    self.is_selecting = true;
                    ctx.notify();
                }
                SelectAction::Update { point, side, .. } => {
                    self.model
                        .lock()
                        .alt_screen_mut()
                        .update_selection(*point, *side);
                    ctx.notify();
                }
                SelectAction::End => {
                    self.is_selecting = false;
                    ctx.notify();
                }
            },
            TerminalAction::Scroll { .. }
            | TerminalAction::AltScroll { .. }
            | TerminalAction::AltScreenContextMenu { .. }
            | TerminalAction::ClickOnGrid { .. }
            | TerminalAction::MiddleClickOnGrid { .. }
            | TerminalAction::MaybeDismissToolTip { .. }
            | TerminalAction::MaybeHoverSecret
            | TerminalAction::MaybeLinkHover
            | TerminalAction::Copy
            | TerminalAction::ClearMarkedText
            | TerminalAction::SetMarkedText(_)
            | TerminalAction::StartFileDropTarget
            | TerminalAction::StopFileDropTarget
            | TerminalAction::DragAndDropFiles(_) => {}
        }
    }
}

impl View for TerminalView {
    fn ui_name() -> &'static str {
        "Terminal"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let output = if let Some(error) = &self.pty_spawn_error {
            warpui::elements::Text::new(format!("Unable to start local shell: {error}"))
                .finish()
        } else if self.model.lock().is_alt_screen_active() {
            self.render_alt_screen(app)
        } else {
            self.render_blocks(app)
        };
        let output = TerminalSizeElement::new(self.resize_tx.clone(), output).finish();
        let mut column = Flex::column().child(Expanded::new(1., output).finish());
        if self.input_is_visible() {
            column = column.child(Shrinkable::new(0., ChildView::new(&self.input).finish()).finish());
        }
        column.finish()
    }

    fn on_focus(&mut self, focus: &FocusContext, ctx: &mut ViewContext<Self>) {
        if focus.is_self_focused() {
            self.focus(ctx);
        }
    }

    fn keymap_context(&self, _app: &AppContext) -> warpui::keymap::Context {
        let mut context = Self::default_keymap_context();
        if self.input_is_visible() {
            context.set.insert(init::INPUT_BOX_VISIBLE_KEY);
        }
        if self.model.lock().is_alt_screen_active() {
            context.set.insert("AltScreen");
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
