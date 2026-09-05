use std::path::PathBuf;

use warpui::elements::{Container, Element, ParentElement, Text};
use warpui::{AppContext, Entity, TypedActionView, View, ViewContext, ViewHandle};

use super::SettingsSection;
use super::settings_page::{
    MatchData, PageType, SettingsPageEvent, SettingsPageMeta, SettingsPageViewHandle, SettingsWidget,
};
use crate::appearance::Appearance;

#[derive(Clone, Debug)]
pub enum CodeIndexingPageAction {
    NoOp,
}

#[derive(Clone, Debug)]
pub enum CodeIndexingPageEvent {
    OpenLspLogs { log_path: PathBuf },
    OpenProjectRules { rule_paths: Vec<PathBuf> },
}

pub struct CodeIndexingPageView {
    page: PageType<Self>,
}

impl CodeIndexingPageView {
    pub fn new(_ctx: &mut ViewContext<Self>) -> Self {
        Self {
            page: PageType::new_monolith(LocalLanguageServersWidget, None, false),
        }
    }
}

struct LocalLanguageServersWidget;

impl SettingsWidget for LocalLanguageServersWidget {
    type View = CodeIndexingPageView;

    fn search_terms(&self) -> &str {
        "language server lsp local path"
    }

    fn render(
        &self,
        _view: &CodeIndexingPageView,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        Container::new(
            Text::new(
                "Language servers are detected and launched from PATH. Missing servers must be installed manually.",
                appearance.ui_font_family(),
                appearance.ui_font_size(),
            )
            .with_color(appearance.theme().main_text_color(appearance.theme().background()).into_solid())
            .finish(),
        )
        .with_uniform_padding(16.)
        .finish()
    }
}

impl Entity for CodeIndexingPageView {
    type Event = CodeIndexingPageEvent;
}

impl TypedActionView for CodeIndexingPageView {
    type Action = CodeIndexingPageAction;

    fn handle_action(&mut self, action: &Self::Action, _ctx: &mut ViewContext<Self>) {
        match action {
            CodeIndexingPageAction::NoOp => {}
        }
    }
}

impl View for CodeIndexingPageView {
    fn ui_name() -> &'static str {
        "CodeIndexingPage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

impl SettingsPageMeta for CodeIndexingPageView {
    fn section() -> SettingsSection {
        SettingsSection::CodeIndexing
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        true
    }

    fn update_filter(&mut self, query: &str, ctx: &mut ViewContext<Self>) -> MatchData {
        self.page.update_filter(query, ctx)
    }

    fn scroll_to_widget(&mut self, widget_id: &'static str) {
        self.page.scroll_to_widget(widget_id)
    }

    fn clear_highlighted_widget(&mut self) {
        self.page.clear_highlighted_widget();
    }
}

impl From<ViewHandle<CodeIndexingPageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<CodeIndexingPageView>) -> Self {
        SettingsPageViewHandle::CodeIndexing(view_handle)
    }
}

pub fn init_actions_from_parent_view<T: warpui::Action + Clone>(
    _app: &mut AppContext,
    _context: &warpui::keymap::ContextPredicate,
    _builder: fn(super::SettingsAction) -> T,
) {
}
