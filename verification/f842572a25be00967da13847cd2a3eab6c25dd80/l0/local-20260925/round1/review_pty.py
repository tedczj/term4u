"""Reproduce the bounded assertions in independent-tui-review.json using pyte 0.8.2."""
import hashlib
import json
from pathlib import Path

import pyte

root = Path(__file__).parent
raw = (root / 'raw/cua18-pty-clean.raw').read_bytes()
assert hashlib.sha256(raw).hexdigest() == '1a8e24e734725c63bbb20f9f40948a7f32be43ed0a74077dd9dafb1152b7f67d'
screen = pyte.Screen(120, 40)
stream = pyte.ByteStream(screen)
stream.feed(raw[:4776])
assert screen.display[39].rstrip() == '你好 X'
assert [screen.buffer[39][x].data for x in (0, 2, 5)] == ['你', '好', 'X']
# Inspect complete observed frames; intermediate repaint bytes are not complete screens.
stream.feed(raw[4776:4983])
assert screen.display[1].rstrip() == 'L0_ALT_SCREEN'
stream.feed(raw[4983:16982])
assert any(row.rstrip() == '你好 X' for row in screen.display)
for forbidden in (b'\x1b]52;', b'\x1b]9;', b'\x1b]777;', b'\x07'):
    assert forbidden not in raw[16982:18737]
print(json.dumps({'bounded_pty_assertions': 'PASS', 'full_workflow': 'INCOMPLETE'}))
