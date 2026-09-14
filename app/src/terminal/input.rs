use std::sync::Arc;

use warp_completer::completer::{
    CompleterOptions, ExplicitTabCompletion, MatchStrategy, PreparedSuggestion, SuggestionResults,
    suggestions,
};
use warp_completer::meta::Span;
use warp_editor::editor::NavigationKey;
use warp_util::user_input::UserInput;
use warpui::elements::{
    ChildView, Container, CrossAxisAlignment, Flex, ParentElement, SavePosition,
};
use warpui::keymap::FixedBinding;
use warpui::ui_components::components::{UiComponent, UiComponentStyles};
use warpui::{
    AppContext, Element, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
    keymap,
};

use crate::appearance::Appearance;
use crate::completer::SessionContext;
use crate::editor::{
    EditorAction, EditorOptions, EditorView, Event as EditorEvent, PlainTextEditorViewAction,
    PropagateAndNoOpNavigationKeys,
};
use crate::terminal::model::session::SessionId;

struct HistoryNavigation {
    session_id: SessionId,
    commands: Vec<String>,
    draft: String,
    draft_cursor: usize,
    index: usize,
    displayed: String,
}

struct CompletionMenu {
    suggestions: Vec<PreparedSuggestion>,
    span: Span,
    buffer: String,
    cursor: usize,
    next: usize,
}

pub const OPEN_COMPLETIONS_KEYBINDING_NAME: &str = "input:open_completion_suggestions";

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MenuPositioning {
    AboveInputBox,
    #[default]
    BelowInputBox,
}

pub trait MenuPositioningProvider: Send + Sync {
    fn menu_position(&self, app: &AppContext) -> MenuPositioning;
}

impl MenuPositioningProvider for MenuPositioning {
    fn menu_position(&self, _: &AppContext) -> MenuPositioning {
        *self
    }
}

#[derive(Clone, Debug)]
pub enum InputAction {
    Focus,
    Clear,
    Submit,
    CtrlD,
    Complete,
    Insert(String),
}

#[derive(Clone, Debug)]
pub enum Event {
    ExecuteCommand(String),
    CtrlC { cleared_buffer_len: usize },
    CtrlD,
    EditorFocused,
    Complete,
    NavigateHistory { previous: bool },
}

pub struct Input {
    editor: ViewHandle<EditorView>,
    save_position_id: String,
    completions: Option<CompletionMenu>,
    completion_request: usize,
    history_navigation: Option<HistoryNavigation>,
}

impl Input {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let editor = ctx.add_typed_action_view(|ctx| {
            EditorView::new(
                EditorOptions {
                    propagate_and_no_op_vertical_navigation_keys:
                        PropagateAndNoOpNavigationKeys::AtBoundary,
                    autogrow: true,
                    soft_wrap: true,
                    use_settings_line_height_ratio: true,
                    supports_vim_mode: true,
                    allow_user_cursor_preference: true,
                    keymap_context_modifier: Some(Box::new(|context, _| {
                        context.set.insert("TerminalCommandEditor");
                        context.set.insert(
                            crate::settings_view::flags::TERMINAL_INPUT_PAGE_KEYS_HANDLED_BY_INPUT,
                        );
                    })),
                    ..Default::default()
                },
                ctx,
            )
        });
        ctx.subscribe_to_view(&editor, |input, _, event, ctx| match event {
            EditorEvent::Enter | EditorEvent::CmdEnter => input.submit(ctx),
            EditorEvent::CtrlC { cleared_buffer_len } => ctx.emit(Event::CtrlC {
                cleared_buffer_len: *cleared_buffer_len,
            }),
            EditorEvent::Navigate(NavigationKey::Up) => {
                ctx.emit(Event::NavigateHistory { previous: true });
            }
            EditorEvent::Navigate(NavigationKey::Down) => {
                ctx.emit(Event::NavigateHistory { previous: false });
            }
            EditorEvent::Edited(_) | EditorEvent::SelectionChanged => {
                if input.history_navigation.as_ref().is_some_and(|navigation| {
                    navigation.displayed != input.buffer_text(ctx)
                }) {
                    input.history_navigation = None;
                }
                if input.completions.as_ref().is_some_and(|menu| {
                    menu.buffer != input.buffer_text(ctx) || menu.cursor != input.cursor(ctx)
                }) {
                    input.completions = None;
                }
                input.completion_request += 1;
                ctx.notify();
            }
            EditorEvent::Escape => {
                input.history_navigation = None;
                input.completions = None;
                input.completion_request += 1;
                ctx.notify();
            }
            EditorEvent::Activate => ctx.emit(Event::EditorFocused),
            _ => {}
        });
        Self {
            editor,
            completions: None,
            completion_request: 0,
            history_navigation: None,
            save_position_id: format!("terminal_input_{}", ctx.view_id()),
        }
    }

    pub fn init(app: &mut AppContext) {
        use warpui::keymap::macros::*;

        app.register_fixed_bindings([
            FixedBinding::new(
                "tab",
                InputAction::Complete,
                id!("TerminalCommandEditor") & !id!("IMEOpen"),
            ),
            FixedBinding::new(
                "ctrl-d",
                InputAction::CtrlD,
                id!("TerminalInput") & id!("InputEmpty"),
            ),
        ]);
    }

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
        self.history_navigation = None;
        self.editor
            .update(ctx, |editor, ctx| editor.set_buffer_text(text, ctx));
    }

    /// Insert at the editor selections, preserving suffix text and the editor's undo history.
    pub fn insert_text(&mut self, text: &str, ctx: &mut ViewContext<Self>) {
        self.history_navigation = None;
        self.completions = None;
        self.completion_request += 1;
        self.editor.update(ctx, |editor, ctx| {
            editor.handle_action(&EditorAction::UserInsert(UserInput::new(text)), ctx);
        });
    }

    /// Navigate an owned snapshot of the active session's local history. Down past the newest
    /// match restores the draft and its cursor; edits or a session switch start a new traversal.
    pub fn navigate_history(
        &mut self,
        session_id: SessionId,
        commands: Vec<String>,
        previous: bool,
        ctx: &mut ViewContext<Self>,
    ) {
        if !self.editor.as_ref(ctx).is_single_cursor_only(ctx) {
            return;
        }
        let buffer = self.buffer_text(ctx);
        if self.history_navigation.as_ref().is_some_and(|navigation| {
            navigation.displayed != buffer || navigation.session_id != session_id
        }) {
            self.history_navigation = None;
        }
        if self.history_navigation.is_none() {
            if !previous {
                return;
            }
            let commands: Vec<_> = commands
                .into_iter()
                .filter(|command| !command.trim().is_empty() && command.starts_with(&buffer))
                .collect();
            if commands.is_empty() {
                return;
            }
            self.history_navigation = Some(HistoryNavigation {
                session_id,
                index: commands.len(),
                commands,
                draft_cursor: self.cursor(ctx),
                draft: buffer.clone(),
                displayed: buffer,
            });
        }
        let navigation = self.history_navigation.as_mut().unwrap();
        navigation.index = if previous {
            navigation.index.saturating_sub(1)
        } else {
            (navigation.index + 1).min(navigation.commands.len())
        };
        let at_draft = navigation.index == navigation.commands.len();
        let (text, cursor) = if at_draft {
            (navigation.draft.clone(), navigation.draft_cursor)
        } else {
            let command = navigation.commands[navigation.index].clone();
            let cursor = command.len();
            (command, cursor)
        };
        navigation.displayed = text.clone();
        self.completions = None;
        self.completion_request += 1;
        self.editor.update(ctx, |editor, ctx| {
            editor.set_buffer_text(&text, ctx);
            editor.select_ranges_by_byte_offset([cursor.into()..cursor.into()], ctx);
        });
        if at_draft {
            self.history_navigation = None;
        }
        ctx.notify();
    }

    pub fn append_to_buffer(&mut self, text: &str, ctx: &mut ViewContext<Self>) {
        self.editor.update(ctx, |editor, ctx| {
            let mut content = editor.buffer_text(ctx);
            content.push_str(text);
            editor.set_buffer_text(&content, ctx);
        });
    }

    pub fn clear_buffer_and_reset_undo_stack(&mut self, ctx: &mut ViewContext<Self>) {
        self.history_navigation = None;
        self.editor
            .update(ctx, |editor, ctx| editor.clear_buffer(ctx));
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
        self.editor.update(ctx, |editor, ctx| {
            editor.handle_action(&EditorAction::Focus, ctx)
        });
    }

    fn cursor(&self, app: &AppContext) -> usize {
        self.editor
            .as_ref(app)
            .end_byte_index_of_last_selection(app)
            .as_usize()
    }

    fn replace_completion(&mut self, span: Span, replacement: &str, ctx: &mut ViewContext<Self>) {
        self.editor.update(ctx, |editor, ctx| {
            editor.select_and_replace(
                replacement,
                [span.start().into()..span.end().into()],
                PlainTextEditorViewAction::Tab,
                ctx,
            );
        });
    }

    fn tab(&mut self, ctx: &mut ViewContext<Self>) {
        if let Some(mut menu) = self.completions.take()
            && menu.buffer == self.buffer_text(ctx)
            && menu.cursor == self.cursor(ctx)
        {
            let replacement = menu.suggestions[menu.next]
                .suggestion
                .replacement
                .to_string();
            self.replace_completion(menu.span, &replacement, ctx);
            menu.span = Span::new(menu.span.start(), menu.span.start() + replacement.len());
            menu.next = (menu.next + 1) % menu.suggestions.len();
            menu.buffer = self.buffer_text(ctx);
            menu.cursor = self.cursor(ctx);
            self.completions = Some(menu);
            ctx.notify();
            return;
        }
        ctx.emit(Event::Complete);
    }

    pub fn complete(&mut self, session: SessionContext, ctx: &mut ViewContext<Self>) {
        if !self.editor.as_ref(ctx).is_single_cursor_only(ctx) {
            return;
        }
        self.completion_request += 1;
        let request = self.completion_request;
        let buffer = self.buffer_text(ctx);
        let cursor = self.cursor(ctx);
        ctx.spawn(
            async move {
                let result = suggestions(
                    &buffer,
                    cursor,
                    None,
                    CompleterOptions {
                        match_strategy: MatchStrategy::CaseInsensitive,
                        ..Default::default()
                    },
                    &session,
                )
                .await;
                (buffer, cursor, result)
            },
            move |input, (buffer, cursor, result), ctx| {
                input.finish_completion(request, buffer, cursor, result, ctx);
            },
        );
    }

    fn finish_completion(
        &mut self,
        request: usize,
        buffer: String,
        cursor: usize,
        result: Option<SuggestionResults>,
        ctx: &mut ViewContext<Self>,
    ) {
        if request != self.completion_request
            || buffer != self.buffer_text(ctx)
            || cursor != self.cursor(ctx)
        {
            return;
        }
        let Some(result) = result else {
            return;
        };
        let span = result.replacement_span;
        let Some(query) = buffer.get(span.start()..span.end()) else {
            return;
        };
        let (suggestions, span) = match result.explicit_tab_completion(query, &['/']) {
            ExplicitTabCompletion::NoAction => return,
            ExplicitTabCompletion::InsertSingle {
                suggestion,
                replacement_span,
            } => {
                self.replace_completion(replacement_span, &suggestion.suggestion.replacement, ctx);
                return;
            }
            ExplicitTabCompletion::InsertCommonPrefixAndOpen {
                common_prefix,
                suggestions,
                replacement_span,
            } => {
                self.replace_completion(replacement_span, &common_prefix, ctx);
                (
                    suggestions,
                    Span::new(
                        replacement_span.start(),
                        replacement_span.start() + common_prefix.len(),
                    ),
                )
            }
            ExplicitTabCompletion::Open {
                suggestions,
                replacement_span,
            } => (suggestions, replacement_span),
        };
        self.completions = Some(CompletionMenu {
            suggestions,
            span,
            buffer: self.buffer_text(ctx),
            cursor: self.cursor(ctx),
            next: 0,
        });
        ctx.notify();
    }

    fn submit(&mut self, ctx: &mut ViewContext<Self>) {
        self.completions = None;
        self.completion_request += 1;
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

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let mut column = Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);
        if let Some(menu) = &self.completions {
            let labels = menu
                .suggestions
                .iter()
                .map(|item| item.suggestion.replacement.as_str())
                .collect::<Vec<_>>()
                .join("  ");
            column.add_child(
                Appearance::as_ref(app)
                    .ui_builder()
                    .paragraph(labels)
                    .with_style(UiComponentStyles {
                        font_family_id: Some(Appearance::as_ref(app).monospace_font_family()),
                        font_size: Some(Appearance::as_ref(app).monospace_font_size()),
                        ..Default::default()
                    })
                    .build()
                    .finish(),
            );
        }
        column.add_child(
            SavePosition::new(
                Container::new(ChildView::new(&self.editor).finish())
                    .with_uniform_padding(8.)
                    .finish(),
                &self.save_position_id,
            )
            .finish(),
        );
        column.finish()
    }

    fn keymap_context(&self, app: &AppContext) -> keymap::Context {
        let mut context = keymap::Context::default();
        context.set.insert(Self::ui_name());
        if self.buffer_text(app).is_empty() {
            context.set.insert("InputEmpty");
        }
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
            InputAction::Complete => self.tab(ctx),
            InputAction::CtrlD => ctx.emit(Event::CtrlD),
            InputAction::Insert(text) => self.insert_text(text, ctx),
        }
    }
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "input_local_tests.rs"]
mod local_tests;
