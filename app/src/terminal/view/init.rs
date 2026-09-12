use warpui::AppContext;
use warpui::keymap::{EditableBinding, FixedBinding};

use super::TerminalAction;
use crate::terminal::TerminalView;
use crate::util::bindings::{CustomAction, is_binding_pty_compliant};

pub const CANCEL_COMMAND_KEYBINDING: &str = "terminal:cancel_command";
pub const INPUT_BOX_VISIBLE_KEY: &str = "InputVisible";
pub const KEYBOARD_PROTOCOL_ENABLED_KEY: &str = "KeyboardProtocolEnabled";

pub fn init(app: &mut AppContext) {
    use warpui::keymap::macros::*;

    app.register_binding_validator::<TerminalView>(is_binding_pty_compliant);
    app.register_editable_bindings([
        EditableBinding::new(
            "terminal:scroll_up_one_page",
            "Scroll Up One Page",
            TerminalAction::PageUp,
        )
        .with_context_predicate(id!("Terminal"))
        .with_key_binding("pageup"),
        EditableBinding::new(
            "terminal:scroll_down_one_page",
            "Scroll Down One Page",
            TerminalAction::PageDown,
        )
        .with_context_predicate(id!("Terminal"))
        .with_key_binding("pagedown"),
        EditableBinding::new("terminal:paste", "Paste", TerminalAction::Paste)
            .with_context_predicate(id!("Terminal"))
            .with_custom_action(CustomAction::Paste),
        EditableBinding::new("terminal:copy", "Copy", TerminalAction::Copy)
            .with_context_predicate(id!("Terminal") & id!("OutputSelected"))
            .with_custom_action(CustomAction::Copy),
    ]);
    app.register_fixed_bindings([
        FixedBinding::new("ctrl-c", TerminalAction::CtrlC, id!("Terminal")),
        FixedBinding::new(
            "up",
            TerminalAction::Up,
            id!("Terminal") & !id!(INPUT_BOX_VISIBLE_KEY),
        ),
        FixedBinding::new(
            "down",
            TerminalAction::Down,
            id!("Terminal") & !id!(INPUT_BOX_VISIBLE_KEY),
        ),
        FixedBinding::new(
            "left",
            TerminalAction::ControlSequence(b"\x1b[D".to_vec()),
            id!("Terminal") & !id!(INPUT_BOX_VISIBLE_KEY),
        ),
        FixedBinding::new(
            "right",
            TerminalAction::ControlSequence(b"\x1b[C".to_vec()),
            id!("Terminal") & !id!(INPUT_BOX_VISIBLE_KEY),
        ),
        FixedBinding::standard(
            warpui::actions::StandardAction::Paste,
            TerminalAction::Paste,
            id!("Terminal"),
        ),
    ]);
}
