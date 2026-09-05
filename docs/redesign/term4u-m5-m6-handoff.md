# Term4u M5/M6 handoff

## Objective
Continue implementing every M5/M6 requirement in `docs/redesign/10-M5-M6未完成项实施详设.md`, then run all prescribed verification, commit the finished implementation, and leave a clean worktree.

Do not restate the design in code or new planning documents; use the design and its linked phase documents as the source of truth.

## Git state
- Current HEAD: `948d475b wip: localize M5 and M6 runtime surfaces`
- `git status --porcelain` was empty immediately after that commit.
- Earlier completed checkpoints:
  - `d18d63a5 docs: detail remaining M5 and M6 implementation`
  - `94912b78 chore: freeze M5 and M6 implementation baseline`
  - `f828cd6b refactor: restrict language servers to local PATH`
  - `2fdeed6f refactor: reduce TUI to local terminal sessions`
- `948d475b` is explicitly a WIP checkpoint, not a verified implementation.

## Current condition
The WIP commit contains broad deletion of cloud authentication, Drive/cloud-object runtime, Warp Agent/blocklist surfaces, GraphQL/server clients, session sharing, and related crates, plus partial local replacements for auth, workflows, notebooks, env vars, terminal/TUI, workspace, pane, root view, settings, and persistence types.

The implementation is not known to compile. The last actual command run before further edits was:

```bash
cargo check -p warp --lib --no-default-features --features local_only
```

At that point it reported 224 error headers. Significant edits were made after that run (including workspace/root/URI and terminal model changes), so that count is stale. Rerun the command first and treat its fresh output as authoritative.

Important: the prior user requested that the current state be committed despite known failures. The original goal still requires the final implementation to pass tests and checks; do not treat the WIP commit as satisfying completion.

## Highest-priority continuation
1. Run the focused `cargo check` above and cluster errors by file.
2. Repair compilation in coherent order:
   - `app/src/persistence/sqlite.rs` and `app/src/persistence/block_list.rs`: still largely pre-localization and likely reference deleted cloud/agent models. Preserve all historical migrations and make removed-feature rows non-fatal/hidden as required by the design.
   - `app/src/lib.rs`: initialization still likely references removed singletons/crates and must become strictly local for GUI/TUI.
   - `app/src/settings_view/**`, `app/src/settings/**`, `app/src/app_menus.rs`: remove remaining account, billing, Agent, cloud sync, referral, and cloud CLI entries while preserving local settings and the fixed local Privacy page.
   - terminal call sites after removal of session-sharing/agent fields, especially local PTY manager and lifecycle APIs.
   - search/command palette variants and renderers after local-only action enum reductions.
   - local workspace/root APIs and tests after their substantial simplification.
3. Do not reintroduce compatibility stubs named after removed cloud/Agent APIs merely to compile. Move genuinely local value types to neutral local modules or delete dead consumers.
4. Restore required local behavior from §2 of the design: PTY/Ctrl-C, tabs/splits/window restore, editor/file tree/file search/code review, local/project workflows, legacy DB load, local notebook edit/restart persistence, PATH-only LSP, logs, bundled skills, macOS/Linux.
5. Reconcile the deletion inventory and evidence under `docs/redesign/phase1-acceptance/m5-m6/`; update `docs/redesign/baseline/deleted-test-ids.txt` and `script/deletion_set.txt` according to the design. Do not silently delete tests for retained behavior.
6. Complete M6 manifest/lockfile/deny work, including exact `email_address` rev and dependency/source/binary negative scans.
7. Run fresh formatting, focused tests, full prescribed checks, GUI/TUI real behavior verification, and the cross-platform matrix before the final non-WIP commit.

## Useful recovery/context notes
- The complete scope and gate commands are already in `docs/redesign/10-M5-M6未完成项实施详设.md`; reference it rather than copying its checklist.
- Baseline/decision evidence is under `docs/redesign/baseline/` and `docs/redesign/phase1-acceptance/m5-m6/`.
- Route A for blocklist was frozen in `docs/redesign/baseline/blocklist-decision.md`.
- M5g and M5b have focused evidence in `docs/redesign/phase1-acceptance/m5-m6/batches/` and their earlier commits.
- A previous disk issue came from a very large `target/debug/incremental`; if disk fills, inspect that directory before assuming a code failure.
- No credentials or secrets are required for the local focused checks.

## Suggested skills
Call the Skill tool for these before relevant work:
- `gui-ui-guidelines` — before repairing any GUI workspace, pane, settings, terminal, notebook, or workflow UI.
- `gui-settings-ui` — before changing Settings pages/search/widget wiring.
- `rust-unit-tests` — when restoring/adapting Rust tests for retained local behavior.
- `logging-and-error-reporting` — before adding or revising warnings for skipped legacy rows or failures.
- `tui-ui-guidelines`, `tui-testing`, and `tui-verify-change` — if touching or re-verifying the headless TUI.
- `cross-platform-cloud-verification` — only after cheap local checks pass, for the smallest required macOS/Linux verification matrix.

## Completion warning
Do not declare the active goal complete until every criterion has fresh evidence at the same final HEAD, all prescribed checks pass, `git status --porcelain` is empty, and the final commit is no longer a known-failing WIP.
