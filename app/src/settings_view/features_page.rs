use settings::ToggleableSetting as _;
use warp_errors::report_if_error;
use warpui::elements::Element;
use warpui::ui_components::components::UiComponent;
use warpui::ui_components::switch::SwitchStateHandle;
use warpui::{AppContext, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle};

use super::SettingsSection;
use super::settings_page::{
    LocalOnlyIconState, MatchData, PageTitle, PageType, SettingsPageMeta, SettingsPageViewHandle,
    SettingsWidget, ToggleState, render_body_item,
};
use crate::appearance::Appearance;
use crate::settings::{InputSettings, SelectionSettings};
use crate::terminal::general_settings::GeneralSettings;
use crate::terminal::session_settings::SessionSettings;
use crate::terminal::settings::TerminalSettings;

#[derive(Clone, Debug, PartialEq)]
pub enum FeaturesPageAction {
    ToggleRestoreSession,
    ToggleCopyOnSelect,
    ToggleConfirmCloseSession,
    ToggleAudibleBell,
    ToggleNativeShellCompletions,
}

pub struct FeaturesPageView {
    page: PageType<Self>,
}

impl FeaturesPageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        ctx.subscribe_to_model(&GeneralSettings::handle(ctx), |_, _, _, ctx| ctx.notify());
        ctx.subscribe_to_model(&SelectionSettings::handle(ctx), |_, _, _, ctx| ctx.notify());
        ctx.subscribe_to_model(&SessionSettings::handle(ctx), |_, _, _, ctx| ctx.notify());
        ctx.subscribe_to_model(&TerminalSettings::handle(ctx), |_, _, _, ctx| ctx.notify());
        ctx.subscribe_to_model(&InputSettings::handle(ctx), |_, _, _, ctx| ctx.notify());

        Self {
            page: PageType::new_uncategorized(
                vec![
                    Box::new(RestoreSessionWidget::default()),
                    Box::new(CopyOnSelectWidget::default()),
                    Box::new(ConfirmCloseSessionWidget::default()),
                    Box::new(AudibleBellWidget::default()),
                    Box::new(NativeShellCompletionsWidget::default()),
                ],
                Some(PageTitle::new("Features")),
            ),
        }
    }
}

impl Entity for FeaturesPageView {
    type Event = ();
}

impl TypedActionView for FeaturesPageView {
    type Action = FeaturesPageAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            FeaturesPageAction::ToggleRestoreSession => {
                GeneralSettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(settings.restore_session.toggle_and_save_value(ctx));
                });
            }
            FeaturesPageAction::ToggleCopyOnSelect => {
                SelectionSettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(settings.copy_on_select.toggle_and_save_value(ctx));
                });
            }
            FeaturesPageAction::ToggleConfirmCloseSession => {
                SessionSettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(
                        settings
                            .should_confirm_close_session
                            .toggle_and_save_value(ctx)
                    );
                });
            }
            FeaturesPageAction::ToggleAudibleBell => {
                TerminalSettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(settings.use_audible_bell.toggle_and_save_value(ctx));
                });
            }
            FeaturesPageAction::ToggleNativeShellCompletions => {
                InputSettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(
                        settings
                            .native_shell_completions_enabled
                            .toggle_and_save_value(ctx)
                    );
                });
            }
        }
        ctx.notify();
    }
}

impl View for FeaturesPageView {
    fn ui_name() -> &'static str {
        "FeaturesPage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

impl SettingsPageMeta for FeaturesPageView {
    fn section() -> SettingsSection {
        SettingsSection::Features
    }

    fn should_render(&self, _: &AppContext) -> bool {
        true
    }

    fn on_page_selected(&mut self, _: bool, _: &mut ViewContext<Self>) {}

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

impl From<ViewHandle<FeaturesPageView>> for SettingsPageViewHandle {
    fn from(view: ViewHandle<FeaturesPageView>) -> Self {
        SettingsPageViewHandle::Features(view)
    }
}

fn render_switch(
    label: &str,
    description: &str,
    checked: bool,
    state: SwitchStateHandle,
    action: FeaturesPageAction,
    appearance: &Appearance,
) -> Box<dyn Element> {
    render_body_item::<FeaturesPageAction>(
        label.to_owned(),
        None,
        LocalOnlyIconState::Hidden,
        ToggleState::Enabled,
        appearance,
        appearance
            .ui_builder()
            .switch(state)
            .check(checked)
            .build()
            .on_click(move |ctx, _, _| ctx.dispatch_typed_action(action.clone()))
            .finish(),
        Some(description.to_owned()),
    )
}

#[derive(Default)]
struct RestoreSessionWidget {
    state: SwitchStateHandle,
}

impl SettingsWidget for RestoreSessionWidget {
    type View = FeaturesPageView;

    fn search_terms(&self) -> &str {
        "restore windows tabs panes previous session startup"
    }

    fn render(
        &self,
        _: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        render_switch(
            "Restore previous session",
            "Restore local windows, tabs, and panes when Term4u starts.",
            *GeneralSettings::as_ref(app).restore_session,
            self.state.clone(),
            FeaturesPageAction::ToggleRestoreSession,
            appearance,
        )
    }
}

#[derive(Default)]
struct CopyOnSelectWidget {
    state: SwitchStateHandle,
}

impl SettingsWidget for CopyOnSelectWidget {
    type View = FeaturesPageView;

    fn search_terms(&self) -> &str {
        "copy selection terminal clipboard"
    }

    fn render(
        &self,
        _: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        render_switch(
            "Copy on select",
            "Copy selected terminal text to the clipboard automatically.",
            *SelectionSettings::as_ref(app).copy_on_select,
            self.state.clone(),
            FeaturesPageAction::ToggleCopyOnSelect,
            appearance,
        )
    }
}

#[derive(Default)]
struct ConfirmCloseSessionWidget {
    state: SwitchStateHandle,
}

impl SettingsWidget for ConfirmCloseSessionWidget {
    type View = FeaturesPageView;

    fn search_terms(&self) -> &str {
        "confirm close running terminal session warning"
    }

    fn render(
        &self,
        _: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        render_switch(
            "Confirm before closing sessions",
            "Ask before closing a local terminal session that may still be running.",
            *SessionSettings::as_ref(app).should_confirm_close_session,
            self.state.clone(),
            FeaturesPageAction::ToggleConfirmCloseSession,
            appearance,
        )
    }
}

#[derive(Default)]
struct AudibleBellWidget {
    state: SwitchStateHandle,
}

impl SettingsWidget for AudibleBellWidget {
    type View = FeaturesPageView;

    fn search_terms(&self) -> &str {
        "terminal audible bell sound beep"
    }

    fn render(
        &self,
        _: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        render_switch(
            "Audible terminal bell",
            "Play a sound when a local terminal emits a bell.",
            *TerminalSettings::as_ref(app).use_audible_bell,
            self.state.clone(),
            FeaturesPageAction::ToggleAudibleBell,
            appearance,
        )
    }
}

#[derive(Default)]
struct NativeShellCompletionsWidget {
    state: SwitchStateHandle,
}

impl SettingsWidget for NativeShellCompletionsWidget {
    type View = FeaturesPageView;

    fn search_terms(&self) -> &str {
        "native shell completions command terminal"
    }

    fn render(
        &self,
        _: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        render_switch(
            "Native shell completions",
            "Use completions reported by the active local shell.",
            *InputSettings::as_ref(app).native_shell_completions_enabled,
            self.state.clone(),
            FeaturesPageAction::ToggleNativeShellCompletions,
            appearance,
        )
    }
}
