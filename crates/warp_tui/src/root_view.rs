use warpui::SingletonEntity;
use warpui_core::elements::tui::{TuiChildView, TuiElement, TuiFlex, TuiText};
use warpui_core::keymap::macros::*;
use warpui_core::keymap::{EditableBinding, FixedBinding};
use warpui_core::platform::TerminationMode;
use warpui_core::{AppContext, Entity, EntityId, TuiView, TypedActionView, ViewContext, keymap};

use crate::keybindings::TUI_BINDING_GROUP;
use crate::session_registry::TuiSessions;
use crate::tui_builder::TuiUiBuilder;
use crate::zero_state::local_zero_state;

#[derive(Debug, Clone)]
pub enum RootTuiAction {
    ExitApp,
    NewTab,
    CloseTab,
    NextTab,
    PreviousTab,
}

pub struct RootTuiView;

pub fn init(app: &mut AppContext) {
    let context = id!(RootTuiView::ui_name());
    app.register_fixed_bindings([FixedBinding::new(
        "ctrl-q",
        RootTuiAction::ExitApp,
        context.clone(),
    )
    .with_group(TUI_BINDING_GROUP)]);
    app.register_editable_bindings([
        EditableBinding::new(
            "tui:new_tab",
            "New local terminal tab",
            RootTuiAction::NewTab,
        )
        .with_context_predicate(context.clone())
        .with_group(TUI_BINDING_GROUP)
        .with_key_binding("ctrl-t"),
        EditableBinding::new(
            "tui:close_tab",
            "Close local terminal tab",
            RootTuiAction::CloseTab,
        )
        .with_context_predicate(context.clone())
        .with_group(TUI_BINDING_GROUP)
        .with_key_binding("ctrl-w"),
        EditableBinding::new(
            "tui:next_tab",
            "Next local terminal tab",
            RootTuiAction::NextTab,
        )
        .with_context_predicate(context.clone())
        .with_group(TUI_BINDING_GROUP)
        .with_key_binding("ctrl-tab"),
        EditableBinding::new(
            "tui:previous_tab",
            "Previous local terminal tab",
            RootTuiAction::PreviousTab,
        )
        .with_context_predicate(context)
        .with_group(TUI_BINDING_GROUP)
        .with_key_binding("ctrl-shift-tab"),
    ]);
}

impl RootTuiView {
    pub(crate) fn new(_ctx: &mut ViewContext<Self>) -> Self {
        Self
    }

    pub(crate) fn activate(&mut self, ctx: &mut ViewContext<Self>) {
        if !ctx.has_singleton_model::<TuiSessions>() {
            ctx.focus_self();
            return;
        }
        if let Some(view) = TuiSessions::as_ref(ctx)
            .focused_session()
            .map(|session| session.view().clone())
        {
            view.update(ctx, |view, ctx| view.activate(ctx));
        } else {
            ctx.focus_self();
        }
    }
}

impl Entity for RootTuiView {
    type Event = ();
}

impl TuiView for RootTuiView {
    fn ui_name() -> &'static str {
        "RootTuiView"
    }

    fn child_view_ids(&self, ctx: &AppContext) -> Vec<EntityId> {
        if !ctx.has_singleton_model::<TuiSessions>() {
            return Vec::new();
        }
        TuiSessions::as_ref(ctx)
            .focused_session()
            .map(|session| vec![session.view().id()])
            .unwrap_or_default()
    }

    fn render(&self, ctx: &AppContext) -> Box<dyn TuiElement> {
        if !ctx.has_singleton_model::<TuiSessions>() {
            return local_zero_state(ctx);
        }
        let sessions = TuiSessions::as_ref(ctx);
        let Some(session) = sessions.focused_session() else {
            return local_zero_state(ctx);
        };
        let builder = TuiUiBuilder::from_app(ctx);
        let active = sessions.focused_index().unwrap_or_default() + 1;
        let header = TuiText::new(format!(
            "Term4u local terminal  [{active}/{}]  ctrl-t new  ctrl-w close  ctrl-q exit",
            sessions.session_count()
        ))
        .with_style(builder.muted_text_style())
        .truncate()
        .finish();
        TuiFlex::column()
            .child(header)
            .flex_child(TuiChildView::new(session.view()).finish())
            .finish()
    }

    fn keymap_context(&self, _ctx: &AppContext) -> keymap::Context {
        let mut context = keymap::Context::default();
        context.set.insert(Self::ui_name());
        context
    }
}

impl TypedActionView for RootTuiView {
    type Action = RootTuiAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            RootTuiAction::ExitApp => {
                ctx.terminate_app(TerminationMode::ForceTerminate, None);
            }
            RootTuiAction::NewTab => {
                let sessions = TuiSessions::handle(ctx);
                TuiSessions::create_local_terminal_session(
                    &sessions,
                    ctx.window_id(),
                    true,
                    std::env::current_dir().ok(),
                    ctx,
                );
            }
            RootTuiAction::CloseTab => {
                TuiSessions::handle(ctx).update(ctx, |sessions, ctx| sessions.close_focused(ctx));
                if TuiSessions::as_ref(ctx).session_count() == 0 {
                    ctx.terminate_app(TerminationMode::ForceTerminate, None);
                }
            }
            RootTuiAction::NextTab => {
                TuiSessions::handle(ctx).update(ctx, |sessions, ctx| sessions.focus_next(ctx));
            }
            RootTuiAction::PreviousTab => {
                TuiSessions::handle(ctx).update(ctx, |sessions, ctx| sessions.focus_previous(ctx));
            }
        }
    }
}
