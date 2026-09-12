use std::path::{Path, PathBuf};

use warpui::{AppContext, ModelHandle, ViewContext, ViewHandle};

use super::{
    BackingView, DetachType, PaneConfiguration, PaneContent, PaneGroup, PaneId, PaneView,
    ShareableLink, ShareableLinkError,
};
use crate::app_state::{CodeReviewPaneSnapshot, LeafContents};
use crate::code::buffer_location::LocalOrRemotePath;
use crate::code_review::code_review_view::{CodeReviewView, CodeReviewViewEvent};
use crate::code_review::diff_state::DiffStateModel;

pub struct CodeReviewPane {
    view: ViewHandle<PaneView<CodeReviewView>>,
    pane_configuration: ModelHandle<PaneConfiguration>,
    repo_path: PathBuf,
    terminal_uuid: Vec<u8>,
}

impl CodeReviewPane {
    pub fn new(
        repo_path: PathBuf,
        terminal_uuid: Vec<u8>,
        ctx: &mut ViewContext<PaneGroup>,
    ) -> Self {
        let diff_model = ctx.add_model(|ctx| DiffStateModel::new_local(repo_path.clone(), ctx));
        let review = ctx.add_typed_action_view(|ctx| {
            CodeReviewView::new(
                Some(LocalOrRemotePath::Local(repo_path.clone())),
                diff_model,
                None,
                None,
                ctx,
            )
        });
        let pane_configuration = ctx.add_model(|_| PaneConfiguration::new("Code Review"));
        let view = ctx.add_typed_action_view(|ctx| {
            let pane_id = PaneId::from_code_review_pane_ctx(ctx);
            review.update(ctx, |review, _| review.set_pane_id(pane_id));
            PaneView::new(pane_id, review, (), pane_configuration.clone(), ctx)
        });
        Self {
            view,
            pane_configuration,
            repo_path,
            terminal_uuid,
        }
    }

    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }

    pub fn close(&self, ctx: &mut ViewContext<PaneGroup>) {
        self.review(ctx)
            .update(ctx, |review, ctx| review.close(ctx));
    }

    fn review(&self, ctx: &AppContext) -> ViewHandle<CodeReviewView> {
        self.view.as_ref(ctx).child(ctx)
    }
}

impl PaneContent for CodeReviewPane {
    fn id(&self) -> PaneId {
        PaneId::from_code_review_pane_view(&self.view)
    }

    fn attach(
        &self,
        _group: &PaneGroup,
        focus_handle: crate::pane_group::focus_state::PaneFocusHandle,
        ctx: &mut ViewContext<PaneGroup>,
    ) {
        let pane_id = self.id();
        self.view
            .update(ctx, |view, ctx| view.set_focus_handle(focus_handle, ctx));
        let review = self.review(ctx);
        review.update(ctx, |review, ctx| review.on_open(ctx));
        ctx.subscribe_to_view(&review, move |group, _, event, ctx| match event {
            CodeReviewViewEvent::Pane(event) => group.handle_pane_event(pane_id, event, ctx),
            CodeReviewViewEvent::OpenFileWithTarget {
                path,
                target,
                line_col,
            } => ctx.emit(crate::pane_group::Event::OpenFileWithTarget {
                path: path.clone(),
                target: target.clone(),
                line_col: *line_col,
            }),
            CodeReviewViewEvent::OpenFileInNewTab {
                path,
                line_and_column,
            } => {
                if let Some(path) = path.to_local_path() {
                    ctx.emit(crate::pane_group::Event::OpenFileWithTarget {
                        path: path.to_owned(),
                        target: crate::util::openable_file_type::FileTarget::CodeEditor(
                            crate::util::openable_file_type::EditorLayout::NewTab,
                        ),
                        line_col: *line_and_column,
                    });
                }
            }
            CodeReviewViewEvent::OpenLspLogs { log_path } => {
                ctx.emit(crate::pane_group::Event::OpenLspLogs {
                    log_path: log_path.clone(),
                })
            }
        });
        ctx.subscribe_to_view(&self.view, move |group, _, event, ctx| {
            group.handle_pane_view_event(pane_id, event, ctx)
        });
    }

    fn detach(
        &self,
        _group: &PaneGroup,
        _detach_type: DetachType,
        ctx: &mut ViewContext<PaneGroup>,
    ) {
        let review = self.review(ctx);
        ctx.unsubscribe_to_view(&review);
        ctx.unsubscribe_to_view(&self.view);
        review.update(ctx, |review, ctx| review.on_close(ctx));
    }

    fn snapshot(&self, _app: &AppContext) -> LeafContents {
        LeafContents::CodeReview(CodeReviewPaneSnapshot::Local {
            repo_path: self.repo_path.clone(),
            terminal_uuid: self.terminal_uuid.clone(),
        })
    }

    fn has_application_focus(&self, ctx: &mut ViewContext<PaneGroup>) -> bool {
        self.view.is_self_or_child_focused(ctx)
    }
    fn focus(&self, ctx: &mut ViewContext<PaneGroup>) {
        self.review(ctx)
            .update(ctx, |review, ctx| review.focus_contents(ctx));
    }
    fn shareable_link(
        &self,
        _ctx: &mut ViewContext<PaneGroup>,
    ) -> Result<ShareableLink, ShareableLinkError> {
        Ok(ShareableLink::Base)
    }
    fn pane_configuration(&self) -> ModelHandle<PaneConfiguration> {
        self.pane_configuration.clone()
    }
    fn is_pane_being_dragged(&self, ctx: &AppContext) -> bool {
        self.view.as_ref(ctx).is_being_dragged()
    }
}
