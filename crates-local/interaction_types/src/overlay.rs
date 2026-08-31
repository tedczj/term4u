//! Action overlay model and `.ass` subtitle generation for burned-in
//! recording annotations. The types are used by the app layer (to collect a
//! per-recording action log) on every platform; `.ass` generation is only built
//! where the burn-in re-encode runs (Linux) or under test.

use std::time::Duration;

use crate::{Action, Key, MouseButton, ScrollDirection, TargetedAction, Vector2I};

/// A group of semantic actions dispatched in one `UseComputer` call.
///
/// One entry represents one *successful* `UseComputer` call: `offset` is when
/// the client began executing the call's action sequence, and `finish_offset`
/// is when that complete sequence (including any explicit waits and the
/// requested post-action screenshot) returned. Failed or cancelled calls never
/// become entries.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ActionLogEntry {
    /// Time from when capture went live to when this group's `UseComputer` call
    /// began executing on the client.
    pub offset: Duration,
    /// Time from when capture went live to when this group's complete action
    /// sequence (and any post-action screenshot) finished.
    pub finish_offset: Duration,
    pub labels: Vec<String>,
    /// Resolved pointer events dispatched during this group, in capture-space
    /// pixels, used to burn in click ripples and drag trails. Empty on paths
    /// that record no pointer geometry.
    pub pointer_events: Vec<PointerEvent>,
}

/// A single resolved pointer event captured at dispatch time.
///
/// `point` is a capture-space pixel (full-screen capture: physical root/screen
/// pixels; window capture: window-local pixels) and `offset` is measured on the
/// same source/1x timeline as [`ActionLogEntry::offset`].
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PointerEvent {
    pub offset: Duration,
    pub kind: PointerEventKind,
    /// The button for a press/release; `None` for a move.
    pub button: Option<MouseButton>,
    pub point: Vector2I,
}

/// Which pointer primitive a [`PointerEvent`] represents.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PointerEventKind {
    Down,
    Move,
    Up,
    /// A pointer-position sample taken when a scroll warps the pointer before
    /// wheeling. It never participates in click/drag gesture classification;
    /// it only keeps the synthetic cursor (and a later release's coordinate)
    /// tracking the pointer.
    Scroll,
}

/// Returns true if a `UseComputer` action batch contains at least one real
/// interaction — any non-`Wait` action (keyboard, typing, pointer, or scroll)
/// or an explicit non-zero wait, whose settling time should be kept in the
/// recording. Only batches made entirely of `Wait(0)` no-ops (for example a
/// screenshot-only call) fail to qualify and are not committed to the
/// recording timeline. A pointer-only batch still qualifies (with empty
/// labels) so its on-screen effects are retained.
pub fn is_meaningful_action_group(actions: &[TargetedAction]) -> bool {
    actions.iter().any(|targeted| !targeted.action.is_no_op())
}

enum LabelCandidate {
    Key(Vec<Key>),
    Label(String),
}
/// Converts one `UseComputer` call into ordered, redaction-safe overlay labels.
///
/// Key down/up primitives are grouped until all pressed keys are released. Text
/// and scroll actions become semantic labels; pointer and meta actions are
/// omitted. The call-level summary preserves provider naming for a lone key
/// group, but structured actions reconstruct multi-action calls and always
/// determine printable-key redaction.
pub fn overlay_labels_for(actions: &[TargetedAction], action_summary: &str) -> Vec<String> {
    let candidates = collect_label_candidates(actions);
    let use_action_summary = matches!(candidates.as_slice(), [LabelCandidate::Key(_)]);

    candidates
        .into_iter()
        .map(|candidate| match candidate {
            LabelCandidate::Key(keys) => {
                key_label(&keys, use_action_summary.then_some(action_summary))
            }
            LabelCandidate::Label(label) => label,
        })
        .collect()
}

fn collect_label_candidates(actions: &[TargetedAction]) -> Vec<LabelCandidate> {
    let mut candidates = Vec::new();
    let mut current_keys = Vec::new();
    let mut pressed_keys = Vec::new();
    for targeted in actions {
        match &targeted.action {
            Action::KeyDown { key } => {
                if pressed_keys.is_empty() && !current_keys.is_empty() {
                    candidates.push(LabelCandidate::Key(std::mem::take(&mut current_keys)));
                }
                if !current_keys.contains(key) {
                    current_keys.push(key.clone());
                }
                pressed_keys.push(key.clone());
            }
            Action::KeyUp { key } => {
                if let Some(index) = pressed_keys.iter().position(|pressed| pressed == key) {
                    pressed_keys.remove(index);
                }
            }
            Action::TypeText { .. } => {
                flush_keys(&mut candidates, &mut current_keys, &mut pressed_keys);
                candidates.push(LabelCandidate::Label("typing\u{2026}".to_string()));
            }
            Action::MouseWheel { direction, .. } => {
                flush_keys(&mut candidates, &mut current_keys, &mut pressed_keys);
                candidates.push(LabelCandidate::Label(scroll_label(*direction).to_string()));
            }
            Action::Wait(_)
            | Action::MouseDown { .. }
            | Action::MouseUp { .. }
            | Action::MouseMove { .. } => {
                flush_keys(&mut candidates, &mut current_keys, &mut pressed_keys);
            }
        }
    }
    flush_keys(&mut candidates, &mut current_keys, &mut pressed_keys);
    candidates
}

fn key_label(keys: &[Key], action_summary: Option<&str>) -> String {
    if matches!(keys, [Key::Char(ch)] if !ch.is_control()) {
        return "typing\u{2026}".to_string();
    }

    let label = action_summary
        .map(key_label_from_summary)
        .unwrap_or_else(|| key_label_from_keys(keys));
    redact_printable_key(label)
}

fn flush_keys(
    candidates: &mut Vec<LabelCandidate>,
    current_keys: &mut Vec<Key>,
    pressed_keys: &mut Vec<Key>,
) {
    if !current_keys.is_empty() {
        candidates.push(LabelCandidate::Key(std::mem::take(current_keys)));
    }
    pressed_keys.clear();
}

fn redact_printable_key(label: String) -> String {
    let mut chars = label.chars();
    if chars.next().is_some_and(|ch| !ch.is_control()) && chars.next().is_none()
        || label.eq_ignore_ascii_case("space")
    {
        "typing\u{2026}".to_string()
    } else {
        label
    }
}

fn key_label_from_summary(summary: &str) -> String {
    summary
        .find('"')
        .zip(summary.rfind('"'))
        .filter(|(first, last)| last > first)
        .map(|(first, last)| summary[first + 1..last].to_string())
        .unwrap_or_else(|| {
            let trimmed = summary.trim();
            if trimmed.is_empty() {
                "key".to_string()
            } else {
                trimmed.to_string()
            }
        })
}

fn key_label_from_keys(keys: &[Key]) -> String {
    keys.iter()
        .map(|key| match key {
            Key::Char(ch) => ch.to_string(),
            Key::Keycode(keycode) => match *keycode as u32 {
                0xFF09 => "Tab",
                0xFF0D => "Return",
                0xFF1B => "Escape",
                0xFF51 => "Left",
                0xFF52 => "Up",
                0xFF53 => "Right",
                0xFF54 => "Down",
                0xFFE1 | 0xFFE2 => "shift",
                0xFFE3 | 0xFFE4 => "ctrl",
                0xFFE9 | 0xFFEA => "alt",
                0xFFEB | 0xFFEC => "super",
                _ => "key",
            }
            .to_string(),
        })
        .collect::<Vec<_>>()
        .join("+")
}

fn scroll_label(direction: ScrollDirection) -> &'static str {
    match direction {
        ScrollDirection::Up => "scroll \u{2191}",
        ScrollDirection::Down => "scroll \u{2193}",
        ScrollDirection::Left => "scroll \u{2190}",
        ScrollDirection::Right => "scroll \u{2192}",
    }
}
