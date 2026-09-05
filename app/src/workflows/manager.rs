use std::collections::HashMap;

use warpui::{Entity, EntityId, ModelContext, SingletonEntity, WindowId};

use crate::pane_group::{PaneContent as _, WorkflowPane};
use crate::workflows::workflow_view::WorkflowView;
use crate::workflows::{Workflow, WorkflowId, WorkflowViewMode};
use crate::workspace::PaneViewLocator;

pub struct WorkflowManager {
    panes: HashMap<WorkflowId, WorkflowPaneData>,
}

#[derive(Debug, Clone)]
pub enum WorkflowOpenSource {
    Existing { id: WorkflowId, workflow: Workflow },
    New { title: Option<String>, command: Option<String> },
    NewFromWorkflow { workflow: Box<Workflow> },
}

impl WorkflowManager {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self {
            panes: HashMap::new(),
        }
    }

    pub fn find_pane(&self, source: &WorkflowOpenSource) -> Option<(WindowId, PaneViewLocator)> {
        let id = match source {
            WorkflowOpenSource::Existing { id, .. } => id,
            WorkflowOpenSource::New { .. } | WorkflowOpenSource::NewFromWorkflow { .. } => {
                return None;
            }
        };
        self.panes.get(id).map(|pane| (pane.window_id, pane.locator))
    }

    pub fn create_pane(
        &mut self,
        source: &WorkflowOpenSource,
        mode: WorkflowViewMode,
        window_id: WindowId,
        ctx: &mut ModelContext<Self>,
    ) -> WorkflowPane {
        let view = ctx.add_typed_action_view(window_id, WorkflowView::new_in_pane);
        match source {
            WorkflowOpenSource::Existing { workflow, .. }
            | WorkflowOpenSource::NewFromWorkflow { workflow } => {
                let workflow = (**workflow).clone();
                view.update(ctx, |view, ctx| view.load(workflow, mode, ctx));
            }
            WorkflowOpenSource::New { title, command } => {
                let title = title.clone();
                let command = command.clone();
                view.update(ctx, |view, ctx| view.open_new_workflow(title, command, ctx));
            }
        }
        WorkflowPane::new(view, ctx)
    }

    pub fn register_pane(
        &mut self,
        pane: &WorkflowPane,
        pane_group_id: EntityId,
        window_id: WindowId,
        ctx: &mut ModelContext<Self>,
    ) {
        let id = pane.get_view(ctx).as_ref(ctx).workflow_id();
        self.panes.insert(
            id,
            WorkflowPaneData {
                window_id,
                locator: PaneViewLocator {
                    pane_group_id,
                    pane_id: pane.id(),
                },
            },
        );
    }

    pub fn deregister_pane(&mut self, pane: &WorkflowPane, ctx: &mut ModelContext<Self>) {
        let id = pane.get_view(ctx).as_ref(ctx).workflow_id();
        self.panes.remove(&id);
    }
}

struct WorkflowPaneData {
    window_id: WindowId,
    locator: PaneViewLocator,
}

impl Entity for WorkflowManager {
    type Event = ();
}

impl SingletonEntity for WorkflowManager {}
