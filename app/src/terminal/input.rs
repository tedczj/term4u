use std::sync::Arc;

use warpui::elements::{ChildView, Container, ParentElement, SavePosition};
use warpui::{
    AppContext, Element, Entity, TypedActionView, View, ViewContext, ViewHandle,
    keymap,
};

use crate::editor::{EditorView, Event as EditorEvent};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandExecutionSource {
    User,
}

impl CommandExecutionSource {
    pub fn is_ai_command(&self) -> bool {
        false
    }

    pub fn should_preserve_input(&self) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuPositioning {
    AboveInputBox,
    BelowInputBox,
}

pub trait MenuPositioningProvider: Send + Sync {
    fn menu_position(&self, app: &AppContext) -> MenuPositioning;
}

#[derive(Clone, Debug)]
pub enum InputAction {
    Focus,
    Clear,
    Submit,
    Insert(String),
}

#[derive(Clone, Debug)]
pub enum Event {
    ExecuteCommand(String),
    CtrlC { cleared_buffer_len: usize },
    CtrlD,
    EditorFocused,
}

pub struct Input {
    editor: ViewHandle<EditorView>,
    save_position_id: String,
}

impl Input {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let editor = ctx.add_typed_action_view(|ctx| EditorView::new(Default::default(), ctx));
        ctx.subscribe_to_view(&editor, |input, _, event, ctx| match event {
            EditorEvent::Enter | EditorEvent::CmdEnter => input.submit(ctx),
            EditorEvent::Activate => ctx.emit(Event::EditorFocused),
            EditorEvent::Edited(_)
            | EditorEvent::Blurred
            | EditorEvent::Navigate(_)
            | EditorEvent::SelectionChanged
            | EditorEvent::Escape
            | EditorEvent::UnhandledModifierKey(_)
            | EditorEvent::UnhandledCmdEnter => {}
        });
        Self {
            editor,
            save_position_id: format!("terminal_input_{}", ctx.view_id()),
        }
    }

    pub fn init(_app: &mut AppContext) {}

    pub fn editor(&self) -> &ViewHandle<EditorView> {
        &self.editor
    }

    pub fn buffer_text(&self, app: &AppContext) -> String {
        self.editor.as_ref(app).buffer_text(app)
    }

    pub fn save_position_id(&self) -> String {
        self.save_position_id.clone()
    }

    pub fn editor_save_position_id(&self) -> String {
        self.save_position_id()
    }

    pub fn replace_buffer_content(&mut self, text: &str, ctx: &mut ViewContext<Self>) {
        self.editor
            .update(ctx, |editor, ctx| editor.set_buffer_text(text, ctx));
    }

    pub fn append_to_buffer(&mut self, text: &str, ctx: &mut ViewContext<Self>) {
        self.editor.update(ctx, |editor, ctx| {
            let mut content = editor.buffer_text(ctx);
            content.push_str(text);
            editor.set_buffer_text(&content, ctx);
        });
    }

    pub fn clear_buffer_and_reset_undo_stack(&mut self, ctx: &mut ViewContext<Self>) {
        self.editor.update(ctx, |editor, ctx| editor.clear_buffer(ctx));
    }

    pub fn set_pending_command(&mut self, command: &str, ctx: &mut ViewContext<Self>) {
        self.replace_buffer_content(command, ctx);
        self.submit(ctx);
    }

    pub fn send_input_buffer_to_terminal_editor(
        &mut self,
        contents: Arc<String>,
        ctx: &mut ViewContext<Self>,
    ) {
        self.replace_buffer_content(&contents, ctx);
    }

    pub fn run_command_in_synced_terminal_input(&mut self, ctx: &mut ViewContext<Self>) {
        self.submit(ctx);
    }

    pub fn focus_input_box(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.focus(&self.editor);
    }

    fn submit(&mut self, ctx: &mut ViewContext<Self>) {
        let command = self.buffer_text(ctx);
        if command.trim().is_empty() {
            return;
        }
        self.clear_buffer_and_reset_undo_stack(ctx);
        ctx.emit(Event::ExecuteCommand(command));
    }
}

impl Entity for Input {
    type Event = Event;
}

impl View for Input {
    fn ui_name() -> &'static str {
        "TerminalInput"
    }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        SavePosition::new(
            Container::new(ChildView::new(&self.editor).finish())
                .with_uniform_padding(8.)
                .finish(),
            &self.save_position_id,
        )
        .finish()
    }

    fn keymap_context(&self, _app: &AppContext) -> keymap::Context {
        let mut context = keymap::Context::default();
        context.set.insert(Self::ui_name());
        context
    }
}

impl TypedActionView for Input {
    type Action = InputAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            InputAction::Focus => self.focus_input_box(ctx),
            InputAction::Clear => self.clear_buffer_and_reset_undo_stack(ctx),
            InputAction::Submit => self.submit(ctx),
            InputAction::Insert(text) => self.append_to_buffer(text, ctx),
        }
    }
}
