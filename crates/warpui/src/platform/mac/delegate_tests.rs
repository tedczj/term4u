use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use objc2::rc::Retained;
use objc2_foundation::NSString;

use super::*;

struct CallbackDrop(Arc<AtomicUsize>);
impl Drop for CallbackDrop {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn l0_07_notification_completion_releases_callback_on_success_and_error() {
    let calls = Arc::new(AtomicUsize::new(0));
    let drops = Arc::new(AtomicUsize::new(0));
    for outcome in [2, 0, 3, 1] {
        let tracker = CallbackDrop(drops.clone());
        let calls = calls.clone();
        let callback: NotificationSendErrorCallback = Box::new(move |error| {
            drop(tracker);
            assert!(matches!(
                (outcome, error),
                (0, NotificationSendError::PermissionsDenied)
                    | (3, NotificationSendError::PermissionsNotYetGranted)
                    | (1, NotificationSendError::Other { .. })
            ));
            calls.fetch_add(1, Ordering::SeqCst);
        });
        let message = NSString::from_str("");
        // SAFETY: this transfers the exact boxed callback ownership used by sendNotification;
        // the NSString remains alive until the synchronous completion returns.
        unsafe {
            warp_on_notification_send_completed(
                outcome,
                Retained::as_ptr(&message).cast_mut().cast(),
                Box::into_raw(Box::new(callback)).cast(),
            );
        }
    }
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert_eq!(drops.load(Ordering::SeqCst), 4);
}
