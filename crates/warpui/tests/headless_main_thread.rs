use libtest_mimic::{Arguments, Trial};

fn main() {
    let mut args = Arguments::from_args();
    // This test must run on the process main thread to service the macOS run loop.
    args.test_threads = Some(1);

    #[cfg(target_os = "macos")]
    let tests = vec![
        Trial::test("services_main_dispatch_queue", || {
            macos::services_main_dispatch_queue().map_err(|error| format!("{error:#}").into())
        }),
        Trial::test("l0_07_native_ime_ranges_and_delayed_close", || {
            macos::native_ime_ranges_and_delayed_close()
                .map_err(|error| format!("{error:#}").into())
        }),
    ];

    #[cfg(not(target_os = "macos"))]
    let tests = Vec::<Trial>::new();

    libtest_mimic::run(&args, tests).exit();
}

#[cfg(target_os = "macos")]
mod macos {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;
    use std::time::Duration;

    use anyhow::anyhow;
    use dispatch2::run_on_main;
    use instant::Instant;
    use warpui::r#async::Timer;
    use warpui::platform::TerminationMode;
    use warpui::platform::app::{AppBuilder, AppCallbacks};

    pub(super) fn native_ime_ranges_and_delayed_close() -> anyhow::Result<()> {
        use objc2::msg_send;
        use objc2::rc::{Allocated, Retained, autoreleasepool};
        use objc2::runtime::{AnyClass, AnyObject, Bool};
        use objc2_foundation::{NSPoint, NSRange, NSRect, NSSize, NSString};

        objc2::MainThreadMarker::new().expect("IME client runs on the process main thread");
        let drain_main_queue = || {
            let completed = Arc::new(AtomicBool::new(false));
            dispatch2::DispatchQueue::main().exec_async({
                let completed = completed.clone();
                move || completed.store(true, Ordering::SeqCst)
            });
            let deadline = Instant::now() + Duration::from_secs(2);
            while !completed.load(Ordering::SeqCst) && Instant::now() < deadline {
                core_foundation::runloop::CFRunLoop::run_in_mode(
                    unsafe { core_foundation::runloop::kCFRunLoopDefaultMode },
                    Duration::from_millis(10),
                    false,
                );
            }
            assert!(completed.load(Ordering::SeqCst), "main queue must drain");
        };
        autoreleasepool(|_| {
            let class = AnyClass::get(c"WarpHostView").expect("native host view is linked");
            // SAFETY: this is the host view's designated initializer, in test mode with no device
            // or Rust window state. Marked-text methods stay local to this isolated view.
            let view: Retained<AnyObject> = unsafe {
                let allocated: Allocated<AnyObject> = msg_send![class, alloc];
                msg_send![allocated,
                    initWithFrame: NSRect::new(NSPoint::new(0., 0.), NSSize::new(800., 600.)),
                    metalDevice: std::ptr::null::<AnyObject>(),
                    enableTitlebarDrag: Bool::NO,
                    testMode: Bool::YES]
            };
            let text = NSString::from_str("a😀中");
            // SAFETY: these selectors implement NSTextInputClient; ranges use NSString UTF-16.
            unsafe {
                let _: () = msg_send![&*view, setMarkedText: &*text,
                    selectedRange: NSRange::new(3, 1), replacementRange: NSRange::new(0, 0)];
                let marked: NSRange = msg_send![&*view, markedRange];
                let selected: NSRange = msg_send![&*view, selectedRange];
                assert_eq!(marked, NSRange::new(0, 4));
                assert_eq!(selected, NSRange::new(3, 1));
                let _: () = msg_send![&*view, setMarkedText: &*text,
                    selectedRange: NSRange::new(usize::MAX, usize::MAX),
                    replacementRange: NSRange::new(0, 0)];
                let selected: NSRange = msg_send![&*view, selectedRange];
                assert_eq!(selected, NSRange::new(4, 0));
                let _: () = msg_send![&*view, unmarkText];
                let has_marked: Bool = msg_send![&*view, hasMarkedText];
                let selected: NSRange = msg_send![&*view, selectedRange];
                assert!(!has_marked.as_bool());
                assert_eq!(selected, NSRange::new(0, 0));

                let _: () = msg_send![&*view, closeIMEAsync];
                let _: () = msg_send![&*view, setMarkedText: &*text,
                    selectedRange: NSRange::new(3, 1), replacementRange: NSRange::new(0, 0)];
                drain_main_queue();
                let has_marked: Bool = msg_send![&*view, hasMarkedText];
                assert!(
                    has_marked.as_bool(),
                    "an old close must not cancel a new composition"
                );

                let _: () = msg_send![&*view, closeIMEAsync];
                let _: () = msg_send![&*view, setMarkedText: &*text,
                    selectedRange: NSRange::new(4, 0), replacementRange: NSRange::new(0, 0)];
                drain_main_queue();
                let has_marked: Bool = msg_send![&*view, hasMarkedText];
                assert!(
                    !has_marked.as_bool(),
                    "updates within the old composition must still close"
                );
            }
        });
        Ok(())
    }

    pub(super) fn services_main_dispatch_queue() -> anyhow::Result<()> {
        objc2::MainThreadMarker::new()
            .expect("the custom test harness must run on the process main thread");
        let process_main_thread = thread::current().id();
        let dispatched_on_main = Arc::new(AtomicBool::new(false));
        let worker_returned = Arc::new(AtomicBool::new(false));

        AppBuilder::new_headless(AppCallbacks::default(), Box::new(()), None).run({
            let dispatched_on_main = dispatched_on_main.clone();
            let worker_returned = worker_returned.clone();
            move |ctx| {
                thread::spawn({
                    let dispatched_on_main = dispatched_on_main.clone();
                    let worker_returned = worker_returned.clone();
                    move || {
                        run_on_main(|_| {
                            dispatched_on_main.store(
                                thread::current().id() == process_main_thread,
                                Ordering::SeqCst,
                            );
                        });
                        worker_returned.store(true, Ordering::SeqCst);
                    }
                });

                let weak_app = ctx.weak_app();
                ctx.foreground_executor()
                    .spawn(async move {
                        let deadline = Instant::now() + Duration::from_secs(5);
                        while Instant::now() < deadline
                            && !(dispatched_on_main.load(Ordering::SeqCst)
                                && worker_returned.load(Ordering::SeqCst))
                        {
                            Timer::after(Duration::from_millis(10)).await;
                        }

                        let test_result = if dispatched_on_main.load(Ordering::SeqCst)
                            && worker_returned.load(Ordering::SeqCst)
                        {
                            Ok(())
                        } else {
                            Err(anyhow!(
                                "headless Warp did not service the GCD main queue before timeout"
                            ))
                        };
                        if let Some(mut app) = weak_app.upgrade() {
                            app.update(|ctx| {
                                ctx.terminate_app(
                                    TerminationMode::ForceTerminate,
                                    Some(test_result),
                                );
                            });
                        }
                    })
                    .detach();
            }
        })
    }
}
