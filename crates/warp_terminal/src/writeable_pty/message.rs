use std::borrow::Cow;

use crate::SizeInfo;
use crate::event_listener::ClipboardRequest;

/// Messages that may be sent to the `EventLoop`.
#[derive(Debug)]
pub enum Message {
    /// Data that should be written to the PTY.
    Input(Cow<'static, [u8]>),
    ClipboardResponse(ClipboardResponse),

    /// Indicates that the `EventLoop` should be shut down.
    Shutdown,

    /// Indicates that the child process has exited.
    ///
    /// Only used on Windows, as we need to pass this information to the
    /// event loop via the channel (and cannot use the child event token).
    #[cfg_attr(not(windows), allow(dead_code))]
    ChildExited,

    /// Instruction to resize the PTY.
    Resize(SizeInfo),
}

/// An authorized clipboard reply, revalidated at the final PTY write boundary.
#[derive(Clone)]
pub struct ClipboardResponse {
    pub bytes: Cow<'static, [u8]>,
    pub request: ClipboardRequest,
}

impl std::fmt::Debug for ClipboardResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ClipboardResponse(<redacted>)")
    }
}
