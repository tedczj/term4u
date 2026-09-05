use warpui::elements::{Container, Element, Flex, ParentElement, Text};
use warpui::{
    AppContext, Entity, TypedActionView, View, ViewContext, ViewHandle,
};

use super::SettingsSection;
use super::settings_page::{
    MatchData, PageType, SettingsPageEvent, SettingsPageMeta, SettingsPageViewHandle, SettingsWidget,
};
use crate::appearance::Appearance;
use crate::settings::local_privacy_policy::LocalPrivacyPolicy;

pub struct PrivacyPageView {
    page: PageType<Self>,
}

impl PrivacyPageView {
    pub fn new(_ctx: &mut ViewContext<Self>) -> Self {
        Self {
            page: PageType::new_monolith(LocalPrivacyWidget, None, false),
        }
    }
}

#[derive(Clone, Copy)]
pub enum PrivacyPageViewEvent {
    LaunchNetworkLogging,
    ShowAddRegexModal,
    HideAddRegexModal,
}

#[derive(Clone, Debug)]
pub enum PrivacyPageAction {
    NoOp,
}

impl Entity for PrivacyPageView {
    type Event = SettingsPageEvent;
}

impl TypedActionView for PrivacyPageView {
    type Action = PrivacyPageAction;

    fn handle_action(&mut self, action: &Self::Action, _ctx: &mut ViewContext<Self>) {
        match action {
            PrivacyPageAction::NoOp => {}
        }
    }
}

impl View for PrivacyPageView {
    fn ui_name() -> &'static str {
        "PrivacyPage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

struct LocalPrivacyWidget;

impl SettingsWidget for LocalPrivacyWidget {
    type View = PrivacyPageView;

    fn search_terms(&self) -> &str {
        "privacy local offline telemetry crash reporting cloud storage"
    }

    fn render(
        &self,
        _view: &PrivacyPageView,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        let values = [
            ("Telemetry", LocalPrivacyPolicy::TELEMETRY_ENABLED),
            ("Crash reporting", LocalPrivacyPolicy::CRASH_REPORTING_ENABLED),
            ("Cloud storage", LocalPrivacyPolicy::CLOUD_STORAGE_ENABLED),
        ];
        let mut column = Flex::column().with_spacing(12.);
        for (label, enabled) in values {
            column.add_child(
                Text::new(
                    format!("{label}: {}", if enabled { "Enabled" } else { "Disabled" }),
                    appearance.ui_font_family(),
                    appearance.ui_font_size(),
                )
                .with_color(appearance.theme().main_text_color(appearance.theme().background()).into_solid())
                .finish(),
            );
        }
        Container::new(column.finish()).with_uniform_padding(16.).finish()
    }
}

impl SettingsPageMeta for PrivacyPageView {
    fn section() -> SettingsSection {
        SettingsSection::Privacy
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

impl From<ViewHandle<PrivacyPageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<PrivacyPageView>) -> Self {
        SettingsPageViewHandle::Privacy(view_handle)
    }
}
