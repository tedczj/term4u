use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

use warpui::elements::{ChildView, Container, Flex, ParentElement};
use warpui::{
    AppContext, Element, Entity, ModelHandle, TypedActionView, View, ViewContext, ViewHandle,
};

use crate::editor::{EditorView, Event as EditorEvent, SingleLineEditorOptions};
use crate::menu::{MenuItem, MenuItemFields};
use crate::pane_group::focus_state::PaneFocusHandle;
use crate::pane_group::pane::view;
use crate::pane_group::{BackingView, PaneConfiguration, PaneEvent};
use crate::safe_error;
use crate::workflows::{Workflow, WorkflowId, WorkflowSource, WorkflowType, WorkflowViewMode};

pub fn init(_app: &mut AppContext) {}

#[derive(Debug, Clone)]
pub enum WorkflowViewEvent {
    Pane(PaneEvent),
    RunWorkflow {
        workflow: Arc<WorkflowType>,
        source: WorkflowSource,
        argument_override: Option<HashMap<String, String>>,
    },
    UpdatedWorkflow(WorkflowId),
}

#[derive(Debug, Clone)]
pub enum WorkflowViewAction {
    Focus,
    Run,
    Close,
    ToggleMaximized,
}

pub struct WorkflowView {
    id: WorkflowId,
    name: ViewHandle<EditorView>,
    command: ViewHandle<EditorView>,
    pane_configuration: ModelHandle<PaneConfiguration>,
    focus_handle: Option<PaneFocusHandle>,
}

impl WorkflowView {
    pub fn new_in_pane(ctx: &mut ViewContext<Self>) -> Self {
        let name = ctx.add_typed_action_view(|ctx| {
            EditorView::single_line(SingleLineEditorOptions::default(), ctx)
        });
        let command = ctx.add_typed_action_view(|ctx| EditorView::new(Default::default(), ctx));
        ctx.subscribe_to_view(&name, |view, _, event, ctx| {
            if matches!(event, EditorEvent::Edited(_)) {
                view.persist(ctx);
            }
        });
        ctx.subscribe_to_view(&command, |view, _, event, ctx| {
            if matches!(event, EditorEvent::Edited(_)) {
                view.persist(ctx);
            }
        });
        Self {
            id: WorkflowId::new(),
            name,
            command,
            pane_configuration: ctx.add_model(|_| PaneConfiguration::new("Workflow")),
            focus_handle: None,
        }
    }

    pub fn load(&mut self, workflow: Workflow, _mode: WorkflowViewMode, ctx: &mut ViewContext<Self>) {
        self.name.update(ctx, |editor, ctx| {
            editor.system_reset_buffer_text(&workflow.name, ctx)
        });
        self.command.update(ctx, |editor, ctx| {
            editor.system_reset_buffer_text(&workflow.command, ctx)
        });
        self.pane_configuration.update(ctx, |configuration, ctx| {
            configuration.set_title(workflow.name, ctx)
        });
    }

    pub fn open_new_workflow(
        &mut self,
        title: Option<String>,
        command: Option<String>,
        ctx: &mut ViewContext<Self>,
    ) {
        self.load(
            Workflow::new(title.unwrap_or_default(), command.unwrap_or_default()),
            WorkflowViewMode::Create,
            ctx,
        );
        self.persist(ctx);
    }

    pub fn workflow_id(&self) -> WorkflowId {
        self.id.clone()
    }

    pub fn workflow(&self, app: &AppContext) -> Workflow {
        Workflow::new(
            self.name.as_ref(app).buffer_text(app),
            self.command.as_ref(app).buffer_text(app),
        )
    }

    pub fn pane_configuration(&self) -> &ModelHandle<PaneConfiguration> {
        &self.pane_configuration
    }

    pub fn workflow_link(&self, _app: &AppContext) -> Option<String> {
        None
    }

    pub fn focus(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.focus(&self.command);
    }

    fn persist(&mut self, ctx: &mut ViewContext<Self>) {
        let workflow = self.workflow(ctx);
        let directory = crate::user_config::workflows_dir();
        let result = (|| -> anyhow::Result<()> {
            fs::create_dir_all(&directory)?;
            let path = directory.join(format!("{}.yaml", self.id));
            fs::write(path, serde_yaml::to_string(&workflow)?)?;
            Ok(())
        })();
        if let Err(error) = result {
            safe_error!(
                safe: ("Failed to save local workflow"),
                full: ("Failed to save local workflow: {error:#}")
            );
            return;
        }
        self.pane_configuration.update(ctx, |configuration, ctx| {
            configuration.set_title(
                if workflow.name.is_empty() {
                    "Workflow".to_owned()
                } else {
                    workflow.name.clone()
                },
                ctx,
            )
        });
        ctx.emit(WorkflowViewEvent::UpdatedWorkflow(self.id.clone()));
    }
}

impl Entity for WorkflowView {
    type Event = WorkflowViewEvent;
}

impl View for WorkflowView {
    fn ui_name() -> &'static str {
        "WorkflowView"
    }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        Container::new(
            Flex::column()
                .child(ChildView::new(&self.name).finish())
                .child(ChildView::new(&self.command).finish())
                .finish(),
        )
        .with_uniform_padding(16.)
        .finish()
    }
}

impl TypedActionView for WorkflowView {
    type Action = WorkflowViewAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            WorkflowViewAction::Focus => self.focus(ctx),
            WorkflowViewAction::Run => ctx.emit(WorkflowViewEvent::RunWorkflow {
                workflow: Arc::new(WorkflowType::Local(self.workflow(ctx))),
                source: WorkflowSource::Local,
                argument_override: None,
            }),
            WorkflowViewAction::Close => ctx.emit(WorkflowViewEvent::Pane(PaneEvent::Close)),
            WorkflowViewAction::ToggleMaximized => {
                ctx.emit(WorkflowViewEvent::Pane(PaneEvent::ToggleMaximized))
            }
        }
    }
}

impl BackingView for WorkflowView {
    type PaneHeaderOverflowMenuAction = WorkflowViewAction;
    type CustomAction = ();
    type AssociatedData = ();

    fn handle_pane_header_overflow_menu_action(
        &mut self,
        action: &Self::PaneHeaderOverflowMenuAction,
        ctx: &mut ViewContext<Self>,
    ) {
        self.handle_action(action, ctx);
    }

    fn pane_header_overflow_menu_items(&self, app: &AppContext) -> Vec<MenuItem<WorkflowViewAction>> {
        let is_maximized = self
            .focus_handle
            .as_ref()
            .is_some_and(|handle| handle.is_maximized(app));
        vec![
            MenuItemFields::toggle_pane_action(is_maximized)
                .with_on_select_action(WorkflowViewAction::ToggleMaximized)
                .into_item(),
        ]
    }

    fn close(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.emit(WorkflowViewEvent::Pane(PaneEvent::Close));
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
