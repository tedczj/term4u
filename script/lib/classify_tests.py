#!/usr/bin/env python3
"""Run the redesign test classifier from its executable M0 location."""

import os
import runpy

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
runpy.run_path(
    os.path.join(ROOT, "docs", "redesign", "tools", "classify_tests.py"),
    run_name="__main__",
)
