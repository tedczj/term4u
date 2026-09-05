use warpui::{AppContext, ModelHandle, SingletonEntity, ViewContext, ViewHandle};

use super::super::PaneGroup;
use super::view::PaneView;
use super::{DetachType, PaneConfiguration, PaneContent, PaneId, ShareableLink, ShareableLinkError};
use crate::app_state::{LeafContents, NotebookPaneSnapshot};
use crate::notebooks::manager::{NotebookManager, NotebookSource};
use crate::notebooks::model::NotebookId;
use crate::notebooks::notebook::{NotebookEvent, NotebookView};

pub struct NotebookPane {
    view: ViewHandle<PaneView<NotebookView>>,
    pane_configuration: ModelHandle<PaneConfiguration>,
}

impl NotebookPane {
    pub fn new(notebook_view: ViewHandle<NotebookView>, ctx: &mut AppContext) -> Self {
        let pane_configuration = notebook_view.as_ref(ctx).pane_configuration().clone();
        let view = ctx.add_typed_action_view(notebook_view.window_id(ctx), |ctx| {
            let pane_id = PaneId::from_notebook_pane_ctx(ctx);
            PaneView::new(pane_id, notebook_view, (), pane_configuration.clone(), ctx)
        });
        Self {
            view,
            pane_configuration,
        }
    }

    pub fn restore(id: Option<NotebookId>, ctx: &mut ViewContext<PaneGroup>) -> Self {
        let source = id.map_or(
            NotebookSource::New { title: None },
            NotebookSource::Existing,
        );
        NotebookManager::handle(ctx).update(ctx, |manager, ctx| {
            manager.create_pane(&source, ctx.window_id(), ctx)
        })
    }

    pub fn notebook_view(&self, ctx: &AppContext) -> ViewHandle<NotebookView> {
        self.view.as_ref(ctx).child(ctx)
    }
}

impl PaneContent for NotebookPane {
    fn id(&self) -> PaneId {
        PaneId::from_notebook_pane_view(&self.view)
    }

    fn snapshot(&self, app: &AppContext) -> LeafContents {
        LeafContents::Notebook(NotebookPaneSnapshot::LocalNotebook {
            notebook_id: self.notebook_view(app).as_ref(app).notebook_id(),
        })
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
        ctx.subscribe_to_view(&self.notebook_view(ctx), move |group, _, event, ctx| {
            let NotebookEvent::Pane(event) = event;
            group.handle_pane_event(pane_id, event, ctx);
        });
        ctx.subscribe_to_view(&self.view, move |group, _, event, ctx| {
            group.handle_pane_view_event(pane_id, event, ctx);
        });
        NotebookManager::handle(ctx).update(ctx, |manager, ctx| {
            manager.register_pane(self, ctx.view_id(), ctx.window_id(), ctx);
        });
    }

    fn detach(
        &self,
        _group: &PaneGroup,
        _detach_type: DetachType,
        ctx: &mut ViewContext<PaneGroup>,
    ) {
        ctx.unsubscribe_to_view(&self.notebook_view(ctx));
        ctx.unsubscribe_to_view(&self.view);
        NotebookManager::handle(ctx).update(ctx, |manager, ctx| manager.deregister_pane(self, ctx));
        self.notebook_view(ctx)
            .update(ctx, |view, ctx| view.on_detach(ctx));
    }

    fn has_application_focus(&self, ctx: &mut ViewContext<PaneGroup>) -> bool {
        self.view.is_self_or_child_focused(ctx)
    }

    fn focus(&self, ctx: &mut ViewContext<PaneGroup>) {
        self.notebook_view(ctx)
            .update(ctx, |view, ctx| view.focus(ctx));
    }

    fn shareable_link(
        &self,
        _ctx: &mut ViewContext<PaneGroup>,
    ) -> Result<ShareableLink, ShareableLinkError> {
        Err(ShareableLinkError::Unexpected(
            "Local notebooks do not have shareable URLs".to_owned(),
        ))
    }

    fn pane_configuration(&self) -> ModelHandle<PaneConfiguration> {
        self.pane_configuration.clone()
    }

    fn is_pane_being_dragged(&self, ctx: &AppContext) -> bool {
        self.view.as_ref(ctx).is_being_dragged()
    }
}
