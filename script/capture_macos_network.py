#!/usr/bin/env python3
"""Collect bounded macOS network and process evidence for a separate user-run probe."""

import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import signal
import socket
import subprocess
import sys
import tempfile
import time


def utc_now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--duration", type=int, default=900, help="Maximum capture seconds (30–1800)")
    args = parser.parse_args()
    if platform.system() != "Darwin":
        parser.error("This collector supports macOS only")
    if os.geteuid() != 0 or not os.environ.get("SUDO_UID"):
        parser.error("Run with sudo from your terminal; applications must run separately without sudo")
    if not 30 <= args.duration <= 1800:
        parser.error("--duration must be between 30 and 1800 seconds")

    owner = int(os.environ["SUDO_UID"])
    group = int(os.environ["SUDO_GID"])
    os.umask(0o077)
    directory = Path(tempfile.mkdtemp(prefix="term4u-network-", dir="/private/tmp"))
    os.chown(directory, owner, group)
    print(f"Evidence directory: {directory}", flush=True)
    print("Raw logs stay local and may include other processes. Do not publish them unfiltered.", flush=True)
    commands = {
        "packets": ["/usr/sbin/tcpdump", "-i", "pktap,all", "-nn", "-k", "INPD", "-l", "tcp or udp"],
        "dns": ["/usr/bin/log", "stream", "--level", "debug", "--style", "compact",
                "--predicate", 'process == "mDNSResponder"'],
        "processes": ["/usr/bin/eslogger", "exec", "fork", "exit"],
    }
    children = {}
    handles = {}
    stopping = False
    report = {"started": utc_now(), "status": "STARTING", "commands": commands}

    def stop(signum, frame):
        nonlocal stopping
        stopping = True

    def write_json(name, value):
        path = directory / name
        path.write_text(json.dumps(value, indent=2) + "\n")
        os.chown(path, owner, group)

    signal.signal(signal.SIGINT, stop)
    signal.signal(signal.SIGTERM, stop)
    try:
        for name, command in commands.items():
            path = directory / (name + ".log")
            handles[name] = path.open("w")
            os.chown(path, owner, group)
            children[name] = subprocess.Popen(command, stdout=handles[name], stderr=subprocess.STDOUT)
        time.sleep(2)
        exited = [name for name, child in children.items() if child.poll() is not None]
        if exited:
            raise RuntimeError("Collectors exited: " + ", ".join(exited))
        if "listening on" not in (directory / "packets.log").read_text():
            raise RuntimeError("tcpdump did not confirm that capture started")

        probe = subprocess.Popen(["/usr/bin/true"])
        probe.wait()
        with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as sock:
            sock.sendto(b"TERM4U_NETWORK_CALIBRATION", ("127.0.0.1", 49331))
        dns_probe = subprocess.Popen([
            "/usr/bin/python3", "-c",
            "import socket,os\ntry: socket.getaddrinfo('term4u-calibration-%s.example.com'%os.getpid(),443)"
            "\nexcept OSError: pass",
        ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        try:
            dns_probe.wait(timeout=10)
        except subprocess.TimeoutExpired:
            dns_probe.kill()
            dns_probe.wait()
            raise RuntimeError("System DNS calibration timed out")
        time.sleep(2)
        events = (directory / "processes.log").read_text()
        if not re.search(r'"pid"\s*:\s*' + str(probe.pid) + r'\b', events):
            raise RuntimeError("Endpoint Security did not record the controlled exec PID")
        packets = (directory / "packets.log").read_text()
        if not re.search(r'proc [^,:]+:' + str(os.getpid()) + r'[,)]', packets):
            raise RuntimeError("Packet capture did not attribute the controlled UDP probe")
        dns = (directory / "dns.log").read_text()
        if not re.search(r'client pid: ' + str(dns_probe.pid) + r'\b', dns):
            raise RuntimeError("System DNS log did not attribute the controlled resolver request")
        report["calibration"] = {"exec_probe_pid": probe.pid, "udp_probe_pid": os.getpid(),
                                 "udp_port": 49331, "dns_probe_pid": dns_probe.pid}
        report["status"] = "CAPTURING"
        report["ready"] = utc_now()
        write_json("ready.json", report)
        print("READY: start the GUI/TUI probes as your normal user.", flush=True)
        print(f"Stop: touch {directory}/STOP  (or press Ctrl-C here)", flush=True)
        deadline = time.monotonic() + args.duration
        while not stopping and time.monotonic() < deadline and not (directory / "STOP").exists():
            exited = [name for name, child in children.items() if child.poll() is not None]
            if exited:
                raise RuntimeError("Collectors stopped during observation: " + ", ".join(exited))
            time.sleep(0.5)
        report["status"] = "COLLECTED_NOT_YET_VERIFIED"
    except (OSError, RuntimeError) as error:
        report["status"] = "BLOCKED"
        report["error"] = str(error)
        print(f"BLOCKED: {error}", file=sys.stderr)
    finally:
        report["exit_codes"] = {}
        for name, child in children.items():
            if child.poll() is None:
                child.terminate()
            try:
                child.wait(timeout=5)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait()
                report["status"] = "BLOCKED"
                report["error"] = "A collector required SIGKILL; capture is not verified"
            report["exit_codes"][name] = child.returncode
            handles[name].close()
        report["finished"] = utc_now()
        report["sha256"] = {path.name: hashlib.sha256(path.read_bytes()).hexdigest()
                            for path in directory.glob("*.log")}
        write_json("result.json", report)
        process_log = directory / "processes.log"
        if process_log.exists() and "ES_NEW_CLIENT_RESULT_ERR_NOT_PERMITTED" in process_log.read_text():
            print("Enable Full Disk Access for the terminal running this command in System Settings > "
                  "Privacy & Security, restart that terminal, then rerun. sudo alone cannot grant TCC access.",
                  file=sys.stderr)
        print(f"Finished: {report['status']}; evidence: {directory}", flush=True)
    return 1 if report["status"] == "BLOCKED" else 0


if __name__ == "__main__":
    sys.exit(main())
