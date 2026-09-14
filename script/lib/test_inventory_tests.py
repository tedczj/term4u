#!/usr/bin/env python3
"""Exercise inventory failure handling without a Rust toolchain."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "test_inventory"


class TestInventory(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="term4u-inventory-test-")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        (self.root / "script").mkdir()
        shutil.copy2(SCRIPT, self.root / "script/test_inventory")
        self.base = self.root / "test-data/localization"
        self.base.mkdir(parents=True)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        cargo = self.bin / "cargo"
        cargo.write_text(
            '#!/bin/bash\n'
            'printf "%s" "$FAKE_NEXTEST_STDOUT"\n'
            'printf "%s" "$FAKE_NEXTEST_STDERR" >&2\n'
            'exit "$FAKE_NEXTEST_EXIT"\n'
        )
        cargo.chmod(0o755)
        self.env = {
            **os.environ,
            "PATH": f"{self.bin}:{os.environ['PATH']}",
            "FAKE_NEXTEST_STDOUT": json.dumps({
                "rust-suites": {"warp": {"testcases": {"local_behavior": {}}}}
            }),
            "FAKE_NEXTEST_STDERR": "",
            "FAKE_NEXTEST_EXIT": "0",
        }

    def run_inventory(self, *args):
        return subprocess.run(
            [str(self.root / "script/test_inventory"), *args],
            env=self.env,
            text=True,
            capture_output=True,
            check=False,
        )

    def test_list_returns_actual_test_ids(self):
        result = self.run_inventory("list")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "warp::local_behavior\n")

    def test_cargo_failure_is_not_an_empty_inventory(self):
        self.env.update(FAKE_NEXTEST_EXIT="42", FAKE_NEXTEST_STDERR="compile failed")
        result = self.run_inventory("list")
        self.assertEqual(result.returncode, 42)
        self.assertEqual(result.stdout, "")
        self.assertIn("compile failed", result.stderr)

    def test_failed_snapshot_preserves_existing_baseline(self):
        target = self.base / "existing.txt"
        target.write_text("original::test\n")
        self.env["FAKE_NEXTEST_EXIT"] = "42"
        result = self.run_inventory("snapshot", "existing")
        self.assertEqual(result.returncode, 42)
        self.assertEqual(target.read_text(), "original::test\n")
        self.assertEqual(list(self.base.glob(".test-inventory.*")), [])

    def test_failed_snapshot_does_not_create_empty_baseline(self):
        self.env["FAKE_NEXTEST_EXIT"] = "42"
        result = self.run_inventory("snapshot", "new")
        self.assertEqual(result.returncode, 42)
        self.assertFalse((self.base / "new.txt").exists())

    def test_successful_snapshot_contains_test_ids(self):
        result = self.run_inventory("snapshot", "new")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.base / "new.txt").read_text(), "warp::local_behavior\n")

    def test_invalid_or_empty_json_never_publishes_a_snapshot(self):
        for value in ["not json", "{}", '{"rust-suites": {}}',
                      '{"rust-suites": {"warp": {"testcases": {}}}}']:
            with self.subTest(value=value):
                self.env["FAKE_NEXTEST_STDOUT"] = value
                result = self.run_inventory("snapshot", "new")
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("invalid nextest inventory", result.stderr)
                self.assertFalse((self.base / "new.txt").exists())

    def test_unapproved_disappearance_still_fails(self):
        (self.base / "phase1-before.txt").write_text("warp::local_behavior\nwarp::missing\n")
        (self.base / "deleted-test-ids.txt").write_text("")
        result = self.run_inventory("verify")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("warp::missing", result.stderr)

    def test_stale_deletion_allowlist_still_fails(self):
        (self.base / "phase1-before.txt").write_text("warp::local_behavior\n")
        (self.base / "deleted-test-ids.txt").write_text("warp::local_behavior\n")
        result = self.run_inventory("verify")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("did not disappear", result.stderr)

    def test_exact_disappearance_inventory_passes(self):
        (self.base / "phase1-before.txt").write_text("warp::local_behavior\nwarp::removed\n")
        (self.base / "deleted-test-ids.txt").write_text("warp::removed\n")
        result = self.run_inventory("verify")
        self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
