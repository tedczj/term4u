use settings::{Setting as _, ToggleableSetting as _};
use warp_errors::report_if_error;
use warpui::elements::Element;
use warpui::notification::RequestPermissionsOutcome;
use warpui::ui_components::components::UiComponent;
use warpui::ui_components::switch::SwitchStateHandle;
use warpui::{AppContext, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle};

use super::SettingsSection;
use super::settings_page::{
    LocalOnlyIconState, MatchData, PageTitle, PageType, SettingsPageMeta, SettingsPageViewHandle,
    SettingsWidget, ToggleState, render_body_item,
};
use crate::appearance::Appearance;
use crate::settings::{AppEditorSettings, InputSettings, SelectionSettings};
use crate::terminal::general_settings::GeneralSettings;
use crate::terminal::session_settings::{
    NotificationsMode, NotificationsSettings, SessionSettings,
};
use crate::terminal::settings::TerminalSettings;
use crate::view_components::DismissibleToast;
use crate::workspace::ToastStack;

#[derive(Clone, Debug, PartialEq)]
pub enum FeaturesPageAction {
    ToggleVimMode,
    ToggleRestoreSession,
    ToggleCopyOnSelect,
    ToggleConfirmCloseSession,
    ToggleAudibleBell,
    ToggleNotifications,
    ToggleAttentionNotifications,
    ToggleCompletionNotifications,
    ToggleNotificationSound,
    ToggleNativeShellCompletions,
}

pub struct FeaturesPageView {
    page: PageType<Self>,
}

impl FeaturesPageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        ctx.subscribe_to_model(&AppEditorSettings::handle(ctx), |_, _, _, ctx| ctx.notify());
        ctx.subscribe_to_model(&GeneralSettings::handle(ctx), |_, _, _, ctx| ctx.notify());
        ctx.subscribe_to_model(&SelectionSettings::handle(ctx), |_, _, _, ctx| ctx.notify());
        ctx.subscribe_to_model(&SessionSettings::handle(ctx), |_, _, _, ctx| ctx.notify());
        ctx.subscribe_to_model(&TerminalSettings::handle(ctx), |_, _, _, ctx| ctx.notify());
        ctx.subscribe_to_model(&InputSettings::handle(ctx), |_, _, _, ctx| ctx.notify());

        Self {
            page: PageType::new_uncategorized(
                vec![
                    Box::new(VimModeWidget::default()),
                    Box::new(RestoreSessionWidget::default()),
                    Box::new(CopyOnSelectWidget::default()),
                    Box::new(ConfirmCloseSessionWidget::default()),
                    Box::new(AudibleBellWidget::default()),
                    Box::new(NotificationsWidget::default()),
                    Box::new(AttentionNotificationsWidget::default()),
                    Box::new(CompletionNotificationsWidget::default()),
                    Box::new(NotificationSoundWidget::default()),
                    Box::new(NativeShellCompletionsWidget::default()),
                ],
                Some(PageTitle::new("Features")),
            ),
        }
    }
    fn update_notifications(
        ctx: &mut ViewContext<Self>,
        update: impl FnOnce(&mut NotificationsSettings),
    ) {
        SessionSettings::handle(ctx).update(ctx, |settings, ctx| {
            let mut value = settings.notifications.value().clone();
            update(&mut value);
            report_if_error!(settings.notifications.set_value(value, ctx));
        });
    }

    fn notification_permission_result(
        &mut self,
        outcome: RequestPermissionsOutcome,
        ctx: &mut ViewContext<Self>,
    ) {
        if SessionSettings::as_ref(ctx).notifications.mode != NotificationsMode::Enabled {
            return;
        }
        let message = match outcome {
            RequestPermissionsOutcome::Accepted => "Desktop notifications are enabled.",
            RequestPermissionsOutcome::PermissionsDenied => {
                "Allow Term4u notifications in macOS System Settings to receive desktop alerts."
            }
            RequestPermissionsOutcome::OtherError { .. } => {
                "Notification permissions could not be requested."
            }
        };
        let window = ctx.window_id();
        ToastStack::handle(ctx).update(ctx, |stack, ctx| {
            stack.add_ephemeral_toast(DismissibleToast::default(message.to_owned()), window, ctx);
        });
    }
}

impl Entity for FeaturesPageView {
    type Event = ();
}

impl TypedActionView for FeaturesPageView {
    type Action = FeaturesPageAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            FeaturesPageAction::ToggleVimMode => {
                AppEditorSettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(settings.vim_mode.toggle_and_save_value(ctx));
                });
            }
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
            FeaturesPageAction::ToggleNotifications => {
                let enable =
                    SessionSettings::as_ref(ctx).notifications.mode != NotificationsMode::Enabled;
                Self::update_notifications(ctx, |value| {
                    value.mode = if enable {
                        NotificationsMode::Enabled
                    } else {
                        NotificationsMode::Disabled
                    }
                });
                if enable {
                    ctx.request_desktop_notification_permissions(
                        Self::notification_permission_result,
                    );
                }
            }
            FeaturesPageAction::ToggleAttentionNotifications => {
                Self::update_notifications(ctx, |value| {
                    value.is_needs_attention_enabled = !value.is_needs_attention_enabled
                });
            }
            FeaturesPageAction::ToggleCompletionNotifications => {
                Self::update_notifications(ctx, |value| {
                    value.is_long_running_enabled = !value.is_long_running_enabled
                });
            }
            FeaturesPageAction::ToggleNotificationSound => {
                Self::update_notifications(ctx, |value| {
                    value.play_notification_sound = !value.play_notification_sound
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
struct VimModeWidget {
    state: SwitchStateHandle,
}

impl SettingsWidget for VimModeWidget {
    type View = FeaturesPageView;

    fn search_terms(&self) -> &str {
        "vim mode keybindings modal editing terminal input code"
    }

    fn render(
        &self,
        _: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        render_switch(
            "Vim keybindings",
            "Use Vim-style editing in terminal input and local editors.",
            AppEditorSettings::as_ref(app).vim_mode_enabled(),
            self.state.clone(),
            FeaturesPageAction::ToggleVimMode,
            appearance,
        )
    }
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

#[derive(Default)]
struct NotificationsWidget {
    state: SwitchStateHandle,
}
impl SettingsWidget for NotificationsWidget {
    type View = FeaturesPageView;
    fn search_terms(&self) -> &str {
        "desktop notifications enable allow"
    }
    fn render(
        &self,
        _: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        render_switch(
            "Desktop notifications",
            "Allow desktop notifications for background terminal activity.",
            SessionSettings::as_ref(app).notifications.mode == NotificationsMode::Enabled,
            self.state.clone(),
            FeaturesPageAction::ToggleNotifications,
            appearance,
        )
    }
}

#[derive(Default)]
struct AttentionNotificationsWidget {
    state: SwitchStateHandle,
}
impl SettingsWidget for AttentionNotificationsWidget {
    type View = FeaturesPageView;
    fn search_terms(&self) -> &str {
        "terminal notifications attention bell program"
    }
    fn render(
        &self,
        _: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        render_switch(
            "Terminal attention notifications",
            "Notify when a background terminal rings or requests attention.",
            SessionSettings::as_ref(app)
                .notifications
                .is_needs_attention_enabled,
            self.state.clone(),
            FeaturesPageAction::ToggleAttentionNotifications,
            appearance,
        )
    }
}

#[derive(Default)]
struct NotificationSoundWidget {
    state: SwitchStateHandle,
}
impl SettingsWidget for NotificationSoundWidget {
    type View = FeaturesPageView;
    fn search_terms(&self) -> &str {
        "desktop notifications sound"
    }
    fn render(
        &self,
        _: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        render_switch(
            "Notification sound",
            "Play sound for notifications requested by terminal programs. Bell sounds use the separate audible bell setting.",
            SessionSettings::as_ref(app)
                .notifications
                .play_notification_sound,
            self.state.clone(),
            FeaturesPageAction::ToggleNotificationSound,
            appearance,
        )
    }
}

#[derive(Default)]
struct CompletionNotificationsWidget {
    state: SwitchStateHandle,
}
impl SettingsWidget for CompletionNotificationsWidget {
    type View = FeaturesPageView;
    fn search_terms(&self) -> &str {
        "desktop notifications long running command completion duration threshold"
    }
    fn render(
        &self,
        _: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let settings = SessionSettings::as_ref(app).notifications.value();
        render_switch(
            "Long command notifications",
            &format!(
                "With desktop notifications enabled, notify when background commands finish after at least {} seconds.",
                settings.long_running_threshold.as_secs_f64()
            ),
            settings.is_long_running_enabled,
            self.state.clone(),
            FeaturesPageAction::ToggleCompletionNotifications,
            appearance,
        )
    }
}

#[cfg(test)]
#[path = "features_page_tests.rs"]
mod tests;
