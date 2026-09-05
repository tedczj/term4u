use warpui::elements::{ChildView, Expanded, Flex, ParentElement};
use warpui::{
    AppContext, Element, Entity, TypedActionView, View, ViewContext, ViewHandle,
};

use crate::editor::{EditorView, Event as EditorEvent, SingleLineEditorOptions};
use crate::local_objects::notebook_store::NotebookStore;
use crate::menu::{MenuItem, MenuItemFields};
use crate::notebooks::model::{Notebook, NotebookId};
use crate::pane_group::focus_state::PaneFocusHandle;
use crate::pane_group::pane::view;
use crate::pane_group::{BackingView, PaneConfiguration, PaneEvent};
use crate::{safe_error, safe_warn};

pub fn init(_app: &mut AppContext) {}

#[derive(Debug, Clone)]
pub enum NotebookEvent {
    Pane(PaneEvent),
}

impl From<PaneEvent> for NotebookEvent {
    fn from(event: PaneEvent) -> Self {
        Self::Pane(event)
    }
}

#[derive(Debug, Clone)]
pub enum NotebookAction {
    Focus,
    Close,
    ToggleMaximized,
}

pub struct NotebookView {
    id: Option<NotebookId>,
    title: ViewHandle<EditorView>,
    body: ViewHandle<EditorView>,
    pane_configuration: warpui::ModelHandle<PaneConfiguration>,
    focus_handle: Option<PaneFocusHandle>,
}

impl NotebookView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let title = ctx.add_typed_action_view(|ctx| {
            let mut editor = EditorView::single_line(SingleLineEditorOptions::default(), ctx);
            editor.set_placeholder_text("Untitled", ctx);
            editor
        });
        let body = ctx.add_typed_action_view(|ctx| EditorView::new(Default::default(), ctx));
        ctx.subscribe_to_view(&title, |view, _, event, ctx| {
            if matches!(event, EditorEvent::Edited(_)) {
                view.persist(ctx);
            }
        });
        ctx.subscribe_to_view(&body, |view, _, event, ctx| {
            if matches!(event, EditorEvent::Edited(_)) {
                view.persist(ctx);
            }
        });
        let pane_configuration = ctx.add_model(|_| PaneConfiguration::new("Untitled"));
        Self {
            id: None,
            title,
            body,
            pane_configuration,
            focus_handle: None,
        }
    }

    pub fn load(&mut self, id: NotebookId, ctx: &mut ViewContext<Self>) -> bool {
        let Some(notebook) = NotebookStore::as_ref(ctx).get(&id).cloned() else {
            safe_warn!(
                safe: ("Local notebook could not be restored"),
                full: ("Local notebook {id} could not be restored")
            );
            return false;
        };
        self.id = Some(id);
        self.title.update(ctx, |editor, ctx| {
            editor.system_reset_buffer_text(&notebook.title, ctx)
        });
        self.body.update(ctx, |editor, ctx| {
            editor.system_reset_buffer_text(&notebook.data, ctx)
        });
        self.pane_configuration.update(ctx, |configuration, ctx| {
            configuration.set_title(notebook.title, ctx)
        });
        true
    }

    pub fn open_new(&mut self, title: Option<String>, ctx: &mut ViewContext<Self>) {
        let title = title.unwrap_or_default();
        match NotebookStore::handle(ctx).update(ctx, |store, _| store.create(title.clone())) {
            Ok(id) => {
                self.id = Some(id);
                self.title.update(ctx, |editor, ctx| {
                    editor.system_reset_buffer_text(&title, ctx)
                });
                self.pane_configuration.update(ctx, |configuration, ctx| {
                    configuration.set_title(
                        if title.is_empty() {
                            "Untitled".to_owned()
                        } else {
                            title
                        },
                        ctx,
                    )
                });
            }
            Err(error) => safe_error!(
                safe: ("Failed to create local notebook"),
                full: ("Failed to create local notebook: {error:#}")
            ),
        }
    }

    fn persist(&mut self, ctx: &mut ViewContext<Self>) {
        let Some(id) = self.id.clone() else {
            return;
        };
        let title = self.title.as_ref(ctx).buffer_text(ctx);
        let data = self.body.as_ref(ctx).buffer_text(ctx);
        let notebook = Notebook {
            id,
            title: title.clone(),
            data,
        };
        if let Err(error) = NotebookStore::handle(ctx).update(ctx, |store, _| store.upsert(notebook))
        {
            safe_error!(
                safe: ("Failed to save local notebook"),
                full: ("Failed to save local notebook: {error:#}")
            );
            return;
        }
        self.pane_configuration.update(ctx, |configuration, ctx| {
            configuration.set_title(
                if title.is_empty() {
                    "Untitled".to_owned()
                } else {
                    title
                },
                ctx,
            )
        });
    }

    pub fn focus(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.focus(&self.body);
    }

    pub fn notebook_id(&self) -> Option<NotebookId> {
        self.id.clone()
    }

    pub fn pane_configuration(&self) -> &warpui::ModelHandle<PaneConfiguration> {
        &self.pane_configuration
    }

    pub fn on_detach(&mut self, ctx: &mut ViewContext<Self>) {
        self.persist(ctx);
    }
}

impl Entity for NotebookView {
    type Event = NotebookEvent;
}

impl View for NotebookView {
    fn ui_name() -> &'static str {
        "NotebookView"
    }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        Flex::column()
            .child(ChildView::new(&self.title).finish())
            .child(Expanded::new(1., ChildView::new(&self.body).finish()).finish())
            .finish()
    }
}

impl TypedActionView for NotebookView {
    type Action = NotebookAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            NotebookAction::Focus => self.focus(ctx),
            NotebookAction::Close => ctx.emit(NotebookEvent::Pane(PaneEvent::Close)),
            NotebookAction::ToggleMaximized => {
                ctx.emit(NotebookEvent::Pane(PaneEvent::ToggleMaximized))
            }
        }
    }
}

impl BackingView for NotebookView {
    type PaneHeaderOverflowMenuAction = NotebookAction;
    type CustomAction = ();
    type AssociatedData = ();

    fn handle_pane_header_overflow_menu_action(
        &mut self,
        action: &Self::PaneHeaderOverflowMenuAction,
        ctx: &mut ViewContext<Self>,
    ) {
        self.handle_action(action, ctx);
    }

    fn pane_header_overflow_menu_items(&self, app: &AppContext) -> Vec<MenuItem<NotebookAction>> {
        let is_maximized = self
            .focus_handle
            .as_ref()
            .is_some_and(|handle| handle.is_maximized(app));
        vec![
            MenuItemFields::toggle_pane_action(is_maximized)
                .with_on_select_action(NotebookAction::ToggleMaximized)
                .into_item(),
        ]
    }

    fn close(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.emit(NotebookEvent::Pane(PaneEvent::Close));
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

    fn set_focus_handle(&mut self, focus_handle: PaneFocusHandle, _ctx: &mut ViewContext<Self>) {
        self.focus_handle = Some(focus_handle);
    }
}
