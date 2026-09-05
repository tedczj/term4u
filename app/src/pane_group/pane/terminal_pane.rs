use std::sync::mpsc::SyncSender;

use warpui::{AppContext, EntityId, ModelHandle, ViewContext, ViewHandle, WindowId};

use super::{
    DetachType, PaneConfiguration, PaneContent, PaneId, PaneView, ShareableLink,
    ShareableLinkError, TerminalPaneId,
};
use crate::app_state::{LeafContents, TerminalPaneSnapshot};
use crate::pane_group::{self, PaneGroup};
use crate::persistence::{BlockCompleted, ModelEvent};
use crate::session_management::SessionNavigationData;
use crate::terminal::view::Event;
use crate::terminal::{TerminalManager, TerminalView};
use crate::workspace::PaneViewLocator;

pub type TerminalPaneView = PaneView<TerminalView>;

pub struct TerminalPane {
    model_event_sender: Option<SyncSender<ModelEvent>>,
    uuid: Vec<u8>,
    pane_configuration: ModelHandle<PaneConfiguration>,
    view: ViewHandle<TerminalPaneView>,
}

impl TerminalPane {
    pub(in crate::pane_group) fn new(
        uuid: Vec<u8>,
        terminal_manager: ModelHandle<Box<dyn TerminalManager>>,
        terminal_view: ViewHandle<TerminalView>,
        model_event_sender: Option<SyncSender<ModelEvent>>,
        ctx: &mut ViewContext<PaneGroup>,
    ) -> Self {
        let pane_configuration = terminal_view.as_ref(ctx).pane_configuration().clone();
        let view = ctx.add_typed_action_view(|ctx| {
            PaneView::new(
                PaneId::from_terminal_pane_ctx(ctx),
                terminal_view,
                terminal_manager,
                pane_configuration.clone(),
                ctx,
            )
        });
        Self {
            model_event_sender,
            uuid,
            pane_configuration,
            view,
        }
    }

    #[cfg(any(test, feature = "integration_tests"))]
    pub(in crate::pane_group) fn pane_view(&self) -> ViewHandle<TerminalPaneView> {
        self.view.clone()
    }

    pub(crate) fn terminal_view(&self, ctx: &AppContext) -> ViewHandle<TerminalView> {
        self.view.as_ref(ctx).child(ctx)
    }

    pub(in crate::pane_group) fn session_uuid(&self) -> Vec<u8> {
        self.uuid.clone()
    }

    pub(in crate::pane_group) fn terminal_manager(
        &self,
        ctx: &AppContext,
    ) -> ModelHandle<Box<dyn TerminalManager>> {
        self.view.as_ref(ctx).child_data(ctx).clone()
    }

    pub(in crate::pane_group) fn delete_blocks(&self) {
        if let Some(sender) = &self.model_event_sender {
            let _ = sender.send(ModelEvent::DeleteBlocks(self.uuid.clone()));
        }
    }

    pub fn session_navigation_data(
        &self,
        pane_group_id: EntityId,
        window_id: WindowId,
        app: &AppContext,
    ) -> SessionNavigationData {
        let view = self.terminal_view(app);
        let view = view.as_ref(app);
        SessionNavigationData::new(
            view.full_prompt(app),
            view.prompt_elements(app),
            view.session_command_context(app),
            PaneViewLocator {
                pane_group_id,
                pane_id: self.id(),
            },
            view.last_focus_ts(),
            window_id,
        )
    }

    pub fn terminal_pane_id(&self) -> TerminalPaneId {
        self.id()
            .as_terminal_pane_id()
            .expect("terminal pane id has terminal type")
    }
}

impl PaneContent for TerminalPane {
    fn id(&self) -> PaneId {
        PaneId::from_terminal_pane_view(&self.view)
    }

    fn attach(
        &self,
        _group: &PaneGroup,
        focus_handle: crate::pane_group::focus_state::PaneFocusHandle,
        ctx: &mut ViewContext<PaneGroup>,
    ) {
        self.view
            .update(ctx, |view, ctx| view.set_focus_handle(focus_handle, ctx));
        let pane_id = self.id();
        let sender = self.model_event_sender.clone();
        let uuid = self.uuid.clone();
        ctx.subscribe_to_view(&self.terminal_view(ctx), move |group, _, event, ctx| {
            match event {
                Event::Pane(event) => group.handle_pane_event(pane_id, event, ctx),
                Event::ExecuteCommand(event) => {
                    ctx.emit(pane_group::Event::ExecuteCommand(event.clone()))
                }
                Event::SyncInput(event) => ctx.emit(pane_group::Event::SyncInput(event.clone())),
                Event::BlockCompleted { block, is_local } => {
                    if let Some(sender) = &sender {
                        let _ = sender.send(ModelEvent::SaveBlock(BlockCompleted {
                            pane_id: uuid.clone(),
                            is_local: *is_local,
                            block: block.clone(),
                        }));
                    }
                }
                Event::Exited => ctx.emit(pane_group::Event::Exited {
                    add_to_undo_stack: true,
                }),
                Event::FocusSession => ctx.emit(pane_group::Event::ActiveSessionChanged),
                Event::OpenFileInWarp { path, session } => {
                    ctx.emit(pane_group::Event::OpenFileInWarp {
                        path: crate::code::buffer_location::LocalOrRemotePath::Local(path.clone()),
                        session: session.clone(),
                    })
                }
                #[cfg(feature = "local_fs")]
                Event::OpenCodeInWarp { source, layout } => {
                    ctx.emit(pane_group::Event::OpenCodeInWarp {
                        source: source.clone(),
                        layout: *layout,
                        line_col: None,
                    })
                }
                #[cfg(feature = "local_fs")]
                Event::OpenFileWithTarget {
                    path,
                    target,
                    line_col,
                } => ctx.emit(pane_group::Event::OpenFileWithTarget {
                    path: path.clone(),
                    target: target.clone(),
                    line_col: *line_col,
                }),
                Event::AppStateChanged => ctx.emit(pane_group::Event::AppStateChanged),
                Event::ShowCommandSearch(options) => {
                    ctx.emit(pane_group::Event::ShowCommandSearch(options.clone()))
                }
                Event::BlockListCleared
                | Event::CtrlD
                | Event::InterruptPty
                | Event::ShutdownPty
                | Event::WriteBytesToPty { .. }
                | Event::Resize { .. }
                | Event::BlockStarted { .. }
                | Event::SessionBootstrapped
                | Event::ShellSpawned(_)
                | Event::PtySpawnFailed { .. }
                | Event::RunNativeShellCompletions { .. } => {}
            }
        });
        ctx.subscribe_to_view(&self.view, move |group, _, event, ctx| {
            group.handle_pane_view_event(pane_id, event, ctx);
        });
    }

    fn detach(
        &self,
        _group: &PaneGroup,
        detach_type: DetachType,
        ctx: &mut ViewContext<PaneGroup>,
    ) {
        ctx.unsubscribe_to_view(&self.terminal_view(ctx));
        ctx.unsubscribe_to_view(&self.view);
        self.terminal_manager(ctx)
            .as_ref(ctx)
            .on_view_detached(detach_type, ctx);
    }

    fn snapshot(&self, app: &AppContext) -> LeafContents {
        let view = self.terminal_view(app);
        let view = view.as_ref(app);
        LeafContents::Terminal(TerminalPaneSnapshot {
            uuid: self.uuid.clone(),
            cwd: view.current_working_directory(app),
            shell_launch_data: view.active_shell_launch_data(),
            is_active: self.has_application_focus_for_snapshot(app),
        })
    }

    fn has_application_focus(&self, ctx: &mut ViewContext<PaneGroup>) -> bool {
        self.view.is_self_or_child_focused(ctx)
    }

    fn focus(&self, ctx: &mut ViewContext<PaneGroup>) {
        self.terminal_view(ctx)
            .update(ctx, |view, ctx| view.focus(ctx));
    }

    fn shareable_link(
        &self,
        _ctx: &mut ViewContext<PaneGroup>,
    ) -> Result<ShareableLink, ShareableLinkError> {
        Err(ShareableLinkError::Unexpected(
            "Local terminal sessions do not have shareable URLs".to_owned(),
        ))
    }

    fn pane_configuration(&self) -> ModelHandle<PaneConfiguration> {
        self.pane_configuration.clone()
    }

    fn is_pane_being_dragged(&self, ctx: &AppContext) -> bool {
        self.view.as_ref(ctx).is_being_dragged()
    }
}

impl TerminalPane {
    fn has_application_focus_for_snapshot(&self, app: &AppContext) -> bool {
        self.view.is_self_or_child_focused(app)
    }
}
