use warpui::App;
use warpui::platform::WindowStyle;

use super::*;
use crate::settings_view::settings_page::FilteredPageType;
use crate::test_util::terminal::initialize_app_for_terminal_view;

#[test]
fn l0_09_clipboard_permission_is_independently_searchable() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, page) = app.add_window(WindowStyle::NotStealFocus, PrivacyPageView::new);
        page.update(&mut app, |page, ctx| {
            page.update_filter("clipboard", ctx);
            let FilteredPageType::Uncategorized { widgets, title, .. } = page.page.get_filtered()
            else {
                panic!("privacy must have searchable widgets");
            };
            assert!(title.is_some());
            assert_eq!(widgets.len(), 1);
            assert!(widgets[0].search_terms().contains("OSC52"));

            page.update_filter("write only", ctx);
            let FilteredPageType::Uncategorized { widgets, .. } = page.page.get_filtered() else {
                unreachable!()
            };
            assert_eq!(widgets.len(), 1);

            page.update_filter("telemetry", ctx);
            let FilteredPageType::Uncategorized { widgets, .. } = page.page.get_filtered() else {
                unreachable!()
            };
            assert_eq!(widgets.len(), 1);
            assert!(!widgets[0].search_terms().contains("OSC52"));

            page.update_filter("", ctx);
            let FilteredPageType::Uncategorized { widgets, .. } = page.page.get_filtered() else {
                unreachable!()
            };
            assert_eq!(widgets.len(), 2);
        });
    });
}

#[test]
fn l0_07_permission_actions_keep_read_and_write_separate() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, page) = app.add_window(WindowStyle::NotStealFocus, PrivacyPageView::new);
        app.read(|ctx| {
            assert_eq!(
                *TerminalSettings::as_ref(ctx).osc52_clipboard_access,
                Osc52ClipboardAccess::Deny
            )
        });
        page.update(&mut app, |page, ctx| {
            page.handle_action(
                &PrivacyPageAction::SetClipboardAccess(Osc52ClipboardAccess::WriteOnly),
                ctx,
            )
        });
        app.read(|ctx| {
            let access = *TerminalSettings::as_ref(ctx).osc52_clipboard_access;
            assert!(access.allows_write());
            assert!(!access.allows_read());
        });
        page.update(&mut app, |page, ctx| {
            page.handle_action(
                &PrivacyPageAction::SetClipboardAccess(Osc52ClipboardAccess::ReadWrite),
                ctx,
            )
        });
        app.read(|ctx| {
            assert!(
                TerminalSettings::as_ref(ctx)
                    .osc52_clipboard_access
                    .allows_read()
            )
        });
        page.update(&mut app, |page, ctx| {
            page.handle_action(
                &PrivacyPageAction::SetClipboardAccess(Osc52ClipboardAccess::Deny),
                ctx,
            )
        });
        app.read(|ctx| {
            assert!(
                !TerminalSettings::as_ref(ctx)
                    .osc52_clipboard_access
                    .allows_write()
            )
        });
    });
}

async fn parse_clipboard_packet(
    app: &mut App,
    terminal: &ViewHandle<crate::terminal::TerminalView>,
    packet: &[u8],
    marker: &'static str,
) {
    let configuration = terminal.read(app, |view, _| view.pane_configuration().clone());
    let (tx, rx) = async_channel::bounded(1);
    app.update(|ctx| {
        ctx.subscribe_to_model(&configuration, move |configuration, _, ctx| {
            if configuration.as_ref(ctx).title() == marker {
                let _ = tx.try_send(());
            }
        });
    });
    terminal.update(app, |view, _| {
        let mut model = view.model.lock();
        model.process_bytes(packet);
        model.process_bytes(format!("\x1b]2;{marker}\x07").as_str());
    });
    rx.recv().await.unwrap();
}

#[test]
fn l0_07_privacy_setting_action_controls_the_actual_osc_consumer() {
    use warpui::clipboard::ClipboardContent;

    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) =
            crate::test_util::terminal::add_window_with_id_and_terminal(&mut app, None);
        let (_, page) = app.add_window(WindowStyle::NotStealFocus, PrivacyPageView::new);
        let (tx, rx) = async_channel::unbounded();
        app.update(|ctx| {
            ctx.clipboard()
                .write(ClipboardContent::plain_text("original".into()));
            ctx.subscribe_to_view(&terminal, move |_, event, _| {
                if let crate::terminal::view::Event::ClipboardResponse(response) = event {
                    let bytes = &response.bytes;
                    tx.try_send(bytes.to_vec()).unwrap();
                }
            });
        });
        page.update(&mut app, |page, ctx| {
            page.handle_action(
                &PrivacyPageAction::SetClipboardAccess(Osc52ClipboardAccess::WriteOnly),
                ctx,
            )
        });
        parse_clipboard_packet(
            &mut app,
            &terminal,
            b"\x1b]52;c;bmV3\x07\x1b]52;c;?\x07",
            "write-only",
        )
        .await;
        app.update(|ctx| assert_eq!(ctx.clipboard().read().plain_text, "new"));
        assert!(rx.is_empty());

        page.update(&mut app, |page, ctx| {
            page.handle_action(
                &PrivacyPageAction::SetClipboardAccess(Osc52ClipboardAccess::ReadWrite),
                ctx,
            )
        });
        parse_clipboard_packet(&mut app, &terminal, b"\x1b]52;c;?\x1b\\", "read-write").await;
        assert_eq!(rx.try_recv().unwrap(), b"\x1b]52;c;bmV3\x1b\\");

        page.update(&mut app, |page, ctx| {
            page.handle_action(
                &PrivacyPageAction::SetClipboardAccess(Osc52ClipboardAccess::Deny),
                ctx,
            )
        });
        parse_clipboard_packet(
            &mut app,
            &terminal,
            b"\x1b]52;c;b2xk\x07\x1b]52;c;?\x07",
            "deny",
        )
        .await;
        app.update(|ctx| assert_eq!(ctx.clipboard().read().plain_text, "new"));
        assert!(rx.is_empty());
    });
}
