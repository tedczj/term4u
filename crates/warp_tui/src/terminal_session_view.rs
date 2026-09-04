use std::borrow::Cow;
use std::sync::Arc;

use async_channel::Sender;
use parking_lot::FairMutex;
use warp::tui_export::{
    ModelEvent, PtyIntent, PtyIntentEvent, SizeInfo, SizeUpdate, TerminalModel, TerminalSurface,
    TerminalSurfaceInit, WAKEUP_THROTTLE_PERIOD, throttle,
};
use warpui_core::elements::tui::{
    TuiChildView, TuiContainer, TuiElement, TuiFlex, TuiSize, TuiText,
};
use warpui_core::{AppContext, Entity, EntityId, TuiView, TypedActionView, ViewContext, keymap};

use crate::alt_screen_view::AltScreenElement;
use crate::terminal_content_element::TuiTerminalContentElement;
use crate::transcript_view::TuiTranscriptView;
use crate::zero_state::local_zero_state;

pub(crate) enum TuiTerminalSessionEvent {
    WriteUserInput(Cow<'static, [u8]>),
    Resize(SizeUpdate),
}

impl PtyIntentEvent for TuiTerminalSessionEvent {
    fn pty_intent(&self) -> Option<PtyIntent> {
        match self {
            TuiTerminalSessionEvent::WriteUserInput(bytes) => {
                Some(PtyIntent::WriteBytes(bytes.clone()))
            }
            TuiTerminalSessionEvent::Resize(size) => Some(PtyIntent::Resize(*size)),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum TuiTerminalSessionAction {
    ForwardUserPtyBytes(Vec<u8>),
}

pub(crate) struct TuiTerminalSessionView {
    transcript: warpui_core::ViewHandle<TuiTranscriptView>,
    terminal_model: Arc<FairMutex<TerminalModel>>,
    size_info: SizeInfo,
    terminal_resize_tx: Sender<TuiSize>,
    pty_spawn_error: Option<String>,
}

pub(crate) fn init(_app: &mut AppContext) {}

impl TuiTerminalSessionView {
    pub(crate) fn new(surface_init: TerminalSurfaceInit, ctx: &mut ViewContext<Self>) -> Self {
        let TerminalSurfaceInit {
            model,
            model_events,
            wakeups_rx,
            size_info,
            ..
        } = surface_init;
        let transcript_model = model.clone();
        let transcript = ctx.add_tui_view(move |_| TuiTranscriptView::new(transcript_model));
        ctx.subscribe_to_model(&model_events, |_, _, _: &ModelEvent, ctx| ctx.notify());

        let model_for_wakeups = model.clone();
        ctx.spawn_stream_local(
            throttle(WAKEUP_THROTTLE_PERIOD, wakeups_rx),
            move |_, _, ctx| {
                let mut model = model_for_wakeups.lock();
                if !model.is_alt_screen_active() {
                    model.block_list_mut().update_background_block_height();
                    model.block_list_mut().update_active_block_height();
                }
                drop(model);
                ctx.notify();
            },
            |_, _| {},
        );

        let (terminal_resize_tx, terminal_resize_rx) = async_channel::unbounded();
        ctx.spawn_stream_local(terminal_resize_rx, Self::handle_terminal_resize, |_, _| {});

        Self {
            transcript,
            terminal_model: model,
            size_info,
            terminal_resize_tx,
            pty_spawn_error: None,
        }
    }

    pub(crate) fn activate(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.focus_self();
        ctx.notify();
    }

    fn handle_terminal_resize(&mut self, size: TuiSize, ctx: &mut ViewContext<Self>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        let update = SizeUpdate::from_cell_dimensions(
            self.size_info,
            usize::from(size.height),
            usize::from(size.width),
        );
        if !update.rows_or_columns_changed() {
            return;
        }
        self.terminal_model.lock().resize(update);
        self.size_info = update.new_size();
        ctx.emit(TuiTerminalSessionEvent::Resize(update));
        ctx.notify();
    }
}

impl Entity for TuiTerminalSessionView {
    type Event = TuiTerminalSessionEvent;
}

impl TuiView for TuiTerminalSessionView {
    fn ui_name() -> &'static str {
        "TuiTerminalSessionView"
    }

    fn child_view_ids(&self, _ctx: &AppContext) -> Vec<EntityId> {
        vec![self.transcript.id()]
    }

    fn render(&self, ctx: &AppContext) -> Box<dyn TuiElement> {
        if let Some(error) = &self.pty_spawn_error {
            return TuiContainer::new(
                TuiText::new(format!("Unable to start local shell: {error}"))
                    .truncate()
                    .finish(),
            )
            .with_padding_x(2)
            .with_padding_top(1)
            .finish();
        }

        let content = if self.terminal_model.lock().is_alt_screen_active() {
            AltScreenElement::new(self.terminal_model.clone()).finish()
        } else if self.transcript.as_ref(ctx).is_empty() {
            local_zero_state(ctx)
        } else {
            TuiChildView::new(&self.transcript).finish()
        };
        let terminal = TuiTerminalContentElement::new(self.terminal_resize_tx.clone(), content)
            .with_pty_input(self.terminal_model.clone())
            .finish();
        TuiFlex::column().flex_child(terminal).finish()
    }

    fn keymap_context(&self, _ctx: &AppContext) -> keymap::Context {
        let mut context = keymap::Context::default();
        context.set.insert(Self::ui_name());
        context
    }
}

impl TypedActionView for TuiTerminalSessionView {
    type Action = TuiTerminalSessionAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            TuiTerminalSessionAction::ForwardUserPtyBytes(bytes) => {
                ctx.emit(TuiTerminalSessionEvent::WriteUserInput(Cow::Owned(
                    bytes.clone(),
                )));
            }
        }
    }
}

impl TerminalSurface for TuiTerminalSessionView {
    fn on_shell_determined(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.notify();
    }

    fn on_pty_spawn_failed(&mut self, error: anyhow::Error, ctx: &mut ViewContext<Self>) {
        log::error!("failed to start local TUI shell: {error:#}");
        self.pty_spawn_error = Some(error.to_string());
        ctx.notify();
    }
}

#[cfg(test)]
#[path = "terminal_session_view_tests.rs"]
mod tests;
