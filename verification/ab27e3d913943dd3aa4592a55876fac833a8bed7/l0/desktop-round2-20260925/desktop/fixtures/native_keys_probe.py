import datetime
import json
import os
from pathlib import Path
import select
import sys
import termios
import time
import tty

root = Path(__file__).resolve().parents[1]
output = root / 'raw' / 'native-key-bytes.jsonl'
fd = sys.stdin.fileno()
before = termios.tcgetattr(fd)
with output.open('x') as log:
    def record(**data):
        log.write(json.dumps({'utc': datetime.datetime.now(datetime.timezone.utc).isoformat(), **data}) + '\n')
        log.flush()
    record(event='started', pid=os.getpid(), ppid=os.getppid(), tty=os.ttyname(fd))
    try:
        tty.setraw(fd)
        os.write(sys.stdout.fileno(), b'\x1b[?1049h\x1b[2J\x1b[HNative key probe READY. Send Escape, Return, Up, then Ctrl-C.\r\n')
        deadline = time.monotonic() + 180
        while time.monotonic() < deadline:
            if not select.select([fd], [], [], 0.5)[0]:
                continue
            data = os.read(fd, 4096)
            if not data:
                break
            record(event='input', hex=data.hex())
            os.write(sys.stdout.fileno(), ('bytes=' + data.hex() + '\r\n').encode())
            if b'\x03' in data:
                break
    finally:
        os.write(sys.stdout.fileno(), b'\x1b[?1049l')
        termios.tcsetattr(fd, termios.TCSANOW, before)
        record(event='finished')
