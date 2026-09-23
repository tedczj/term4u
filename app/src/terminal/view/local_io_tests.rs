use std::cell::RefCell;
use std::ffi::OsString;
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStringExt;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use instant::Instant;
use nix::fcntl::{FcntlArg, OFlag, fcntl};
use nix::pty::openpty;
use nix::sys::termios::{SetArg, cfmakeraw, tcgetattr, tcsetattr};
use pathfinder_geometry::vector::vec2f;
use settings::Setting;
use warp_core::features::FeatureFlag;
use warpui::clipboard::ClipboardContent;
use warpui::presenter::Presenter;
use warpui::{
    App, EntityIdSet, Event as UiEvent, TypedActionView, ViewHandle, WindowId, WindowInvalidation,
};

use super::*;
use crate::terminal::model_events::ModelEvent;
use crate::terminal::settings::Osc52ClipboardAccess;
use crate::terminal::view::{Event, TerminalAction, TerminalInputState};
use crate::test_util::terminal::{
    add_window_with_id_and_terminal, initialize_app_for_terminal_view,
};

fn scene(
    app: &mut App,
    window: WindowId,
    terminal: &ViewHandle<TerminalView>,
) -> Rc<RefCell<Presenter>> {
    let mut presenter = Presenter::new(window);
    let mut updated = EntityIdSet::default();
    updated.insert(app.root_view_id(window).unwrap());
    updated.insert(terminal.id());
    app.update(move |ctx| {
        presenter.invalidate(
            WindowInvalidation {
                updated,
                ..Default::default()
            },
            ctx,
        );
        presenter.build_scene(vec2f(800., 600.), 1., None, ctx);
        Rc::new(RefCell::new(presenter))
    })
}

fn send(app: &mut App, window: WindowId, presenter: &Rc<RefCell<Presenter>>, event: UiEvent) {
    let presenter = presenter.clone();
    app.update(move |ctx| ctx.simulate_window_event(event, window, presenter));
}

fn writes(app: &mut App, terminal: &ViewHandle<TerminalView>) -> async_channel::Receiver<Vec<u8>> {
    let (tx, rx) = async_channel::unbounded();
    let model = terminal.read(app, |view, _| view.model.clone());
    app.update(|ctx| {
        ctx.subscribe_to_view(terminal, move |_, event, _| {
            let bytes = match event {
                Event::WriteBytesToPty { bytes } => Some(bytes),
                Event::ClipboardResponse(response) => Some(&response.bytes),
                _ => None,
            };
            if let Some(bytes) = bytes {
                assert!(
                    model.try_lock().is_some(),
                    "PTY writes must not hold the model lock"
                );
                tx.try_send(bytes.to_vec()).unwrap();
            }
        });
    });
    rx
}

/// A title in the same parser stream is a barrier: all earlier OSC/model events were delivered.
async fn parse(app: &mut App, terminal: &ViewHandle<TerminalView>, bytes: &[u8]) {
    static BARRIER: AtomicU64 = AtomicU64::new(0);
    let title = format!(
        "l0-parser-barrier-{}",
        BARRIER.fetch_add(1, Ordering::Relaxed)
    );
    let osc = format!("\x1b]2;{title}\x07");
    let (tx, rx) = async_channel::unbounded();
    let events = terminal.read(app, |view, _| view.model_events.clone());
    app.update(|ctx| {
        ctx.subscribe_to_model(&events, move |_, event, _| {
            if matches!(event, ModelEvent::Title(actual) if actual == &title) {
                let _ = tx.try_send(());
            }
        });
    });
    terminal.update(app, |view, _| {
        let mut model = view.model.lock();
        model.process_bytes(bytes);
        model.process_bytes(osc.as_bytes());
    });
    rx.recv().await.unwrap();
}

fn set_clipboard_policy(app: &mut App, policy: Osc52ClipboardAccess) {
    app.update(|ctx| {
        TerminalSettings::handle(ctx).update(ctx, |settings, ctx| {
            settings
                .osc52_clipboard_access
                .set_value(policy, ctx)
                .unwrap();
        });
    });
}

#[test]
fn l0_02_paste_plan_preserves_editor_text_and_rejects_native_control_injection() {
    assert_eq!(
        plan_paste("中\n文".into(), true, false),
        PastePlan::Editor("中\n文".into())
    );
    assert_eq!(plan_paste("".into(), false, false), PastePlan::Empty);
    for payload in ["\0", "\x1b[201~echo bad\r", "\x03", "\u{009b}201~"] {
        assert!(matches!(
            plan_paste(payload.into(), false, true),
            PastePlan::Reject(_)
        ));
    }
    assert!(matches!(
        plan_paste("x".repeat(MAX_LOCAL_TRANSFER_BYTES + 1), true, false),
        PastePlan::Reject(_)
    ));
    assert_eq!(
        plan_paste("a\nb".into(), false, false),
        PastePlan::Confirm(b"a\nb".to_vec())
    );
}

#[test]
fn l0_02_native_paste_uses_parsed_mode_and_a_real_raw_pty() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        let rx = writes(&mut app, &terminal);
        parse(&mut app, &terminal, b"\x1b[?1049h\x1b[?2004h").await;
        terminal.update(&mut app, |view, ctx| {
            view.focus(ctx);
            ctx.clipboard()
                .write(ClipboardContent::plain_text("中\n文\r\ntail".to_owned()));
            view.handle_action(&TerminalAction::Paste, ctx);
        });
        let bytes = rx.try_recv().unwrap();
        let expected = "\x1b[200~中\n文\r\ntail\x1b[201~".as_bytes();
        assert_eq!(bytes, expected);
        assert!(
            rx.try_recv().is_err(),
            "A paste must enqueue exactly one write"
        );

        let pair = openpty(None, None).unwrap();
        // nix 0.26 returns owned raw descriptors; transfer both into RAII files exactly once.
        let mut master = unsafe { File::from_raw_fd(pair.master) };
        let mut slave = unsafe { File::from_raw_fd(pair.slave) };
        let mut attrs = tcgetattr(slave.as_raw_fd()).unwrap();
        cfmakeraw(&mut attrs);
        tcsetattr(slave.as_raw_fd(), SetArg::TCSANOW, &attrs).unwrap();
        fcntl(slave.as_raw_fd(), FcntlArg::F_SETFL(OFlag::O_NONBLOCK)).unwrap();
        master.write_all(&bytes).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut actual = Vec::new();
        while actual.len() < expected.len() && Instant::now() < deadline {
            let mut chunk = [0; 128];
            match slave.read(&mut chunk) {
                Ok(n) if n > 0 => actual.extend_from_slice(&chunk[..n]),
                Ok(_) => break,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(2));
                }
                Err(error) => panic!("Raw PTY read failed: {error}"),
            }
        }
        assert_eq!(
            actual, expected,
            "Kernel PTY delivery must preserve every byte"
        );
        parse(&mut app, &terminal, b"\x1b[?2004l").await;
        terminal.update(&mut app, |view, ctx| {
            view.paste_text("single line".into(), ctx)
        });
        assert_eq!(rx.try_recv().unwrap(), b"single line");
    });
}

#[test]
fn l0_02_multiline_confirmation_cancels_and_rejects_stale_requests() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        let rx = writes(&mut app, &terminal);
        parse(&mut app, &terminal, b"\x1b[?1049h").await;
        terminal.update(&mut app, |view, ctx| {
            view.focus(ctx);
            view.paste_text("one\ntwo".into(), ctx);
            let first = view.local_io.pending_paste.as_ref().unwrap().request;
            view.finish_pending_paste(first, false, ctx);
            assert!(view.local_io.pending_paste.is_none());
            view.paste_text("three\nfour".into(), ctx);
            let second = view.local_io.pending_paste.as_ref().unwrap().request;
            view.finish_pending_paste(first, true, ctx);
            assert_eq!(
                view.local_io.pending_paste.as_ref().unwrap().request,
                second
            );
            view.handle_action(&TerminalAction::TypedCharacters("x".into()), ctx);
            view.finish_pending_paste(second, true, ctx);
        });
        assert_eq!(rx.try_recv().unwrap(), b"x");
        assert!(rx.try_recv().is_err());
        terminal.update(&mut app, |view, ctx| {
            view.paste_text("approved\ntext".into(), ctx);
            let request = view.local_io.pending_paste.as_ref().unwrap().request;
            view.finish_pending_paste(request, true, ctx);
            view.finish_pending_paste(request, true, ctx);
        });
        assert_eq!(rx.try_recv().unwrap(), b"approved\ntext");
        assert!(rx.try_recv().is_err());
    });
}

#[test]
fn l0_05_platform_drop_hits_input_and_output_once_but_not_outside() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (window, terminal) = add_window_with_id_and_terminal(&mut app, None);
        let presenter = scene(&mut app, window, &terminal);
        for position in [vec2f(400., 100.), vec2f(400., 590.)] {
            terminal.update(&mut app, |view, ctx| view.clear_buffer(ctx));
            send(
                &mut app,
                window,
                &presenter,
                UiEvent::DragFiles { location: position },
            );
            terminal.read(&app, |view, _| assert!(view.file_drop_active));
            send(
                &mut app,
                window,
                &presenter,
                UiEvent::DragAndDropFiles {
                    location: position,
                    paths: vec!["/tmp/one file".into(), "/tmp/中文.png".into()],
                },
            );
            terminal.read(&app, |view, ctx| {
                let shell = view.shell_family(ctx);
                let expected = format!(
                    "{} {}",
                    shell.escape("/tmp/one file"),
                    shell.escape("/tmp/中文.png")
                );
                assert_eq!(view.input.as_ref(ctx).buffer_text(ctx), expected);
                assert!(!view.file_drop_active);
            });
        }
        let before = terminal.read(&app, |view, ctx| view.input.as_ref(ctx).buffer_text(ctx));
        send(
            &mut app,
            window,
            &presenter,
            UiEvent::DragAndDropFiles {
                location: vec2f(801., 200.),
                paths: vec!["/tmp/not-in-this-pane".into()],
            },
        );
        terminal.read(&app, |view, ctx| {
            assert_eq!(view.input.as_ref(ctx).buffer_text(ctx), before)
        });
        send(&mut app, window, &presenter, UiEvent::DragFileExit);
        terminal.read(&app, |view, _| assert!(!view.file_drop_active));
    });
}

#[test]
fn l0_05_non_utf8_drop_is_atomic_and_never_lossy() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        terminal.update(&mut app, |view, ctx| {
            view.input
                .update(ctx, |input, ctx| input.replace_buffer_content("draft", ctx));
            view.drop_paths(
                &[
                    PathBuf::from("/tmp/good"),
                    PathBuf::from(OsString::from_vec(vec![0xff])),
                ],
                ctx,
            );
            assert_eq!(view.input.as_ref(ctx).buffer_text(ctx), "draft");
        });
    });
}

#[test]
fn l0_07_ime_platform_composition_commits_once_and_cancels_without_bytes() {
    App::test((), |mut app| async move {
        let _ime = FeatureFlag::ImeMarkedText.override_enabled(true);
        initialize_app_for_terminal_view(&mut app);
        let (window, terminal) = add_window_with_id_and_terminal(&mut app, None);
        parse(&mut app, &terminal, b"\x1b[?1049h").await;
        terminal.update(&mut app, |view, ctx| view.focus(ctx));
        let rx = writes(&mut app, &terminal);
        let presenter = scene(&mut app, window, &terminal);
        send(
            &mut app,
            window,
            &presenter,
            UiEvent::SetMarkedText {
                marked_text: "中文候选".into(),
                selected_range: 0..2,
            },
        );
        terminal.read(&app, |view, _| {
            assert_eq!(
                view.model.lock().alt_screen().grid_handler().marked_text(),
                Some("中文候选")
            )
        });
        assert!(rx.try_recv().is_err());
        send(
            &mut app,
            window,
            &presenter,
            UiEvent::TypedCharacters {
                chars: "中文".into(),
            },
        );
        assert_eq!(rx.try_recv().unwrap(), "中文".as_bytes());
        send(&mut app, window, &presenter, UiEvent::ClearMarkedText);
        assert!(rx.try_recv().is_err());
        terminal.read(&app, |view, _| {
            assert!(
                view.model
                    .lock()
                    .alt_screen()
                    .grid_handler()
                    .marked_text()
                    .is_none()
            )
        });
        send(
            &mut app,
            window,
            &presenter,
            UiEvent::SetMarkedText {
                marked_text: "取消".into(),
                selected_range: 0..0,
            },
        );
        send(&mut app, window, &presenter, UiEvent::ClearMarkedText);
        assert!(rx.try_recv().is_err());
    });
}

#[test]
fn l0_07_osc52_parser_obeys_separate_read_write_policy_and_exact_response() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        let rx = writes(&mut app, &terminal);
        app.update(|ctx| {
            ctx.clipboard()
                .write(ClipboardContent::plain_text("original".into()))
        });
        parse(&mut app, &terminal, b"\x1b]52;c;bmV3\x07\x1b]52;c;?\x07").await;
        app.update(|ctx| assert_eq!(ctx.clipboard().read().plain_text, "original"));
        assert!(rx.try_recv().is_err());
        set_clipboard_policy(&mut app, Osc52ClipboardAccess::WriteOnly);
        parse(&mut app, &terminal, b"\x1b]52;c;bmV3\x07\x1b]52;c;?\x07").await;
        app.update(|ctx| assert_eq!(ctx.clipboard().read().plain_text, "new"));
        assert!(rx.try_recv().is_err());
        set_clipboard_policy(&mut app, Osc52ClipboardAccess::ReadWrite);
        parse(&mut app, &terminal, b"\x1b]52;c;?\x1b\\").await;
        assert_eq!(rx.try_recv().unwrap(), b"\x1b]52;c;bmV3\x1b\\");
        assert!(rx.try_recv().is_err());
        parse(&mut app, &terminal, b"\x1b]52;p;?\x07\x1b]52;c;%%%\x07").await;
        app.update(|ctx| assert_eq!(ctx.clipboard().read().plain_text, "new"));
        assert!(rx.try_recv().is_err());
    });
}

#[test]
fn l0_07_clipboard_event_debug_does_not_disclose_payload() {
    let event = crate::terminal::event::Event::ClipboardStore(
        ClipboardType::Clipboard,
        "private-token".into(),
        crate::terminal::event_listener::ChannelEventListener::new_for_test()
            .reserve_clipboard_request()
            .unwrap(),
    );
    assert!(!format!("{event:?}").contains("private-token"));
}

#[test]
fn l0_02_parsed_mode_changes_invalidate_a_pending_confirmation() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        let rx = writes(&mut app, &terminal);
        parse(&mut app, &terminal, b"\x1b[?1049h").await;
        let request = terminal.update(&mut app, |view, ctx| {
            view.focus(ctx);
            view.paste_text("first\nsecond".into(), ctx);
            view.local_io.pending_paste.as_ref().unwrap().request
        });
        parse(&mut app, &terminal, b"\x1b[?2004h\x1b[?2004l").await;
        terminal.update(&mut app, |view, ctx| {
            assert!(view.local_io.pending_paste.is_none());
            view.finish_pending_paste(request, true, ctx);
        });
        assert!(rx.try_recv().is_err());
    });
}

#[test]
fn l0_07_osc52_decoded_payload_limit_is_enforced_before_clipboard_events() {
    use base64::Engine;

    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        set_clipboard_policy(&mut app, Osc52ClipboardAccess::WriteOnly);
        app.update(|ctx| {
            ctx.clipboard()
                .write(ClipboardContent::plain_text("original".into()))
        });
        let encoded =
            base64::engine::general_purpose::STANDARD
                .encode(vec![b'X'; MAX_LOCAL_TRANSFER_BYTES + 1]);
        let sequence = format!("\x1b]52;c;{encoded}\x07");
        parse(&mut app, &terminal, sequence.as_bytes()).await;
        app.update(|ctx| assert_eq!(ctx.clipboard().read().plain_text, "original"));

        let encoded =
            base64::engine::general_purpose::STANDARD.encode(vec![b'X'; MAX_LOCAL_TRANSFER_BYTES]);
        let sequence = format!("\x1b]52;c;{encoded}\x1b\\");
        parse(&mut app, &terminal, sequence.as_bytes()).await;
        app.update(|ctx| {
            assert_eq!(
                ctx.clipboard().read().plain_text,
                "X".repeat(MAX_LOCAL_TRANSFER_BYTES)
            )
        });
    });
}

#[test]
fn l0_07_compound_selection_cannot_bypass_the_host_clipboard_policy() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        set_clipboard_policy(&mut app, Osc52ClipboardAccess::ReadWrite);
        app.update(|ctx| {
            ctx.clipboard()
                .write(ClipboardContent::plain_text("original".into()))
        });
        let rx = writes(&mut app, &terminal);
        parse(&mut app, &terminal, b"\x1b]52;cp;bmV3\x07\x1b]52;cp;?\x07").await;
        app.update(|ctx| assert_eq!(ctx.clipboard().read().plain_text, "original"));
        assert!(rx.is_empty());
    });
}

#[test]
fn l0_07_reset_discards_queued_clipboard_reads_and_writes() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        set_clipboard_policy(&mut app, Osc52ClipboardAccess::ReadWrite);
        app.update(|ctx| {
            ctx.clipboard()
                .write(ClipboardContent::plain_text("original".into()))
        });
        let rx = writes(&mut app, &terminal);
        parse(
            &mut app,
            &terminal,
            b"\x1b]52;c;b2xk\x07\x1b]52;c;?\x07\x1bc",
        )
        .await;
        app.update(|ctx| assert_eq!(ctx.clipboard().read().plain_text, "original"));
        assert!(rx.is_empty());
        parse(&mut app, &terminal, b"\x1b]52;c;bmV3\x07\x1b]52;c;?\x07").await;
        assert_eq!(rx.try_recv().unwrap(), b"\x1b]52;c;bmV3\x07");
        assert!(rx.is_empty());
    });
}

#[test]
fn l0_07_clipboard_queue_is_bounded_and_recovers_after_delivery() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        set_clipboard_policy(&mut app, Osc52ClipboardAccess::ReadWrite);
        app.update(|ctx| {
            ctx.clipboard()
                .write(ClipboardContent::plain_text("safe".into()))
        });
        let rx = writes(&mut app, &terminal);
        parse(&mut app, &terminal, &b"\x1b]52;c;?\x07".repeat(20)).await;
        assert_eq!(rx.len(), 8);
        while let Ok(bytes) = rx.try_recv() {
            assert_eq!(bytes, b"\x1b]52;c;c2FmZQ==\x07");
        }
        parse(&mut app, &terminal, b"\x1b]52;c;?\x07").await;
        assert_eq!(rx.try_recv().unwrap(), b"\x1b]52;c;c2FmZQ==\x07");
    });
}

#[test]
fn l0_07_clipboard_request_identity_and_slot_follow_the_origin() {
    let listener = crate::terminal::event_listener::ChannelEventListener::new_for_test();
    let request = listener.reserve_clipboard_request().unwrap();
    listener.set_clipboard_session(1);
    assert!(listener.clipboard_request_is_current(&request));
    listener.set_clipboard_session(1);
    assert!(listener.clipboard_request_is_current(&request));
    let other = crate::terminal::event_listener::ChannelEventListener::new_for_test();
    assert!(!other.clipboard_request_is_current(&request));
    listener.set_clipboard_session(2);
    assert!(!listener.clipboard_request_is_current(&request));
    let new_request = listener.reserve_clipboard_request().unwrap();
    assert!(listener.clipboard_request_is_current(&new_request));
    listener.end_clipboard_session(1);
    assert!(listener.clipboard_request_is_current(&new_request));
    listener.end_clipboard_session(2);
    assert!(!listener.clipboard_request_is_current(&new_request));
    listener.reset_clipboard_scope();
    assert!(!listener.clipboard_request_is_current(&new_request));
}

#[test]
fn l0_07_exiting_terminal_rejects_clipboard_events_already_queued() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        set_clipboard_policy(&mut app, Osc52ClipboardAccess::ReadWrite);
        app.update(|ctx| {
            ctx.clipboard()
                .write(ClipboardContent::plain_text("original".into()))
        });
        let rx = writes(&mut app, &terminal);
        terminal.update(&mut app, |view, _| {
            let mut model = view.model.lock();
            model.process_bytes("\x1b]52;c;b2xk\x07\x1b]52;c;?\x07");
            model.exit(crate::terminal::model::terminal_model::ExitReason::ShellNotFound);
        });
        parse(&mut app, &terminal, b"\x1b]52;c;bmV3\x07\x1b]52;c;?\x07").await;
        app.update(|ctx| assert_eq!(ctx.clipboard().read().plain_text, "original"));
        assert!(rx.is_empty());
    });
}

#[test]
fn l0_07_revoking_permission_invalidates_an_already_encoded_reply() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        set_clipboard_policy(&mut app, Osc52ClipboardAccess::ReadWrite);
        let (tx, rx) = async_channel::unbounded();
        app.update(|ctx| {
            ctx.clipboard()
                .write(ClipboardContent::plain_text("safe".into()));
            ctx.subscribe_to_view(&terminal, move |_, event, _| {
                if let Event::ClipboardResponse(response) = event {
                    tx.try_send(response.clone()).unwrap();
                }
            });
        });
        parse(&mut app, &terminal, b"\x1b]52;c;?\x07").await;
        let response = rx.try_recv().unwrap();
        assert_eq!(&response.bytes[..], b"\x1b]52;c;c2FmZQ==\x07");
        terminal.read(&app, |view, _| {
            assert!(
                view.model
                    .lock()
                    .clipboard_request_is_current(&response.request)
            )
        });
        set_clipboard_policy(&mut app, Osc52ClipboardAccess::Deny);
        terminal.read(&app, |view, _| {
            assert!(
                !view
                    .model
                    .lock()
                    .clipboard_request_is_current(&response.request)
            )
        });
    });
}

#[test]
fn l0_07_native_normal_grid_has_an_ime_anchor_even_with_hidden_cursor() {
    App::test((), |mut app| async move {
        let _ime = FeatureFlag::ImeMarkedText.override_enabled(true);
        initialize_app_for_terminal_view(&mut app);
        let (window, terminal) = add_window_with_id_and_terminal(&mut app, None);
        terminal.update(&mut app, |view, ctx| {
            {
                let mut model = view.model.lock();
                *model = crate::terminal::model::TerminalModel::mock(
                    None,
                    Some(model.event_proxy.clone()),
                );
                model.simulate_long_running_block("cat", "n ");
                assert!(matches!(
                    model.terminal_input_state(),
                    TerminalInputState::LongRunningCommand
                ));
                assert!(model.block_list().active_block().is_visible());
            }
            view.focus(ctx);
            ctx.notify();
        });
        parse(&mut app, &terminal, b"").await;
        let presenter = scene(&mut app, window, &terminal);
        let cursor_id = format!("terminal_view:cursor_{}", terminal.id());
        let before = presenter
            .borrow()
            .position_cache()
            .get_position(&cursor_id)
            .expect("native normal-grid input must publish a caret anchor");
        let rx = writes(&mut app, &terminal);
        send(
            &mut app,
            window,
            &presenter,
            UiEvent::SetMarkedText {
                marked_text: "中abc".into(),
                selected_range: 1..2,
            },
        );
        let presenter = scene(&mut app, window, &terminal);
        let after = presenter
            .borrow()
            .position_cache()
            .get_position(&cursor_id)
            .unwrap();
        let cell_width = terminal.read(&app, |view, _| view.size_info.cell_width_px().as_f32());
        assert!((after.origin_x() - before.origin_x() - 3. * cell_width).abs() < 0.01);
        assert_eq!(before.origin_y(), after.origin_y());
        assert!(rx.try_recv().is_err());
        parse(&mut app, &terminal, b"\x1b[?25l").await;
        let presenter = scene(&mut app, window, &terminal);
        assert_eq!(
            presenter.borrow().position_cache().get_position(&cursor_id),
            Some(after)
        );
        send(&mut app, window, &presenter, UiEvent::ClearMarkedText);
        let presenter = scene(&mut app, window, &terminal);
        assert_eq!(
            presenter.borrow().position_cache().get_position(&cursor_id),
            Some(before)
        );
        assert!(rx.try_recv().is_err());
    });
}

#[test]
fn l0_07_bell_and_notification_policy_limits_effects_without_changing_focus() {
    use std::cell::Cell;

    use warpui::windowing::WindowManager;
    use warpui::windowing::state::ApplicationStage;

    use crate::terminal::session_settings::{NotificationsMode, SessionSettings};

    App::test((), |mut app| async move {
        let _notifications = FeatureFlag::PluggableNotifications.override_enabled(true);
        initialize_app_for_terminal_view(&mut app);
        let (window, terminal) = add_window_with_id_and_terminal(&mut app, None);
        let rings = Rc::new(Cell::new(0));
        let model = terminal.read(&app, |view, _| view.model.clone());
        app.add_singleton_model({
            let rings = rings.clone();
            let model = model.clone();
            move |_| {
                AudibleBell::for_test(move || {
                    assert!(
                        model.try_lock().is_some(),
                        "host sound must not hold the terminal lock"
                    );
                    rings.set(rings.get() + 1);
                    Ok(())
                })
            }
        });
        let clock = Rc::new(Cell::new(Instant::now()));
        let notifications = Rc::new(RefCell::new(Vec::new()));
        terminal.update(&mut app, |view, ctx| {
            let clock = clock.clone();
            view.local_io.clock_for_test = Some(Box::new(move || clock.get()));
            let notifications = notifications.clone();
            let model = model.clone();
            view.local_io.notification_for_test = Some(Box::new(move |content| {
                assert!(
                    model.try_lock().is_some(),
                    "host notification must not hold the terminal lock"
                );
                notifications.borrow_mut().push(content);
                Ok(())
            }));
            view.focus(ctx);
        });
        parse(&mut app, &terminal, b"\x07\x1b]777;notify;title;body\x07").await;
        assert_eq!(rings.get(), 0);
        assert!(notifications.borrow().is_empty());
        app.update(|ctx| {
            TerminalSettings::handle(ctx).update(ctx, |s, ctx| {
                s.use_audible_bell.set_value(true, ctx).unwrap()
            });
            SessionSettings::handle(ctx).update(ctx, |s, ctx| {
                let mut value = s.notifications.value().clone();
                value.mode = NotificationsMode::Enabled;
                s.notifications.set_value(value, ctx).unwrap();
            });
            WindowManager::handle(ctx).update(ctx, |m, _| {
                m.overwrite_for_test(ApplicationStage::Active, Some(window))
            });
        });
        let _presenter = scene(&mut app, window, &terminal);
        terminal.update(&mut app, |view, ctx| {
            view.focus(ctx);
            assert!(ctx.is_self_or_child_focused());
        });
        let focus = app.read(|ctx| ctx.focused_view_id(window));
        parse(&mut app, &terminal, &[7; 100]).await;
        assert_eq!(rings.get(), 1);
        assert!(notifications.borrow().is_empty());
        clock.set(clock.get() + Duration::from_millis(249));
        parse(&mut app, &terminal, b"\x07").await;
        assert_eq!(rings.get(), 1);
        clock.set(clock.get() + Duration::from_millis(1));
        app.update(|ctx| {
            WindowManager::handle(ctx).update(ctx, |m, _| {
                m.overwrite_for_test(ApplicationStage::Inactive, None)
            })
        });
        parse(&mut app, &terminal, &[7; 100]).await;
        assert_eq!(rings.get(), 2);
        assert_eq!(notifications.borrow().len(), 1);
        assert!(
            !notifications.borrow()[0].play_sound(),
            "Bell must not ring again through the notification"
        );
        clock.set(clock.get() + Duration::from_secs(1));
        parse(&mut app, &terminal, b"\x1b]777;notify;build;done\x07").await;
        assert_eq!(notifications.borrow().len(), 2);
        assert!(notifications.borrow()[1].play_sound());
        app.update(|ctx| {
            SessionSettings::handle(ctx).update(ctx, |s, ctx| {
                let mut value = s.notifications.value().clone();
                value.is_needs_attention_enabled = false;
                s.notifications.set_value(value, ctx).unwrap();
            })
        });
        clock.set(clock.get() + Duration::from_secs(1));
        parse(
            &mut app,
            &terminal,
            b"\x07\x1b]777;notify;hidden;disabled\x07",
        )
        .await;
        assert_eq!(rings.get(), 3);
        assert_eq!(notifications.borrow().len(), 2);
        app.update(|ctx| {
            SessionSettings::handle(ctx).update(ctx, |s, ctx| {
                let mut value = s.notifications.value().clone();
                value.is_needs_attention_enabled = true;
                value.play_notification_sound = false;
                s.notifications.set_value(value, ctx).unwrap();
            })
        });
        parse(&mut app, &terminal, b"\x1b]777;notify;silent;done\x07").await;
        assert_eq!(notifications.borrow().len(), 3);
        assert!(!notifications.borrow()[2].play_sound());
        assert_eq!(app.read(|ctx| ctx.focused_view_id(window)), focus);
        let other = app.add_view(window, |ctx| {
            crate::editor::EditorView::new(Default::default(), ctx)
        });
        other.update(&mut app, |_, ctx| ctx.focus_self());
        app.update(|ctx| {
            WindowManager::handle(ctx).update(ctx, |m, _| {
                m.overwrite_for_test(ApplicationStage::Active, Some(window))
            })
        });
        clock.set(clock.get() + Duration::from_secs(1));
        parse(&mut app, &terminal, b"\x1b]777;notify;background;done\x07").await;
        assert_eq!(notifications.borrow().len(), 4);
        assert_eq!(
            app.read(|ctx| ctx.focused_view_id(window)),
            Some(other.id())
        );
        app.update(|ctx| {
            SessionSettings::handle(ctx).update(ctx, |s, ctx| {
                let mut value = s.notifications.value().clone();
                value.mode = NotificationsMode::Disabled;
                s.notifications.set_value(value, ctx).unwrap();
            })
        });
        clock.set(clock.get() + Duration::from_secs(1));
        parse(&mut app, &terminal, b"\x1b]777;notify;disabled;done\x07").await;
        assert_eq!(notifications.borrow().len(), 4);
    });
}

#[test]
fn l0_07_notification_payload_is_bounded_before_model_dispatch() {
    App::test((), |mut app| async move {
        let _notifications = FeatureFlag::PluggableNotifications.override_enabled(true);
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        let sizes = Rc::new(RefCell::new(Vec::new()));
        let events = terminal.read(&app, |view, _| view.model_events.clone());
        app.update(|ctx| {
            let sizes = sizes.clone();
            ctx.subscribe_to_model(&events, move |_, event, _| {
                if let ModelEvent::PluggableNotification { title, body } = event {
                    sizes.borrow_mut().push((
                        title.as_ref().map(|t| t.chars().count()),
                        body.chars().count(),
                    ));
                }
            });
        });
        let osc = format!(
            "\x1b]777;notify;{};{}\x07",
            "题".repeat(500),
            "文".repeat(5000)
        );
        parse(&mut app, &terminal, osc.as_bytes()).await;
        assert_eq!(*sizes.borrow(), vec![(Some(40), 120)]);
    });
}

#[test]
fn l0_07_alt_cursor_anchor_belongs_to_terminal_and_survives_hidden_mode() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (window, terminal) = add_window_with_id_and_terminal(&mut app, None);
        parse(&mut app, &terminal, b"\x1b[?1049h\x1b[3 q\x1b[4;8H").await;
        terminal.update(&mut app, |view, ctx| view.focus(ctx));
        let presenter = scene(&mut app, window, &terminal);
        let id = format!("terminal_view:cursor_{}", terminal.id());
        let before = presenter
            .borrow()
            .position_cache()
            .get_position(&id)
            .expect("alt screen must bind its IME anchor to the terminal view");
        parse(&mut app, &terminal, b"\x1b[?25l").await;
        let presenter = scene(&mut app, window, &terminal);
        assert_eq!(
            presenter.borrow().position_cache().get_position(id),
            Some(before)
        );
    });
}

#[test]
fn l0_07_cursor_blink_respects_setting_focus_and_parsed_modes() {
    use warpui::windowing::WindowManager;
    use warpui::windowing::state::ApplicationStage;

    use crate::settings::{AppEditorSettings, CursorBlink};
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (window, terminal) = add_window_with_id_and_terminal(&mut app, None);
        parse(&mut app, &terminal, b"\x1b[?1049h\x1b[1 q").await;
        terminal.update(&mut app, |view, ctx| view.focus(ctx));
        app.update(|ctx| {
            WindowManager::handle(ctx).update(ctx, |m, _| {
                m.overwrite_for_test(ApplicationStage::Active, Some(window))
            })
        });
        terminal.read(&app, |view, ctx| {
            assert!(view.model.lock().alt_screen().cursor_style().blinking);
            assert!(view.native_cursor_blink_epoch(ctx).is_some());
        });
        app.update(|ctx| {
            AppEditorSettings::handle(ctx).update(ctx, |s, ctx| {
                s.cursor_blink
                    .set_value(CursorBlink::Disabled, ctx)
                    .unwrap()
            })
        });
        terminal.read(&app, |view, ctx| {
            assert!(view.native_cursor_blink_epoch(ctx).is_none())
        });
        app.update(|ctx| {
            AppEditorSettings::handle(ctx).update(ctx, |s, ctx| {
                s.cursor_blink.set_value(CursorBlink::Enabled, ctx).unwrap()
            })
        });
        app.update(|ctx| {
            WindowManager::handle(ctx).update(ctx, |m, _| {
                m.overwrite_for_test(ApplicationStage::Inactive, None)
            })
        });
        terminal.read(&app, |view, ctx| {
            assert!(view.native_cursor_blink_epoch(ctx).is_none())
        });
        parse(&mut app, &terminal, b"\x1b[?12l").await;
        terminal.read(&app, |view, _| {
            assert!(!view.model.lock().alt_screen().cursor_style().blinking)
        });
    });
}

#[test]
fn l0_07_host_effect_failures_are_throttled_and_notification_errors_are_redacted() {
    use std::cell::Cell;

    use warpui::notification::NotificationSendError;

    use crate::terminal::session_settings::{NotificationsMode, SessionSettings};
    use crate::workspace::ToastStackEvent;
    App::test((), |mut app| async move {
        let _notifications = FeatureFlag::PluggableNotifications.override_enabled(true);
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        let clock = Rc::new(Cell::new(Instant::now()));
        let rings = Rc::new(Cell::new(0));
        app.add_singleton_model({
            let rings = rings.clone();
            move |_| {
                AudibleBell::for_test(move || {
                    rings.set(rings.get() + 1);
                    Err(anyhow::anyhow!("unavailable"))
                })
            }
        });
        let toasts = Rc::new(RefCell::new(Vec::new()));
        app.update(|ctx| {
            let toasts = toasts.clone();
            ctx.subscribe_to_model(&ToastStack::handle(ctx), move |_, event, _| {
                if let ToastStackEvent::AddEphemeralToast { toast, .. } = event {
                    toasts.borrow_mut().push(toast.main_text().to_owned());
                }
            });
            TerminalSettings::handle(ctx).update(ctx, |s, ctx| {
                s.use_audible_bell.set_value(true, ctx).unwrap()
            });
            SessionSettings::handle(ctx).update(ctx, |s, ctx| {
                let mut p = s.notifications.value().clone();
                p.mode = NotificationsMode::Enabled;
                s.notifications.set_value(p, ctx).unwrap();
            });
        });
        terminal.update(&mut app, |view, _| {
            let clock = clock.clone();
            view.local_io.clock_for_test = Some(Box::new(move || clock.get()));
            view.local_io.notification_for_test = Some(Box::new(|_| {
                Err(NotificationSendError::Other {
                    error_message: "private-notification-sentinel".into(),
                })
            }));
        });
        parse(&mut app, &terminal, &[7; 100]).await;
        assert_eq!(rings.get(), 1);
        assert_eq!(toasts.borrow().len(), 1);
        clock.set(clock.get() + Duration::from_secs(1));
        parse(&mut app, &terminal, b"\x1b]777;notify;private;payload\x07").await;
        assert_eq!(toasts.borrow().len(), 1);
        clock.set(clock.get() + Duration::from_secs(30));
        parse(&mut app, &terminal, b"\x1b]777;notify;private;payload\x07").await;
        assert_eq!(
            *toasts.borrow(),
            vec!["A terminal notification could not be delivered."; 2]
        );
    });
}

#[test]
fn l0_07_native_cursor_repaint_timer_alternates_scene_visibility() {
    use warpui::r#async::Timer;
    use warpui::windowing::WindowManager;
    use warpui::windowing::state::ApplicationStage;
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (window, terminal) = add_window_with_id_and_terminal(&mut app, None);
        app.update(|ctx| {
            WindowManager::handle(ctx).update(ctx, |m, _| {
                m.overwrite_for_test(ApplicationStage::Active, Some(window))
            })
        });
        parse(
            &mut app,
            &terminal,
            b"\x1b[?1049h\x1b[?25h\x1b[1 q\x1b[4;8H",
        )
        .await;
        terminal.update(&mut app, |view, ctx| view.focus(ctx));
        let mut seen_visible = false;
        let mut seen_hidden = false;
        for _ in 0..16 {
            Timer::after(Duration::from_millis(100)).await;
            let (anchor, visible) = app.read(|ctx| {
                let presenter = ctx.presenter(window).unwrap();
                let presenter = presenter.borrow();
                let anchor = presenter
                    .position_cache()
                    .get_position(format!("terminal_view:cursor_{}", terminal.id()))
                    .expect("cursor anchor survives blink");
                let visible = presenter.scene().unwrap().layers().any(|layer| {
                    layer.rects.iter().any(|r| {
                        r.bounds == anchor && !matches!(r.background, warpui::elements::Fill::None)
                    })
                });
                (anchor, visible)
            });
            assert!(anchor.height() > 0.);
            seen_visible |= visible;
            seen_hidden |= !visible;
            if seen_visible && seen_hidden {
                break;
            }
        }
        assert!(
            seen_visible && seen_hidden,
            "repaint timer must deliver both visible and hidden cursor frames"
        );
    });
}

#[test]
fn l0_07_command_completion_notifications_use_duration_and_independent_preference() {
    use std::cell::Cell;

    use crate::terminal::session_settings::{NotificationsMode, SessionSettings};
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        terminal.update(&mut app, |view, _| {
            let mut model = view.model.lock();
            *model =
                crate::terminal::model::TerminalModel::mock(None, Some(model.event_proxy.clone()));
        });
        parse(&mut app, &terminal, b"").await;
        let sent = Rc::new(RefCell::new(Vec::new()));
        let clock = Rc::new(Cell::new(Instant::now()));
        let completed = Rc::new(Cell::new(0));
        let events = terminal.read(&app, |view, _| view.model_events.clone());
        app.update(|ctx| {
            let completed = completed.clone();
            ctx.subscribe_to_model(&events, move |_, event, _| {
                if let ModelEvent::BlockCompleted(event) = event
                    && matches!(event.block_type, crate::terminal::event::BlockType::User(_))
                {
                    completed.set(completed.get() + 1);
                }
            });
        });
        terminal.update(&mut app, |view, _| {
            let sent = sent.clone();
            let clock = clock.clone();
            view.local_io.clock_for_test = Some(Box::new(move || clock.get()));
            let model = view.model.clone();
            view.local_io.notification_for_test = Some(Box::new(move |message| {
                assert!(
                    model.try_lock().is_some(),
                    "completion notification must release the terminal lock"
                );
                sent.borrow_mut().push(message);
                Ok(())
            }));
        });
        app.update(|ctx| {
            SessionSettings::handle(ctx).update(ctx, |s, ctx| {
                let mut value = s.notifications.value().clone();
                value.mode = NotificationsMode::Enabled;
                value.is_needs_attention_enabled = false;
                value.long_running_threshold = Duration::from_secs(30);
                s.notifications.set_value(value, ctx).unwrap();
            })
        });
        for (i, seconds) in [1, 40, 40].into_iter().enumerate() {
            clock.set(clock.get() + Duration::from_secs(2));
            if i == 2 {
                app.update(|ctx| {
                    SessionSettings::handle(ctx).update(ctx, |s, ctx| {
                        let mut value = s.notifications.value().clone();
                        value.is_long_running_enabled = false;
                        s.notifications.set_value(value, ctx).unwrap();
                    })
                });
            }
            terminal.update(&mut app, |view, _| {
                let mut model = view.model.lock();
                model.simulate_long_running_block("private-command", "private-output");
                model
                    .block_list_mut()
                    .active_block_mut()
                    .override_start_ts(chrono::Local::now() - chrono::Duration::seconds(seconds));
            });
            let hook=serde_json::json!({"hook":"CommandFinished","value":{"exit_code":0,"next_block_id":format!("l0-notification-next-{i}"),"session_id":123}}).to_string();
            let bytes = format!("\x1bP$d{}\x1b\\", hex::encode(hook));
            parse(&mut app, &terminal, bytes.as_bytes()).await;
            if i == 0 {
                assert!(sent.borrow().is_empty());
            }
        }
        assert_eq!(completed.get(), 3);
        assert_eq!(sent.borrow().len(), 1);
        let message = &sent.borrow()[0];
        assert_eq!(message.title(), "Command completed");
        assert!(!message.body().contains("private"));
        assert!(message.data().is_some());
    });
}

#[test]
fn l0_08_alt_screen_mouse_reporting_uses_live_session_state() {
    use crate::pane_group::focus_state::{PaneFocusHandle, PaneGroupFocusState};
    use crate::pane_group::pane::TerminalPaneId;

    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (window, terminal) = add_window_with_id_and_terminal(&mut app, None);
        let rx = writes(&mut app, &terminal);
        parse(&mut app, &terminal, b"\x1b[?1049h\x1b[?1003h\x1b[?1006h").await;
        let pane = TerminalPaneId::dummy_terminal_pane_id();
        let other = TerminalPaneId::dummy_terminal_pane_id();
        for active in [false, true, false] {
            let focused = if active { pane } else { other };
            let focus =
                app.add_model(|_| PaneGroupFocusState::new(focused.into(), Some(focused), true));
            terminal.update(&mut app, |view, ctx| {
                view.install_focus_handle(PaneFocusHandle::new(pane.into(), focus), ctx);
                view.focus(ctx);
            });
            let presenter = scene(&mut app, window, &terminal);
            send(
                &mut app,
                window,
                &presenter,
                UiEvent::MouseMoved {
                    position: vec2f(40., 40.),
                    cmd: false,
                    shift: false,
                    is_synthetic: false,
                },
            );
            if active {
                let bytes = rx
                    .try_recv()
                    .expect("active session should report mouse motion");
                assert!(bytes.starts_with(b"\x1b[<"));
            } else {
                assert!(
                    rx.try_recv().is_err(),
                    "inactive session must not report mouse motion"
                );
            }
        }
    });
}

#[test]
fn l0_08_shell_clear_then_completed_command_keeps_old_output_above_viewport() {
    use warpui::units::{IntoLines, IntoPixels};

    use crate::terminal::{SizeInfo, SizeUpdate, SizeUpdateReason, TerminalModel};
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        terminal.update(&mut app, |view, _| {
            let mut model = view.model.lock();
            *model = TerminalModel::mock(None, Some(model.event_proxy.clone()));
            let size = SizeInfo::new(
                vec2f(800., 600.),
                10_f32.into_pixels(),
                20_f32.into_pixels(),
                0_f32.into_pixels(),
                0_f32.into_pixels(),
            );
            model.resize(SizeUpdate {
                update_reason: SizeUpdateReason::AfterLayout,
                last_size: view.size_info,
                new_size: size,
                new_gap_height: Some(30.into_lines()),
                natural_rows: 30,
                natural_cols: 80,
            });
            view.size_info = size;
            model
                .block_list_mut()
                .set_next_gap_height_in_lines(30.into_lines());
            model.simulate_block("history", "retained history\r\n");
        });
        parse(&mut app, &terminal, b"").await;
        let before = terminal.update(&mut app, |view, ctx| {
            view.handle_wakeup(None, ctx);
            view.transcript_height
        });
        let hook = serde_json::json!({"hook":"Clear","value":{"session_id":123}}).to_string();
        parse(
            &mut app,
            &terminal,
            format!("\x1bP$d{}\x1b\\", hex::encode(hook)).as_bytes(),
        )
        .await;
        terminal.update(&mut app, |view, ctx| {
            view.handle_wakeup(None, ctx);
            assert!(view.transcript_height + 0.1 >= before + 600.);
            view.model
                .lock()
                .simulate_block("after-clear", "new output\r\n");
            view.handle_wakeup(None, ctx);
            assert!(
                view.transcript_height + 0.1 >= before + 600.,
                "old output must stay above the bottom viewport after command completion"
            );
            let model = view.model.lock();
            assert!(model.block_list().active_gap().is_some());
            assert!(
                model
                    .block_list()
                    .blocks()
                    .iter()
                    .any(|b| b.output_to_string().contains("retained history"))
            );
        });
    });
}

#[test]
fn l0_08_reading_anchor_pauses_output_following_until_scrolled_to_bottom() {
    use warpui::units::IntoPixels;

    use crate::terminal::TerminalModel;

    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        terminal.update(&mut app, |view, ctx| {
            let mut model = view.model.lock();
            *model = TerminalModel::mock(None, Some(model.event_proxy.clone()));
            drop(model);
            view.after_layout(vec2f(800., 600.), ctx);
            view.model
                .lock()
                .simulate_long_running_block("stream", &"reading row\r\n".repeat(120));
        });
        parse(&mut app, &terminal, b"").await;
        let position = terminal.update(&mut app, |view, ctx| {
            view.handle_wakeup(None, ctx);
            let position = view.transcript_height
                - view.size_info.pane_height_px
                - view.size_info.cell_height_px().as_f32() * 0.75;
            assert!(position > 0.);
            view.transcript_scroll.scroll_to(position.into_pixels());
            drop(view.render_blocks(ctx));
            position
        });
        parse(&mut app, &terminal, b"appended one\r\nappended two\r\n").await;
        terminal.update(&mut app, |view, ctx| {
            view.handle_wakeup(None, ctx);
            assert!(
                (view.transcript_scroll.scroll_start().as_f32() - position).abs() < 0.1,
                "even a partial line of upward scrolling must pause output following"
            );
            view.transcript_scroll
                .scroll_to(view.transcript_height.into_pixels());
            drop(view.render_blocks(ctx));
        });
        parse(&mut app, &terminal, b"appended after bottom\r\n").await;
        terminal.update(&mut app, |view, ctx| {
            view.handle_wakeup(None, ctx);
            let bottom = (view.transcript_height - view.size_info.pane_height_px).max(0.);
            assert!((view.transcript_scroll.scroll_start().as_f32() - bottom).abs() < 0.1);
        });
    });
}
