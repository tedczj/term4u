import json, os, select, sys, termios, tty
from pathlib import Path
mode = sys.argv[1]
path = Path(__file__).with_name(mode + "-bytes.json")
fd = sys.stdin.fileno()
old = termios.tcgetattr(fd)
received = bytearray()
try:
    tty.setraw(fd)
    os.write(1, b"\x1b[?1049h\x1b[2J\x1b[H")
    if mode == "bracketed":
        os.write(1, b"\x1b[?2004h")
    else:
        os.write(1, b"\x1b[?2004l")
    os.write(1, ("L0 RAW PTY " + mode + " - paste/type; Ctrl-] finishes\r\n").encode())
    while True:
        data = os.read(fd, 4096)
        if b"\x1d" in data:
            received.extend(data.split(b"\x1d")[0])
            break
        received.extend(data)
        path.write_text(json.dumps({"hex": received.hex(), "text": received.decode("utf-8", "replace")}, ensure_ascii=False))
        os.write(1, ("\r\nRECEIVED " + repr(data) + "\r\n").encode())
finally:
    termios.tcsetattr(fd, termios.TCSANOW, old)
    os.write(1, b"\x1b[?2004l\x1b[?1049l")
    path.write_text(json.dumps({"hex": received.hex(), "text": received.decode("utf-8", "replace")}, ensure_ascii=False))
print("L0_CAPTURE", str(path))
