#!/usr/bin/env python3
"""CUA-18 real PTY capture for the frozen standalone/offline Term4u TUI."""
import datetime as dt
import fcntl
import json
import os
from pathlib import Path
import pty
import select
import signal
import struct
import subprocess
import termios
import time

root = Path('/Volumes/SN850X/term4u-l0-20260925/round2')
binary = root / 'term4u-tui'
cwd = root / 'fixtures' / 'final-tui-cwd'
zdotdir = root / 'fixtures' / 'final-zshdot'
cwd.mkdir(parents=True, exist_ok=True)
zdotdir.mkdir(parents=True, exist_ok=True)
(zdotdir / '.zshrc').write_text("PROMPT='L0TUI%# '\n", encoding='utf-8')
raw_path = root / 'raw' / 'final-cua18-pty.raw'
events_path = root / 'logs' / 'final-cua18-pty.jsonl'
profile = 'term4u-l0-20260925-round2-tui-final'
master, slave = pty.openpty()

def winsize(fd, rows, cols):
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack('HHHH', rows, cols, 0, 0))

def termios_summary(attrs):
    return {'iflag': attrs[0], 'oflag': attrs[1], 'cflag': attrs[2], 'lflag': attrs[3],
            'ispeed': attrs[4], 'ospeed': attrs[5],
            'cc': [int(value) if isinstance(value, int) else value.decode('latin1')
                   for value in attrs[6]]}

winsize(master, 40, 120)
winsize(slave, 40, 120)
before = termios.tcgetattr(slave)
with os.fdopen(os.open(raw_path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), 'wb') as raw:
    events = open(events_path, 'x', encoding='utf-8')
    def event(name, **detail):
        data = {'observed_utc': dt.datetime.now(dt.timezone.utc).isoformat(),
                'event': name, **detail}
        events.write(json.dumps(data, ensure_ascii=False) + '\n')
        events.flush()
    event('pty_created', rows=40, cols=120, profile=profile,
          argv=[str(binary)], cwd=str(cwd), zdotdir=str(zdotdir),
          history_file=str(root / 'fixtures' / 'final-zsh-history'))
    env = os.environ.copy()
    env.update({'WARP_DATA_PROFILE': profile, 'TERM': 'xterm-256color',
                'LANG': 'en_US.UTF-8', 'LC_ALL': 'en_US.UTF-8',
                'WARP_SHELL_PATH': '/bin/zsh', 'SHELL': '/bin/zsh',
                'ZDOTDIR': str(zdotdir), 'HISTFILE': str(root / 'fixtures' / 'final-zsh-history')})
    proc = subprocess.Popen([str(binary)], stdin=slave, stdout=slave, stderr=slave,
                            cwd=cwd, env=env, start_new_session=True, close_fds=True)
    event('process_started', pid=proc.pid, ppid=os.getpid(), termios_before=termios_summary(before))
    def collect(seconds):
        end = time.monotonic() + seconds
        while time.monotonic() < end:
            ready, _, _ = select.select([master], [], [], min(0.1, end-time.monotonic()))
            if ready:
                try:
                    chunk = os.read(master, 65536)
                except OSError:
                    return
                if not chunk:
                    return
                raw.write(chunk)
                raw.flush()
    collect(1.5)
    event('initial_zero_state_captured', raw_offset=raw.tell())
    os.write(master, b'\x14')
    event('input_sent', label='ctrl_t_create_terminal_tab', bytes_hex='14', raw_offset=raw.tell())
    collect(4.0)
    event('terminal_tab_start_observed', raw_offset=raw.tell())
    processes = []
    for row in subprocess.check_output(['ps', '-axo', 'pid=,ppid=,command='], text=True).splitlines():
        fields = row.strip().split(None, 2)
        if len(fields) == 3:
            processes.append({'pid': int(fields[0]), 'ppid': int(fields[1]), 'command': fields[2]})
    descendants = {proc.pid}
    while True:
        expanded = descendants | {p['pid'] for p in processes if p['ppid'] in descendants}
        if expanded == descendants:
            break
        descendants = expanded
    event('process_tree', root_pid=proc.pid,
          processes=[p for p in processes if p['pid'] in descendants], raw_offset=raw.tell())
    commands = [
        ('external_cli_ascii', b"python3 -c 'print(\"L0_CUA18_CLI_OK\")'"),
        ('utf8_chinese_input_and_output', "python3 -c 'print(\"你好 X\")'".encode('utf-8')),
        ('alternate_screen_enter_exit', b"python3 -c \"import sys,time;sys.stdout.write(chr(27)+'[?1049hL0_ALT_SCREEN');sys.stdout.flush();time.sleep(.2);sys.stdout.write(chr(27)+'[?1049l');sys.stdout.flush()\""),
        ('synthetic_osc_bel_host_passthrough_probe', b"python3 -c \"import sys;sys.stdout.write(chr(27)+']52;c;TE0gVExfQ0xJUCBURVNUIA=='+chr(7)+chr(27)+']9;L0_SYNTHETIC_NOTIFICATION'+chr(7)+chr(27)+']777;notify;L0_SYNTHETIC_NOTIFICATION'+chr(7)+chr(7))\""),
    ]
    for label, command in commands:
        offset = raw.tell()
        os.write(master, command + b'\r')
        event('input_sent', label=label, bytes_hex=(command + b'\r').hex(), raw_offset=offset)
        collect(2.0)
    offset = raw.tell()
    os.write(master, b'sleep 5\r')
    event('input_sent', label='start_interruptible_command', bytes_hex=b'sleep 5\r'.hex(), raw_offset=offset)
    collect(0.5)
    os.write(master, b'\x03')
    event('input_sent', label='ctrl_c', bytes_hex='03', raw_offset=raw.tell())
    collect(1.0)
    winsize(master, 30, 100)
    os.killpg(proc.pid, signal.SIGWINCH)
    event('resize_sent', rows=30, cols=100, raw_offset=raw.tell())
    collect(1.0)
    os.write(master, b'\x11')
    event('input_sent', label='ctrl_q_exit', bytes_hex='11', raw_offset=raw.tell())
    collect(2.0)
    try:
        code = proc.wait(timeout=2)
    except subprocess.TimeoutExpired:
        event('ctrl_q_exit_timeout', pid=proc.pid)
        proc.terminate()
        try:
            code = proc.wait(timeout=2)
        except subprocess.TimeoutExpired:
            proc.kill()
            code = proc.wait(timeout=2)
    after = termios.tcgetattr(slave)
    event('process_exited', exit_code=code, termios_after=termios_summary(after),
        termios_restored=(before == after))
    rows, cols, _, _ = struct.unpack('HHHH', fcntl.ioctl(master, termios.TIOCGWINSZ, b'\0'*8))
    event('pty_size_after', rows=rows, cols=cols)
    os.close(master)
    os.close(slave)
    events.close()
