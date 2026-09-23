use warpui::App;
use warpui::platform::WindowStyle;

use super::*;
use crate::settings_view::settings_page::FilteredPageType;
use crate::test_util::terminal::initialize_app_for_terminal_view;

#[test]
fn l0_09_notifications_controls_filter_and_preserve_other_preferences() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, page) = app.add_window(WindowStyle::NotStealFocus, FeaturesPageView::new);
        page.update(&mut app, |view, ctx| {
            view.update_filter("notification sound", ctx);
            let FilteredPageType::Uncategorized { widgets, title, .. } = view.page.get_filtered()
            else {
                panic!("expected searchable widgets")
            };
            assert!(title.is_some());
            assert_eq!(widgets.len(), 1);
            let before = SessionSettings::as_ref(ctx).notifications.value().clone();
            view.handle_action(&FeaturesPageAction::ToggleNotifications, ctx);
            view.handle_action(&FeaturesPageAction::ToggleNotificationSound, ctx);
            view.handle_action(&FeaturesPageAction::ToggleAttentionNotifications, ctx);
            let value = SessionSettings::as_ref(ctx).notifications.value();
            assert_eq!(value.mode, NotificationsMode::Enabled);
            assert_eq!(
                value.play_notification_sound,
                !before.play_notification_sound
            );
            assert_eq!(
                value.is_needs_attention_enabled,
                !before.is_needs_attention_enabled
            );
            assert_eq!(value.long_running_threshold, before.long_running_threshold);
            assert_eq!(
                value.is_long_running_enabled,
                before.is_long_running_enabled
            );
            assert_eq!(
                value.is_password_prompt_enabled,
                before.is_password_prompt_enabled
            );
            view.update_filter("long running", ctx);
            let FilteredPageType::Uncategorized { widgets, .. } = view.page.get_filtered() else {
                panic!("expected searchable widgets")
            };
            assert_eq!(widgets.len(), 1);
            view.handle_action(&FeaturesPageAction::ToggleCompletionNotifications, ctx);
            assert_eq!(
                SessionSettings::as_ref(ctx)
                    .notifications
                    .is_long_running_enabled,
                !before.is_long_running_enabled
            );
            view.handle_action(&FeaturesPageAction::ToggleNotifications, ctx);
            assert_eq!(
                SessionSettings::as_ref(ctx).notifications.mode,
                NotificationsMode::Disabled
            );
        });
    });
}
