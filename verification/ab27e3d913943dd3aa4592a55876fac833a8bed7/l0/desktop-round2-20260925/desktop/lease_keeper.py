#!/usr/bin/env python3
"""Auditable long-lived child of script/l0_verify.py serial."""
import datetime as dt
import fcntl
import json
import os
from pathlib import Path
import sys

root = Path(sys.argv[1])
log = root / 'logs' / 'desktop-lease.jsonl'
lock = Path('/tmp') / f'term4u-l0-desktop-{os.getuid()}.lock'
lock_stat = lock.stat()
lock_fd = None
for fd in range(3, 256):
    try:
        st = os.fstat(fd)
    except OSError:
        continue
    if (st.st_dev, st.st_ino) == (lock_stat.st_dev, lock_stat.st_ino):
        lock_fd = fd
        break
if lock_fd is None:
    raise SystemExit('l0_verify lease descriptor was not inherited')


def record(event, detail=None):
    fcntl.flock(lock_fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
    data = {
        'observed_utc': dt.datetime.now(dt.timezone.utc).isoformat(),
        'event': event,
        'pid': os.getpid(),
        'parent_pid': os.getppid(),
        'lease_fd': lock_fd,
        'lease_dev': lock_stat.st_dev,
        'lease_inode': lock_stat.st_ino,
        'desktop_lock_held': True,
        'detail': detail or {},
    }
    with log.open('a', encoding='utf-8') as out:
        out.write(json.dumps(data, ensure_ascii=False) + '\n')
    print(json.dumps(data, ensure_ascii=False), flush=True)

record('LEASE_STARTED', {'protocol': 'CUA_BEGIN/CUA_END then END_SESSION'})
for raw in sys.stdin:
    line = raw.strip()
    if not line:
        continue
    if line == 'END_SESSION':
        record('LEASE_ENDED', {'handshake': 'END_SESSION'})
        break
    try:
        request = json.loads(line)
    except json.JSONDecodeError:
        print('Expected a JSON object or END_SESSION', flush=True)
        continue
    event = request.pop('event', None)
    if event not in ('CUA_BEGIN', 'CUA_END'):
        print('Expected CUA_BEGIN or CUA_END', flush=True)
        continue
    record(event, request)
