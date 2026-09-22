# AGENTS.md

Engineering guidance for this repository. Product requirements, architecture, supported scope, current
status, implementation order and acceptance criteria live only in [docs/DESIGN.md](docs/DESIGN.md).
Update that document directly when the design or completion state changes. Do not create a second
specification, roadmap, handoff, TODO, status report or historical design directory.

## Scope and authorization

Follow the design's local-only, data-preservation, platform and license boundaries.
The current product target is macOS on Apple Silicon (`aarch64-apple-darwin`) only, for both GUI and
TUI. Do not add Intel/Rosetta/universal or other-platform compatibility. Existing surplus compatibility
is deferred cleanup under DESIGN §3.3; do not delete shared macOS code or data migrations by name. Repository history
and inherited skills do not authorize restoring retired cloud APIs, Agent interfaces or integration
harnesses. Use only the applicable local engineering techniques from those skills; do not follow their
old instructions to create parallel specs or call removed services.

Preserve existing user changes, runtime data, credentials, migrations and resources. Never reset a
worktree or remove a user's `.pi/` files merely to obtain a clean status. Test with isolated copies.
Commit, push, PR, merge, tag, release, system installation and privileged capture require authorization
for the specific operation. A requested PR does not authorize merging it or publishing a release.

## Development entry points

- `./script/run` runs the GUI; `./script/run-tui` runs the TUI.
- `./script/format` and `./script/format --check` format/check Rust and configured workspace files.
- `./script/presubmit` runs the repository engineering checks.
- `./script/test_inventory` checks actual test IDs against `test-data/localization/` inputs.
- `python3 script/lib/test_inventory_tests.py` tests inventory failure handling without Cargo;
  it does not replace the actual inventory check.
- `python3 script/lib/classify_tests.py --detail` is a heuristic audit helper using
  `script/deletion_set.txt`, not a compiler result or acceptance verdict.

Use the exact additional build, feature, runtime and network matrix in
[the design](docs/DESIGN.md#verification). Do not replace supported configurations with `--all-features`.
Restore missing tools without deleting gates; do not upgrade the pinned toolchain without authorization.
Do not infer a current PASS from an earlier source revision or an interrupted presubmit run.

## Coding style

- Prefer inference over unnecessary type annotations; use imports rather than long Rust qualifiers.
  Imports normally go at the top; cfg-specific branches may use scoped imports or qualified names.
- Name a context parameter `ctx` and put it last, except that a closure parameter should be last.
- Remove unused parameters and update callers rather than adding underscore prefixes.
- Prefer inline format arguments such as `println!("{message}")`.
- Do not pass `Itertools::format` directly to logging macros that may format more than once.
  Materialize a reusable string such as `iter.join(", ")`.
- For a toggleable local setting, keep the Command Palette entries, context flags and Settings UI
  consistent. Do not reintroduce cloud toggles through the generic setting mechanism.
- Prefer exhaustive matches to wildcard arms so newly introduced variants require explicit handling.

## Comments and documentation

Comments explain non-obvious rationale, not line-by-line behavior or a narration of the current patch.
Keep public doc comments concise; document each property once at the appropriate declaration.
Container comments describe shared behavior rather than repeating every field or enum variant.
Do not enumerate callers in function comments. Do not remove unrelated comments during a change.
Flow comments to the repository's configured 100-column maximum rather than wrapping unusually early.
Architecture/status changes belong in the single design, not duplicated prose across source files.

## Entities, UI and terminal locking

Use the existing App/Entity/Handle and context patterns. Keep GUI MouseStateHandle instances stable
across renders; do not allocate a replacement during every render. TUI and GUI use different rendering
and input systems; choose front-end-appropriate verification techniques.

Be especially careful with `TerminalModel::lock()`. Before changing a call chain, verify whether the
caller already holds the lock. Prefer passing an already-locked model reference; keep lock scope short
and do not call back into code that reacquires it. A compiling change can still deadlock the UI.

Use real macOS display validation for GUI behavior and a real interactive PTY for TUI behavior.
Render-to-lines tests supplement the TUI run; screenshots or GUI tests cannot replace it. Never restore
the retired integration harness just because an inherited skill mentions it.

## Tests and persistence

Use `cargo nextest` for parallel test execution and retain all required presubmit checks.
Unit tests live in separate `${filename}_tests.rs` or `mod_test.rs` files and are included at the end
of the parent module:

```rust
#[cfg(test)]
#[path = "filename_tests.rs"]
mod tests;
```

Do not delete retained-behavior tests, add ignores/dead-code allowances, weaken assertions or broaden
allowlists merely to obtain green checks. Authorized retirement of old integration suites is not a
standing authorization for future removals. Preserve the immutable test-ID baseline; exact removals
and their reasons must match the approved deletion inventory.

SQLite uses Diesel. Historical migrations and raw legacy fields remain intact. Work on isolated
copies of the fixtures specified by the design; compare migration, write and restart behavior rather
than clearing tables or replacing the database.

## Feature flags

Use existing `warp_core/src/features.rs` machinery and a high-level local product boundary.
Where supported by that machinery, prefer runtime checks for an ordinary local UI feature and cfg
for platform or unavailable dependency boundaries. This preference must never weaken compile-time
`offline_hard` / `local_only` guarantees. Keep local UI and actions gated consistently and remove
obsolete flags/branches after the feature stabilizes.

## Pull requests

Always run `./script/format` and all Clippy configurations required by `script/presubmit` before
opening or updating a PR. They must pass completely; do not change this requirement to bypass a
missing tool or failure. Run the remaining applicable tests and the design's acceptance matrix,
reporting exactly what was and was not executed. A draft is not a passing verification result.

Use [.github/pull_request_template.md](.github/pull_request_template.md). Reference design sections
and acceptance IDs instead of creating another spec. Include UI evidence where applicable; do not
claim manual validation merely because a binary built. Use `CHANGELOG-NONE` for documentation/internal
tooling changes, or the appropriate existing changelog entry for user-visible changes.

Do not disclose non-public security vulnerabilities, credentials or raw system-wide capture data in
public PRs/issues. Use a verified private reporting channel rather than an inherited upstream address.
