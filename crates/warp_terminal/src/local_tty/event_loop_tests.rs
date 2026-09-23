use super::*;

fn response(listener: &ChannelEventListener, bytes: &'static [u8]) -> ClipboardResponse {
    ClipboardResponse {
        bytes: Cow::Borrowed(bytes),
        request: listener.reserve_clipboard_request().unwrap(),
    }
}

#[test]
fn l0_07_stale_clipboard_response_never_reaches_the_writer() {
    let listener = ChannelEventListener::new_for_test();
    let mut state = State::default();
    state
        .write_list
        .push_back(Writing::clipboard(response(&listener, b"secret")));
    state
        .write_list
        .push_back(Writing::new(Cow::Borrowed(b"user")));
    listener.invalidate_clipboard_requests();
    let mut output = Vec::new();
    write_pending(&mut state, &mut output, &listener, &mut true).unwrap();
    assert_eq!(output, b"user");
    assert!(!state.needs_write());
}

struct PartialWriter {
    bytes: Vec<u8>,
    written_once: bool,
}

impl Write for PartialWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.written_once {
            return Err(io::Error::from(ErrorKind::WouldBlock));
        }
        self.written_once = true;
        self.bytes.extend_from_slice(&bytes[..2]);
        Ok(2)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn l0_07_epoch_change_discards_the_remainder_of_a_partial_clipboard_write() {
    let listener = ChannelEventListener::new_for_test();
    let mut state = State::default();
    state
        .write_list
        .push_back(Writing::clipboard(response(&listener, b"abcd")));
    state
        .write_list
        .push_back(Writing::new(Cow::Borrowed(b"next")));
    let mut writer = PartialWriter {
        bytes: Vec::new(),
        written_once: false,
    };
    let mut can_write = true;
    write_pending(&mut state, &mut writer, &listener, &mut can_write).unwrap();
    assert_eq!(writer.bytes, b"ab");
    assert!(!can_write);
    listener.set_clipboard_session(1);
    listener.set_clipboard_session(2);
    write_pending(&mut state, &mut writer.bytes, &listener, &mut true).unwrap();
    assert_eq!(writer.bytes, b"abnext");
    assert!(!state.needs_write());
}

#[test]
fn l0_07_clipboard_reply_order_and_debug_redaction_are_preserved() {
    let listener = ChannelEventListener::new_for_test();
    let reply = response(&listener, b"private-content");
    assert!(
        !format!("{:?}", Message::ClipboardResponse(reply.clone())).contains("private-content")
    );
    let mut state = State::default();
    state
        .write_list
        .push_back(Writing::new(Cow::Borrowed(b"before")));
    state.write_list.push_back(Writing::clipboard(reply));
    state
        .write_list
        .push_back(Writing::new(Cow::Borrowed(b"after")));
    let mut output = Vec::new();
    write_pending(&mut state, &mut output, &listener, &mut true).unwrap();
    assert_eq!(output, b"beforeprivate-contentafter");
}
