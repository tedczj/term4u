use std::cell::RefCell;

use pathfinder_geometry::vector::Vector2F;
use warpui::elements::{ChildView, Container, Flex, ParentElement};
use warpui::keymap::Keystroke;
use warpui::{
    AppContext, Element, Entity, ModelHandle, TypedActionView, View, ViewContext, ViewHandle,
};

use crate::code::editor::comments::{EditorCommentsModel, PendingCommentEvent};
use crate::code::editor::line::EditorLineLocation;
use crate::code_review::comments::{CommentId, CommentOrigin};
use crate::editor::{EditorView, Event as EditorEvent};
use crate::view_components::action_button::{
    ActionButton, ButtonSize, DangerNakedTheme, KeystrokeSource, NakedTheme, PrimaryTheme,
};

pub(crate) const DEFAULT_COMMENT_MAX_WIDTH: f32 = 750.0;

#[derive(Debug)]
pub enum CommentEditorEvent {
    ContentChanged,
    CommentSaved {
        id: Option<CommentId>,
        comment_text: String,
        line: Option<EditorLineLocation>,
    },
    CloseEditor,
    DeleteComment { id: CommentId },
}

#[derive(Debug)]
pub enum CommentEditorAction {
    SaveComment,
    CloseEditor,
    RemoveComment,
}

pub struct CommentEditor {
    comment_id: Option<CommentId>,
    editor: ViewHandle<EditorView>,
    save_button: ViewHandle<ActionButton>,
    close_button: ViewHandle<ActionButton>,
    remove_button: ViewHandle<ActionButton>,
    line: Option<EditorLineLocation>,
    show_remove_button: bool,
    save_button_disabled: bool,
    laid_out_size: RefCell<Option<Vector2F>>,
}

impl CommentEditor {
    pub fn new(
        ctx: &mut ViewContext<Self>,
        comment_model: ModelHandle<EditorCommentsModel>,
    ) -> Self {
        Self::build(ctx, comment_model, None, None)
    }

    pub fn new_embedded(
        ctx: &mut ViewContext<Self>,
        comment_model: ModelHandle<EditorCommentsModel>,
        comment_id: Option<CommentId>,
        line: EditorLineLocation,
    ) -> Self {
        Self::build(ctx, comment_model, comment_id, Some(line))
    }

    fn build(
        ctx: &mut ViewContext<Self>,
        comment_model: ModelHandle<EditorCommentsModel>,
        comment_id: Option<CommentId>,
        line: Option<EditorLineLocation>,
    ) -> Self {
        let editor = create_editable_comment_markdown_editor(None, ctx);
        ctx.subscribe_to_view(&editor, |view, _, event, ctx| match event {
            EditorEvent::Edited(_) => {
                view.update_save_button_state(ctx);
                ctx.emit(CommentEditorEvent::ContentChanged);
            }
            EditorEvent::CmdEnter => view.save_comment(ctx),
            EditorEvent::Escape if view.comment_text(ctx).is_empty() => {
                view.reset(ctx);
                ctx.emit(CommentEditorEvent::CloseEditor);
            }
            EditorEvent::Activate
            | EditorEvent::Blurred
            | EditorEvent::Enter
            | EditorEvent::Escape
            | EditorEvent::Navigate(_)
            | EditorEvent::SelectionChanged
            | EditorEvent::UnhandledModifierKey(_)
            | EditorEvent::UnhandledCmdEnter => {}
        });
        ctx.subscribe_to_model(&comment_model, |view, _, event, ctx| match event {
            PendingCommentEvent::NewPendingComment(line) => {
                view.line = Some(line.clone());
                view.reset_editor(ctx);
            }
            PendingCommentEvent::ReopenPendingComment {
                id,
                line,
                comment_text,
                origin,
            } => view.reopen_saved_comment(id, Some(line.clone()), comment_text, origin, ctx),
        });
        let save_button = ctx.add_typed_action_view(|ctx| {
            ActionButton::new("Comment", PrimaryTheme)
                .with_keybinding(
                    KeystrokeSource::Fixed(
                        Keystroke::parse(crate::code_review::CODE_REVIEW_SUBMIT_KEYSTROKE)
                            .unwrap_or_default(),
                    ),
                    ctx,
                )
                .on_click(|ctx| ctx.dispatch_typed_action(CommentEditorAction::SaveComment))
                .with_size(ButtonSize::Small)
        });
        let close_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("Cancel", NakedTheme)
                .on_click(|ctx| ctx.dispatch_typed_action(CommentEditorAction::CloseEditor))
                .with_size(ButtonSize::Small)
        });
        let remove_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("Remove", DangerNakedTheme)
                .on_click(|ctx| ctx.dispatch_typed_action(CommentEditorAction::RemoveComment))
                .with_size(ButtonSize::Small)
        });
        let mut view = Self {
            comment_id,
            editor,
            save_button,
            close_button,
            remove_button,
            line,
            show_remove_button: comment_id.is_some(),
            save_button_disabled: true,
            laid_out_size: RefCell::new(None),
        };
        view.update_save_button_state(ctx);
        view
    }

    pub fn comment_text(&self, app: &AppContext) -> String {
        self.editor.as_ref(app).buffer_text(app)
    }

    pub fn get_laid_out_size(&self) -> Option<Vector2F> {
        *self.laid_out_size.borrow()
    }

    pub fn set_laid_out_size(&self, value: Vector2F) {
        self.laid_out_size.replace(Some(value));
    }

    fn update_save_button_state(&mut self, ctx: &mut ViewContext<Self>) {
        let empty = self.comment_text(ctx).trim().is_empty();
        if empty != self.save_button_disabled {
            self.save_button_disabled = empty;
            self.save_button
                .update(ctx, |button, ctx| button.set_disabled(empty, ctx));
        }
    }

    pub fn reopen_saved_comment(
        &mut self,
        id: &CommentId,
        line: Option<EditorLineLocation>,
        comment_text: &str,
        _origin: &CommentOrigin,
        ctx: &mut ViewContext<Self>,
    ) {
        self.editor.update(ctx, |editor, ctx| {
            editor.system_reset_buffer_text(comment_text, ctx)
        });
        self.comment_id = Some(*id);
        self.line = line;
        self.show_remove_button = true;
        self.save_button
            .update(ctx, |button, ctx| button.set_label("Update", ctx));
        self.update_save_button_state(ctx);
    }

    fn reset_editor(&mut self, ctx: &mut ViewContext<Self>) {
        self.editor
            .update(ctx, |editor, ctx| editor.system_reset_buffer_text("", ctx));
        self.update_save_button_state(ctx);
    }

    fn reset(&mut self, ctx: &mut ViewContext<Self>) {
        self.reset_editor(ctx);
        self.comment_id = None;
        self.line = None;
        self.show_remove_button = false;
        self.save_button
            .update(ctx, |button, ctx| button.set_label("Comment", ctx));
    }

    pub fn save_comment(&mut self, ctx: &mut ViewContext<Self>) {
        let comment_text = self.comment_text(ctx);
        if comment_text.trim().is_empty() {
            return;
        }
        ctx.emit(CommentEditorEvent::CommentSaved {
            id: self.comment_id,
            comment_text,
            line: self.line.clone(),
        });
        self.reset(ctx);
        ctx.emit(CommentEditorEvent::CloseEditor);
    }
}

impl Entity for CommentEditor {
    type Event = CommentEditorEvent;
}

impl View for CommentEditor {
    fn ui_name() -> &'static str {
        "CommentEditor"
    }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        let mut buttons = Flex::row()
            .child(ChildView::new(&self.close_button).finish());
        if self.show_remove_button {
            buttons = buttons.child(ChildView::new(&self.remove_button).finish());
        }
        buttons = buttons.child(ChildView::new(&self.save_button).finish());
        Container::new(
            Flex::column()
                .child(ChildView::new(&self.editor).finish())
                .child(buttons.finish())
                .finish(),
        )
        .with_uniform_padding(8.)
        .finish()
    }
}

impl TypedActionView for CommentEditor {
    type Action = CommentEditorAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            CommentEditorAction::SaveComment => self.save_comment(ctx),
            CommentEditorAction::CloseEditor => {
                self.reset(ctx);
                ctx.emit(CommentEditorEvent::CloseEditor);
            }
            CommentEditorAction::RemoveComment => {
                if let Some(id) = self.comment_id {
                    ctx.emit(CommentEditorEvent::DeleteComment { id });
                    self.reset(ctx);
                }
            }
        }
    }
}

pub(crate) fn create_editable_comment_markdown_editor<V>(
    initial_text: Option<String>,
    ctx: &mut ViewContext<V>,
) -> ViewHandle<EditorView>
where
    V: View,
{
    create_comment_editor(initial_text, ctx)
}

pub(crate) fn create_readonly_comment_markdown_editor<V>(
    initial_text: Option<String>,
    ctx: &mut ViewContext<V>,
) -> ViewHandle<EditorView>
where
    V: View,
{
    create_comment_editor(initial_text, ctx)
}

fn create_comment_editor<V>(
    initial_text: Option<String>,
    ctx: &mut ViewContext<V>,
) -> ViewHandle<EditorView>
where
    V: View,
{
    ctx.add_typed_action_view(|ctx| {
        EditorView::new_with_base_text(initial_text.unwrap_or_default(), Default::default(), ctx)
    })
}
