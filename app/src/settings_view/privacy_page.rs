use settings::Setting as _;
use warp_errors::report_if_error;
use warpui::elements::{ChildView, Container, Element, Flex, ParentElement, Text};
use warpui::{AppContext, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle};

use super::SettingsSection;
use super::settings_page::{
    LocalOnlyIconState, MatchData, PageTitle, PageType, SettingsPageEvent, SettingsPageMeta,
    SettingsPageViewHandle, SettingsWidget, ToggleState, render_body_item,
};
use crate::appearance::Appearance;
use crate::settings::local_privacy_policy::LocalPrivacyPolicy;
use crate::terminal::settings::{Osc52ClipboardAccess, TerminalSettings};
use crate::view_components::{Dropdown, DropdownItem};

pub struct PrivacyPageView {
    page: PageType<Self>,
    clipboard_access: ViewHandle<Dropdown<PrivacyPageAction>>,
}

impl PrivacyPageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let clipboard_access = ctx.add_typed_action_view(|ctx| {
            let mut dropdown = Dropdown::new(ctx);
            dropdown.set_top_bar_max_width(240.);
            dropdown.set_items(
                [
                    Osc52ClipboardAccess::Deny,
                    Osc52ClipboardAccess::WriteOnly,
                    Osc52ClipboardAccess::ReadWrite,
                ]
                .into_iter()
                .map(|access| {
                    DropdownItem::new(
                        access.as_dropdown_label(),
                        PrivacyPageAction::SetClipboardAccess(access),
                    )
                })
                .collect(),
                ctx,
            );
            dropdown.set_selected_by_action(
                PrivacyPageAction::SetClipboardAccess(
                    *TerminalSettings::as_ref(ctx).osc52_clipboard_access,
                ),
                ctx,
            );
            dropdown
        });
        ctx.subscribe_to_model(&TerminalSettings::handle(ctx), |view, _, _, ctx| {
            let access = *TerminalSettings::as_ref(ctx).osc52_clipboard_access;
            view.clipboard_access.update(ctx, |dropdown, ctx| {
                dropdown.set_selected_by_action(PrivacyPageAction::SetClipboardAccess(access), ctx);
            });
            ctx.notify();
        });
        Self {
            clipboard_access,
            page: PageType::new_uncategorized(
                vec![
                    Box::new(LocalPrivacyWidget),
                    Box::new(ClipboardAccessWidget),
                ],
                Some(PageTitle::new("Privacy")),
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PrivacyPageAction {
    SetClipboardAccess(Osc52ClipboardAccess),
}

impl Entity for PrivacyPageView {
    type Event = SettingsPageEvent;
}

impl TypedActionView for PrivacyPageView {
    type Action = PrivacyPageAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            PrivacyPageAction::SetClipboardAccess(access) => {
                TerminalSettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(settings.osc52_clipboard_access.set_value(*access, ctx));
                });
            }
        }
        ctx.notify();
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
            (
                "Crash reporting",
                LocalPrivacyPolicy::CRASH_REPORTING_ENABLED,
            ),
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
                .with_color(
                    appearance
                        .theme()
                        .main_text_color(appearance.theme().background())
                        .into_solid(),
                )
                .finish(),
            );
        }
        Container::new(column.finish())
            .with_uniform_padding(16.)
            .finish()
    }
}

struct ClipboardAccessWidget;

impl SettingsWidget for ClipboardAccessWidget {
    type View = PrivacyPageView;

    fn search_terms(&self) -> &str {
        "privacy terminal clipboard OSC52 OSC 52 permissions access deny read write only"
    }

    fn render(
        &self,
        view: &Self::View,
        appearance: &Appearance,
        _: &AppContext,
    ) -> Box<dyn Element> {
        render_body_item::<PrivacyPageAction>(
            "Terminal clipboard access (OSC 52)".into(),
            None,
            LocalOnlyIconState::Hidden,
            ToggleState::Enabled,
            appearance,
            ChildView::new(&view.clipboard_access).finish(),
            Some("Deny blocks terminal programs from accessing your clipboard. Write only lets them replace its contents. Read and write also lets them read its contents. Applies to GUI terminals.".into()),
        )
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

#[cfg(test)]
#[path = "privacy_page_tests.rs"]
mod tests;
