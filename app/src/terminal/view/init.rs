use warpui::AppContext;
use warpui::keymap::FixedBinding;

use super::TerminalAction;
use crate::terminal::TerminalView;
use crate::util::bindings::is_binding_pty_compliant;

pub const CANCEL_COMMAND_KEYBINDING: &str = "terminal:cancel_command";
pub const INPUT_BOX_VISIBLE_KEY: &str = "InputVisible";
pub const KEYBOARD_PROTOCOL_ENABLED_KEY: &str = "KeyboardProtocolEnabled";

pub fn init(app: &mut AppContext) {
    use warpui::keymap::macros::*;

    app.register_binding_validator::<TerminalView>(is_binding_pty_compliant);
    app.register_fixed_bindings([
        FixedBinding::new("ctrl-c", TerminalAction::CtrlC, id!("Terminal")),
        FixedBinding::new("ctrl-d", TerminalAction::CtrlD, id!("Terminal")),
        FixedBinding::new("up", TerminalAction::Up, id!("Terminal") & !id!(INPUT_BOX_VISIBLE_KEY)),
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
        FixedBinding::standard(
            warpui::actions::StandardAction::Copy,
            TerminalAction::Copy,
            id!("Terminal"),
        ),
    ]);
}
