# Local GUI shell corrections — 2026-09-13

This change reconnects local shell completion and prompt rendering to the simplified GUI.
Tab inserts a unique match, expands a common prefix, or displays candidates for cycling.
Completion results preserve text after the cursor and discard stale requests. The prompt
shows the session identity, directory and Git head using the terminal font; an enabled PS1
uses the shell's prompt grid.

`clear` now renders a viewport gap, measured against the rows actually rendered by the
local transcript, and scrolls after layout. The command issuing Clear stays before the
gap even when its header has not become visible. History remains available by scrolling.
Terminal mouse focus preserves dragged selections, returns a simple click to the input,
and resets cursor blinking on focus. The empty input editor fills the available width.

## Validation

- Focused nextest regression results: 12 passed, 0 failed (`tests.log`).
- `./script/format`: passed.
- Presubmit format, inline-test placement, license boundaries, license configuration,
  network boundaries and inventory regression checks: passed.
- Required workspace, GUI and completer Clippy commands: all passed (`presubmit.log`).
- The full presubmit stops at the C/C++ formatter because `clang-format` is not installed.
  Later WGSL, full-workspace nextest and doc-test stages are not claimed as passed.
- macOS local-only app build and bundle: passed (`build.log`); `codesign --verify --deep --strict` passed.
- Live GUI checks: `cd work` in the home directory completed to `cd workspace/`;
  `cd term4` completed to `cd term4u/`; candidates for `cd app/src/te` cycled through
  `terminal/` and `test_util/`. The Git prompt displayed `(main)` inside the repository
  and removed it outside the repository. Prompt typography matched terminal output.
- Live clear checks: old marker output disappeared after `clear`; `echo AFTERCLEAR`
  displayed normally; repeated clear worked; scrolling upward recovered old markers.
- Cursor focus is covered by the focused regression tests. Automated GUI screenshots
  did not reliably establish the blinking cursor's visible phase after a click;
  manual confirmation remains separate from the test results.

This is a scoped local GUI regression result, not full-project or network acceptance.
