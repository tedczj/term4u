use std::collections::HashMap;
use std::sync::Arc;

use warpui::{AppContext, ModelHandle, SingletonEntity, ViewContext, ViewHandle};

use super::{
    DetachType, PaneConfiguration, PaneContent, PaneGroup, PaneId, PaneView, ShareableLink,
    ShareableLinkError,
};
use crate::app_state::{LeafContents, WorkflowPaneSnapshot};
use crate::workflows::manager::{WorkflowManager, WorkflowOpenSource};
use crate::workflows::workflow_view::{WorkflowView, WorkflowViewEvent};
use crate::workflows::{
    Workflow, WorkflowId, WorkflowSelectionSource, WorkflowSource, WorkflowType, WorkflowViewMode,
};

pub struct WorkflowPane {
    view: ViewHandle<PaneView<WorkflowView>>,
    pane_configuration: ModelHandle<PaneConfiguration>,
}

impl WorkflowPane {
    pub fn new(view: ViewHandle<WorkflowView>, ctx: &mut AppContext) -> Self {
        let pane_configuration = view.as_ref(ctx).pane_configuration().clone();
        let view = ctx.add_typed_action_view(view.window_id(ctx), |ctx| {
            PaneView::new(
                PaneId::from_workflow_pane_ctx(ctx),
                view,
                (),
                pane_configuration.clone(),
                ctx,
            )
        });
        Self {
            view,
            pane_configuration,
        }
    }

    pub fn restore(
        id: WorkflowId,
        workflow: Workflow,
        ctx: &mut ViewContext<PaneGroup>,
    ) -> Self {
        let source = WorkflowOpenSource::Existing { id, workflow };
        WorkflowManager::handle(ctx).update(ctx, |manager, ctx| {
            manager.create_pane(&source, WorkflowViewMode::Edit, ctx.window_id(), ctx)
        })
    }

    pub fn get_view(&self, ctx: &AppContext) -> ViewHandle<WorkflowView> {
        self.view.as_ref(ctx).child(ctx)
    }
}

impl PaneContent for WorkflowPane {
    fn id(&self) -> PaneId {
        PaneId::from_workflow_pane_view(&self.view)
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
        ctx.subscribe_to_view(&self.view, move |group, _, event, ctx| {
            group.handle_pane_view_event(pane_id, event, ctx);
        });
        ctx.subscribe_to_view(&self.get_view(ctx), move |group, _, event, ctx| {
            match event {
                WorkflowViewEvent::Pane(event) => group.handle_pane_event(pane_id, event, ctx),
                WorkflowViewEvent::RunWorkflow {
                    workflow,
                    source,
                    argument_override,
                } => ctx.emit(crate::pane_group::Event::RunWorkflow {
                    workflow: workflow.clone(),
                    workflow_source: *source,
                    argument_override: argument_override.clone(),
                    workflow_selection_source: WorkflowSelectionSource::WorkflowView,
                }),
                WorkflowViewEvent::UpdatedWorkflow(_) => {}
            }
        });
        WorkflowManager::handle(ctx).update(ctx, |manager, ctx| {
            manager.register_pane(self, ctx.view_id(), ctx.window_id(), ctx);
        });
    }

    fn detach(
        &self,
        _group: &PaneGroup,
        _detach_type: DetachType,
        ctx: &mut ViewContext<PaneGroup>,
    ) {
        ctx.unsubscribe_to_view(&self.view);
        ctx.unsubscribe_to_view(&self.get_view(ctx));
        WorkflowManager::handle(ctx).update(ctx, |manager, ctx| manager.deregister_pane(self, ctx));
    }

    fn snapshot(&self, app: &AppContext) -> LeafContents {
        let view = self.get_view(app);
        LeafContents::Workflow(WorkflowPaneSnapshot::LocalWorkflow {
            workflow_id: view.as_ref(app).workflow_id(),
            workflow: view.as_ref(app).workflow(app),
        })
    }

    fn has_application_focus(&self, ctx: &mut ViewContext<PaneGroup>) -> bool {
        self.view.is_self_or_child_focused(ctx)
    }

    fn focus(&self, ctx: &mut ViewContext<PaneGroup>) {
        self.get_view(ctx).update(ctx, |view, ctx| view.focus(ctx));
    }

    fn shareable_link(
        &self,
        _ctx: &mut ViewContext<PaneGroup>,
    ) -> Result<ShareableLink, ShareableLinkError> {
        Err(ShareableLinkError::Unexpected(
            "Local workflows do not have shareable URLs".to_owned(),
        ))
    }

    fn pane_configuration(&self) -> ModelHandle<PaneConfiguration> {
        self.pane_configuration.clone()
    }

    fn is_pane_being_dragged(&self, ctx: &AppContext) -> bool {
        self.view.as_ref(ctx).is_being_dragged()
    }
}
