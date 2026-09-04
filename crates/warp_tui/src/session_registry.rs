use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;

use pathfinder_geometry::vector::Vector2F;
use warp::tui_export::{
    BannerState, IsSharedSessionCreator, LocalTtyTerminalManager, PersistenceWriter,
    TerminalManagerTrait, TerminalSurfaceResult,
};
use warpui::SingletonEntity;
use warpui_core::runtime::TuiDriverHandle;
use warpui_core::{AppContext, Entity, EntityId, ModelContext, ModelHandle, ViewHandle, WindowId};

use crate::terminal_session_view::TuiTerminalSessionView;
use crate::transcript_view::TRANSCRIPT_BLOCK_SPACING;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct TuiSessionId(EntityId);

pub(crate) struct TuiSession {
    id: TuiSessionId,
    view: ViewHandle<TuiTerminalSessionView>,
    _manager: ModelHandle<Box<dyn TerminalManagerTrait>>,
}

impl TuiSession {
    pub(crate) fn view(&self) -> &ViewHandle<TuiTerminalSessionView> {
        &self.view
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TuiSessionsEvent {
    SessionAdded(TuiSessionId),
    SessionRemoved(TuiSessionId),
    FocusChanged(TuiSessionId),
}

pub(crate) struct TuiSessions {
    _driver: TuiDriverHandle,
    sessions: Vec<TuiSession>,
    focused_session_id: Option<TuiSessionId>,
}

impl Entity for TuiSessions {
    type Event = TuiSessionsEvent;
}

impl SingletonEntity for TuiSessions {}

impl TuiSessions {
    pub(crate) fn new(driver: TuiDriverHandle) -> Self {
        Self {
            _driver: driver,
            sessions: Vec::new(),
            focused_session_id: None,
        }
    }

    pub(crate) fn create_local_terminal_session(
        sessions: &ModelHandle<Self>,
        window_id: WindowId,
        focus: bool,
        startup_directory: Option<PathBuf>,
        ctx: &mut AppContext,
    ) -> (TuiSessionId, ViewHandle<TuiTerminalSessionView>) {
        let banner = ctx.add_model(|_| BannerState::default());
        let model_event_sender = PersistenceWriter::as_ref(ctx).sender();
        let manager = LocalTtyTerminalManager::<TuiTerminalSessionView>::create_tui_model(
            startup_directory,
            HashMap::<OsString, OsString>::from_iter(std::env::vars_os()),
            IsSharedSessionCreator::No,
            None,
            banner,
            Vector2F::new(120., 24.),
            model_event_sender,
            None,
            TRANSCRIPT_BLOCK_SPACING,
            ctx,
            move |surface_init, ctx| {
                let surface = ctx.add_typed_action_tui_view(window_id, |ctx| {
                    TuiTerminalSessionView::new(surface_init, ctx)
                });
                TerminalSurfaceResult {
                    surface,
                    post_wire: no_op_post_wire,
                }
            },
        );

        let surface = manager.surface.clone();
        let id = TuiSessionId(surface.id());
        sessions.update(ctx, |sessions, ctx| {
            sessions.sessions.push(TuiSession {
                id,
                view: manager.surface,
                _manager: manager.manager,
            });
            ctx.emit(TuiSessionsEvent::SessionAdded(id));
            if focus {
                sessions.focus_session(id, ctx);
            }
        });
        (id, surface)
    }

    pub(crate) fn focused_session(&self) -> Option<&TuiSession> {
        let id = self.focused_session_id?;
        self.sessions.iter().find(|session| session.id == id)
    }

    pub(crate) fn session_count(&self) -> usize {
        self.sessions.len()
    }

    pub(crate) fn focused_index(&self) -> Option<usize> {
        let id = self.focused_session_id?;
        self.sessions.iter().position(|session| session.id == id)
    }

    pub(crate) fn focus_next(&mut self, ctx: &mut ModelContext<Self>) {
        if self.sessions.is_empty() {
            return;
        }
        let index = next_session_index(self.sessions.len(), self.focused_index())
            .expect("non-empty session list has a next index");
        self.focus_session(self.sessions[index].id, ctx);
    }

    pub(crate) fn focus_previous(&mut self, ctx: &mut ModelContext<Self>) {
        if self.sessions.is_empty() {
            return;
        }
        let index = previous_session_index(self.sessions.len(), self.focused_index())
            .expect("non-empty session list has a previous index");
        self.focus_session(self.sessions[index].id, ctx);
    }

    pub(crate) fn close_focused(&mut self, ctx: &mut ModelContext<Self>) {
        let Some(index) = self.focused_index() else {
            return;
        };
        let removed = self.sessions.remove(index);
        ctx.emit(TuiSessionsEvent::SessionRemoved(removed.id));
        if self.sessions.is_empty() {
            self.focused_session_id = None;
            return;
        }
        let next = index.min(self.sessions.len() - 1);
        self.focus_session(self.sessions[next].id, ctx);
    }

    fn focus_session(&mut self, id: TuiSessionId, ctx: &mut ModelContext<Self>) {
        if self.focused_session_id == Some(id) {
            return;
        }
        self.focused_session_id = Some(id);
        ctx.emit(TuiSessionsEvent::FocusChanged(id));
        if let Some(session) = self.focused_session() {
            session.view.update(ctx, |view, ctx| view.activate(ctx));
        }
        ctx.notify();
    }
}

fn no_op_post_wire(
    _manager: &mut LocalTtyTerminalManager<TuiTerminalSessionView>,
    _surface: &ViewHandle<TuiTerminalSessionView>,
    _ctx: &mut AppContext,
) {
}

fn next_session_index(len: usize, current: Option<usize>) -> Option<usize> {
    (len > 0).then(|| current.map_or(0, |index| (index + 1) % len))
}

fn previous_session_index(len: usize, current: Option<usize>) -> Option<usize> {
    (len > 0).then(|| {
        current
            .unwrap_or_default()
            .checked_sub(1)
            .unwrap_or(len - 1)
    })
}

#[cfg(test)]
#[path = "session_registry_tests.rs"]
mod tests;
