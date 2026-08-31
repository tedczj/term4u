use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};

#[test]
fn guard_allows_unix_socket() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("offline-guard.sock");
    let listener = UnixListener::bind(&path).unwrap();

    let sender = std::thread::spawn(move || {
        let mut stream = UnixStream::connect(path).unwrap();
        stream.write_all(b"local").unwrap();
    });
    let (mut stream, _) = listener.accept().unwrap();
    let mut message = String::new();
    stream.read_to_string(&mut message).unwrap();

    sender.join().unwrap();
    assert_eq!(message, "local");
}
