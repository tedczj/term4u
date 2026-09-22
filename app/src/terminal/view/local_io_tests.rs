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
use crate::terminal::view::{Event, TerminalAction};
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
            if let Event::WriteBytesToPty { bytes } = event {
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
