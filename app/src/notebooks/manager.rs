use std::collections::HashMap;

use warpui::{Entity, EntityId, ModelContext, SingletonEntity, WeakViewHandle, WindowId};

use crate::notebooks::model::NotebookId;
use crate::notebooks::notebook::NotebookView;
use crate::pane_group::NotebookPane;
use crate::workspace::PaneViewLocator;

pub struct NotebookManager {
    panes: HashMap<NotebookId, NotebookPaneData>,
}

#[derive(Debug, Clone)]
pub enum NotebookSource {
    Existing(NotebookId),
    New { title: Option<String> },
}

impl NotebookManager {
    pub fn new_local(_ctx: &mut ModelContext<Self>) -> Self {
        Self {
            panes: HashMap::new(),
        }
    }

    pub fn find_pane(&self, source: &NotebookSource) -> Option<(WindowId, PaneViewLocator)> {
        let NotebookSource::Existing(id) = source else {
            return None;
        };
        self.panes
            .get(id)
            .map(|pane| (pane.window_id, pane.locator))
    }

    pub fn create_pane(
        &mut self,
        source: &NotebookSource,
        window_id: WindowId,
        ctx: &mut ModelContext<Self>,
    ) -> NotebookPane {
        let view = ctx.add_typed_action_view(window_id, NotebookView::new);
        match source {
            NotebookSource::Existing(id) => {
                let id = id.clone();
                view.update(ctx, |view, ctx| {
                    view.load(id, ctx);
                });
            }
            NotebookSource::New { title } => {
                let title = title.clone();
                view.update(ctx, |view, ctx| view.open_new(title, ctx));
            }
        }
        NotebookPane::new(view, ctx)
    }

    pub fn register_pane(
        &mut self,
        pane: &NotebookPane,
        pane_group_id: EntityId,
        window_id: WindowId,
        ctx: &mut ModelContext<Self>,
    ) {
        let Some(id) = pane.notebook_view(ctx).as_ref(ctx).notebook_id() else {
            return;
        };
        self.panes.insert(
            id.clone(),
            NotebookPaneData {
                window_id,
                locator: PaneViewLocator {
                    pane_group_id,
                    pane_id: pane.id(),
                },
                handle: pane.notebook_view(ctx).downgrade(),
            },
        );
    }

    pub fn deregister_pane(&mut self, pane: &NotebookPane, ctx: &mut ModelContext<Self>) {
        let Some(id) = pane.notebook_view(ctx).as_ref(ctx).notebook_id() else {
            return;
        };
        self.panes.remove(&id);
    }

    pub fn close_notebooks(&self, ctx: &mut ModelContext<Self>) {
        for pane in self.panes.values() {
            if let Some(view) = pane.handle.upgrade(ctx) {
                view.update(ctx, |view, ctx| view.on_detach(ctx));
            }
        }
    }
}

struct NotebookPaneData {
    window_id: WindowId,
    handle: WeakViewHandle<NotebookView>,
    locator: PaneViewLocator,
}

impl Entity for NotebookManager {
    type Event = ();
}

impl SingletonEntity for NotebookManager {}
