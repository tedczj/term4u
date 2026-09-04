use std::borrow::Cow;

use warp::tui_export::{PtyIntent, PtyIntentEvent as _};

use super::TuiTerminalSessionEvent;

#[test]
fn user_input_is_forwarded_to_the_local_pty() {
    let event = TuiTerminalSessionEvent::WriteUserInput(Cow::Borrowed(b"printf local\r"));

    let Some(PtyIntent::WriteBytes(bytes)) = event.pty_intent() else {
        panic!("user input should become a PTY byte write");
    };
    assert_eq!(&*bytes, b"printf local\r");
}

#[test]
fn ctrl_c_is_forwarded_to_the_local_pty() {
    let event = TuiTerminalSessionEvent::WriteUserInput(Cow::Borrowed(b"\x03"));

    let Some(PtyIntent::WriteBytes(bytes)) = event.pty_intent() else {
        panic!("ctrl-c should become a PTY byte write");
    };
    assert_eq!(&*bytes, b"\x03");
}
