import base64
import json
import os
from pathlib import Path
import select
import sys
import termios
import time
import tty

out = Path(__file__).resolve().parents[1] / 'raw' / 'tui-protocol-receipt.json'
fd = sys.stdin.fileno()
before = termios.tcgetattr(fd)
payload = (b'\x1b]52;c;' + base64.b64encode(b'L0 synthetic clipboard') + b'\x07'
           b'\x1b]52;c;?\x07\x1b]9;L0 synthetic notification\x07'
           b'\x1b]777;notify;L0 title;L0 body\x07\x07')
received = bytearray()
try:
    tty.setraw(fd)
    os.write(sys.stdout.fileno(), payload)
    deadline = time.monotonic() + 0.75
    while time.monotonic() < deadline:
        if select.select([fd], [], [], 0.05)[0]:
            received.extend(os.read(fd, 4096))
    with out.open('x') as log:
        json.dump({'pid': os.getpid(), 'ppid': os.getppid(), 'tty': os.ttyname(fd),
                   'sent_hex': payload.hex(), 'response_hex': received.hex()}, log, indent=2)
        log.write('\n')
finally:
    termios.tcsetattr(fd, termios.TCSANOW, before)
print('L0_PROTOCOL_PROBE_DONE', flush=True)
