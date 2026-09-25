import hashlib
import json
from pathlib import Path
import termios

import pyte

root = Path(__file__).parent
raw_path = root / 'raw/complete-cua18-pty.raw'
events_path = root / 'logs/complete-cua18-pty.jsonl'
raw = raw_path.read_bytes()
events = [json.loads(line) for line in events_path.read_text().splitlines()]
inputs = {e['label']: e for e in events if e['event'] == 'input_sent'}
started = next(e for e in events if e['event'] == 'process_started')
ended = next(e for e in events if e['event'] == 'process_exited')
tree = next(e for e in events if e['event'] == 'process_tree')
assert ended['exit_code'] == 0
before = started['termios_before']
after = ended['termios_after']
mode_delta = before['lflag'] ^ after['lflag']
assert mode_delta in (0, termios.PENDIN)
assert {k: v for k, v in before.items() if k != 'lflag'} == {k: v for k, v in after.items() if k != 'lflag'}
assert tree['root_pid'] == started['pid']
assert any('terminal-server' in p['command'] for p in tree['processes'])
assert any('zsh' in p['command'] for p in tree['processes'])
screen = pyte.Screen(120, 40)
stream = pyte.ByteStream(screen)
alt_start = inputs['alternate_screen_enter_exit']['raw_offset']
protocol_start = inputs['synthetic_osc_bel_host_passthrough_probe']['raw_offset']
protocol_end = inputs['start_interruptible_command']['raw_offset']
stream.feed(raw[:alt_start])
rows = [y for y, row in enumerate(screen.display) if row.rstrip() == '你好 X']
assert len(rows) == 1
row = rows[0]
assert [screen.buffer[row][x].data for x in (0, 2, 5)] == ['你', '好', 'X']
(root / 'raw/complete-cua18-chinese-frame.txt').write_text('\n'.join(screen.display) + '\n')
alt_observed = None
for offset in range(alt_start, protocol_start):
    stream.feed(raw[offset:offset + 1])
    # Inspect the ASCII marker without rendering transient wide-cell repaint fragments.
    line = ''.join(screen.buffer[1][x].data for x in range(120)).rstrip()
    if line == 'L0_ALT_SCREEN':
        alt_observed = offset + 1
        break
assert alt_observed is not None
stream.feed(raw[alt_observed:protocol_start])
assert any(row.rstrip() == '你好 X' for row in screen.display)
counts = {name: raw[protocol_start:protocol_end].count(sequence) for name, sequence in
          [('OSC52', b'\x1b]52;'), ('OSC9', b'\x1b]9;'), ('OSC777', b'\x1b]777;'), ('BEL', b'\x07')]}
assert all(value == 0 for value in counts.values())
receipt = json.loads((root / 'raw/tui-protocol-receipt.json').read_text())
assert receipt['response_hex'] == ''
assert receipt['ppid'] in {p['pid'] for p in tree['processes'] if 'zsh' in p['command']}
sent = bytes.fromhex(receipt['sent_hex'])
assert b'\x1b]52;c;?\x07' in sent
assert b'\x1b]777;notify;L0 title;L0 body\x07' in sent
result = {'reviewer': 'root', 'candidate_scope': 'round2 frozen TUI, complete isolated real PTY sample',
          'raw_sha256': hashlib.sha256(raw).hexdigest(), 'events_sha256': hashlib.sha256(events_path.read_bytes()).hexdigest(),
          'parser': 'pyte 0.8.2 / wcwidth 0.9.1', 'bounded_assertions': 'PASS',
          'chinese_output': {'row': row, 'columns': {'你': 0, '好': 2, 'X': 5}},
          'alternate_screen_visible_at_raw_offset': alt_observed, 'old_output_restored_before': protocol_start,
          'protocol_observation_interval': [protocol_start, protocol_end], 'emitted_control_sequence_counts': counts,
          'clipboard_query_response_bytes': 0, 'query_observation_seconds': 0.75,
          'protocol_probe': receipt, 'full_termios_restored_except_kernel_PENDIN': True, 'lflag_delta': mode_delta,
          'process_tree': tree['processes'], 'exit_code': ended['exit_code'],
          'limits': ['No GUI system-permission or OS IME acceptance inferred from this TUI byte test.',
                     'The check covers the captured output interval; not all protocol requests or resources.']}
(root / 'independent-complete-pty-review.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
print(json.dumps(result, ensure_ascii=False))
