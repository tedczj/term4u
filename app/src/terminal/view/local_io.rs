//! Local input and host-clipboard boundaries. Payloads must never be logged here.

use std::path::PathBuf;
use std::time::Duration;

use instant::Instant;
use warpui::clipboard::ClipboardContent;
use warpui::modals::{AlertDialogWithCallbacks, ModalButton};
use warpui::{SingletonEntity, ViewContext};

use super::TerminalView;
use crate::terminal::model::block::BlockId;
use crate::terminal::model::escape_sequences::{BRACKETED_PASTE_END, BRACKETED_PASTE_START};
use crate::terminal::model::session::SessionId;
use crate::terminal::settings::TerminalSettings;
use crate::terminal::{AudibleBell, ClipboardType};
use crate::view_components::DismissibleToast;
use crate::workspace::ToastStack;

pub(super) const MAX_LOCAL_TRANSFER_BYTES: usize = 1024 * 1024;
const BELL_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Default)]
pub(super) struct LocalIoState {
    pub pending_paste: Option<PendingPaste>,
    next_request: u64,
    last_bell: Option<Instant>,
}

pub(super) struct PendingPaste {
    pub request: u64,
    session_id: Option<SessionId>,
    block_id: BlockId,
    bytes: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum PastePlan {
    Empty,
    Editor(String),
    Native(Vec<u8>),
    Confirm(Vec<u8>),
    Reject(&'static str),
}

/// Native paste is one PTY write, including the bracket delimiters. Control characters are
/// rejected rather than rewritten: stripping ESC could turn a displayed payload into a command.
pub(super) fn plan_paste(text: String, editor: bool, bracketed: bool) -> PastePlan {
    if text.is_empty() {
        return PastePlan::Empty;
    }
    if text.len() > MAX_LOCAL_TRANSFER_BYTES {
        return PastePlan::Reject("Paste exceeds the 1 MiB local transfer limit.");
    }
    if editor {
        return PastePlan::Editor(text);
    }
    if text
        .chars()
        .any(|c| c.is_control() && !matches!(c, '\t' | '\r' | '\n'))
    {
        return PastePlan::Reject("Paste contains terminal control characters and was not sent.");
    }
    if !bracketed {
        return if text.contains(['\r', '\n']) {
            PastePlan::Confirm(text.into_bytes())
        } else {
            PastePlan::Native(text.into_bytes())
        };
    }
    let mut bytes =
        Vec::with_capacity(text.len() + BRACKETED_PASTE_START.len() + BRACKETED_PASTE_END.len());
    bytes.extend_from_slice(BRACKETED_PASTE_START);
    bytes.extend_from_slice(text.as_bytes());
    bytes.extend_from_slice(BRACKETED_PASTE_END);
    PastePlan::Native(bytes)
}

impl TerminalView {
    pub(super) fn invalidate_pending_paste(&mut self) {
        self.local_io.pending_paste = None;
    }

    pub(super) fn paste_text(&mut self, text: String, ctx: &mut ViewContext<Self>) {
        self.invalidate_pending_paste();
        let editor = self.input_is_visible();
        let bracketed = !editor && self.model.lock().needs_bracketed_paste();
        match plan_paste(text, editor, bracketed) {
            PastePlan::Empty => {}
            PastePlan::Editor(text) => {
                self.input.update(ctx, |input, ctx| {
                    input.insert_text(&text, ctx);
                    input.focus_input_box(ctx);
                });
            }
            PastePlan::Native(bytes) => self.write_bytes(bytes, ctx),
            PastePlan::Reject(reason) => self.show_local_io_warning(reason, ctx),
            PastePlan::Confirm(bytes) => {
                self.local_io.next_request += 1;
                let request = self.local_io.next_request;
                self.local_io.pending_paste = Some(PendingPaste {
                    request,
                    session_id: self.model_events.as_ref(ctx).active_session_id(),
                    block_id: self.model.lock().block_list().active_block().id().clone(),
                    bytes,
                });
                // Cancel is first so Return cannot accidentally approve an executable paste.
                ctx.show_native_platform_modal(AlertDialogWithCallbacks::for_view(
                    "Paste multiple lines into the running program?",
                    "This program has not enabled bracketed paste. Newlines may execute commands. No text has been sent yet.",
                    vec![
                        ModalButton::for_view("Cancel", move |view: &mut Self, ctx| {
                            view.finish_pending_paste(request, false, ctx);
                        }),
                        ModalButton::for_view("Paste", move |view: &mut Self, ctx| {
                            view.finish_pending_paste(request, true, ctx);
                        }),
                    ],
                    |view, _| view.invalidate_pending_paste(),
                ));
            }
        }
    }

    pub(super) fn finish_pending_paste(
        &mut self,
        request: u64,
        accepted: bool,
        ctx: &mut ViewContext<Self>,
    ) {
        if self
            .local_io
            .pending_paste
            .as_ref()
            .is_none_or(|pending| pending.request != request)
        {
            return;
        }
        let pending = self.local_io.pending_paste.take().unwrap();
        if !accepted || self.input_is_visible() || !ctx.is_self_focused() {
            return;
        }
        let is_same_target = pending.session_id
            == self.model_events.as_ref(ctx).active_session_id()
            && pending.block_id == *self.model.lock().block_list().active_block().id();
        // Read under the lock, but emit only after releasing it.
        let bracketed = self.model.lock().needs_bracketed_paste();
        if is_same_target && !bracketed {
            self.write_bytes(pending.bytes, ctx);
        }
    }

    pub(super) fn drop_paths(&mut self, paths: &[PathBuf], ctx: &mut ViewContext<Self>) {
        self.file_drop_active = false;
        ctx.notify();
        let shell = self.shell_family(ctx);
        let mut text = String::new();
        for path in paths {
            let Some(path) = path.to_str() else {
                self.show_local_io_warning(
                    "The dropped path is not valid UTF-8; nothing was inserted.",
                    ctx,
                );
                return;
            };
            if path.len() > MAX_LOCAL_TRANSFER_BYTES {
                self.show_local_io_warning(
                    "Dropped paths exceed the 1 MiB local transfer limit.",
                    ctx,
                );
                return;
            }
            let escaped = shell.escape(path);
            let separator = usize::from(!text.is_empty());
            if text.len() + separator + escaped.len() > MAX_LOCAL_TRANSFER_BYTES {
                self.show_local_io_warning(
                    "Dropped paths exceed the 1 MiB local transfer limit.",
                    ctx,
                );
                return;
            }
            if separator != 0 {
                text.push(' ');
            }
            text.push_str(&escaped);
        }
        self.paste_text(text, ctx);
    }

    pub(super) fn store_terminal_clipboard(
        &self,
        selection: ClipboardType,
        text: &str,
        ctx: &mut ViewContext<Self>,
    ) {
        let allowed = selection == ClipboardType::Clipboard
            && TerminalSettings::as_ref(ctx)
                .osc52_clipboard_access
                .allows_write()
            && text.len() <= MAX_LOCAL_TRANSFER_BYTES;
        if allowed {
            ctx.clipboard()
                .write(ClipboardContent::plain_text(text.to_owned()));
        }
        log::debug!("l0_id=L0-07 route=osc52_write allowed={allowed}");
    }

    pub(super) fn load_terminal_clipboard(
        &self,
        selection: ClipboardType,
        encode: &(dyn Fn(&str) -> String + Send + Sync),
        ctx: &mut ViewContext<Self>,
    ) {
        if selection != ClipboardType::Clipboard
            || !TerminalSettings::as_ref(ctx)
                .osc52_clipboard_access
                .allows_read()
        {
            log::debug!("l0_id=L0-07 route=osc52_read outcome=denied");
            return;
        }
        let text = ctx.clipboard().read().plain_text;
        if text.len() > MAX_LOCAL_TRANSFER_BYTES {
            log::debug!("l0_id=L0-07 route=osc52_read outcome=over_limit");
            return;
        }
        // The parser supplies the encoder and terminator. This is a protocol response, not paste.
        self.write_bytes(encode(&text).into_bytes(), ctx);
    }

    pub(super) fn ring_local_bell(&mut self, ctx: &mut ViewContext<Self>) {
        if !*TerminalSettings::as_ref(ctx).use_audible_bell
            || !ctx.has_singleton_model::<AudibleBell>()
        {
            return;
        }
        let now = Instant::now();
        if self
            .local_io
            .last_bell
            .is_some_and(|last| now.duration_since(last) < BELL_INTERVAL)
        {
            return;
        }
        self.local_io.last_bell = Some(now);
        if AudibleBell::as_ref(ctx).ring().is_err() {
            log::warn!("l0_id=L0-07 route=bell outcome=platform_error");
        }
    }

    fn show_local_io_warning(&self, reason: &'static str, ctx: &mut ViewContext<Self>) {
        let window_id = self.input.window_id(ctx);
        ToastStack::handle(ctx).update(ctx, |stack, ctx| {
            stack.add_ephemeral_toast(DismissibleToast::error(reason.to_owned()), window_id, ctx);
        });
    }
}

#[cfg(test)]
#[path = "local_io_tests.rs"]
mod tests;
