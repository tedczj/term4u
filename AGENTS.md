# AGENTS.md

This file provides guidance when working with code in this repository.

## Term4u Scope and Current Plan

Term4u is a macOS-only local/offline derivative of Warp. Internal `warp_*` crate names are retained;
GUI and TUI binaries are `term4u` and `term4u-tui`. The active implementation and acceptance checklist is
[docs/redesign/11-本地化收敛施工单.md](docs/redesign/11-本地化收敛施工单.md); the
[design index](docs/redesign/README.md) and [milestones](docs/redesign/08-实施顺序与里程碑.md)
track its status. Historical upstream designs are not parallel task lists.

As reviewed at `eef716cd69b84e0676eb90ad3ee776397aa6b515` on 2026-09-13, R1/R2 have recorded batch
acceptance; R3–R6 and V1 remain open. A previous full presubmit PASS does not apply automatically to a
later source revision. The latest GUI-shell record stops at missing `clang-format`; do not claim that
its later WGSL, workspace nextest or doc-test stages passed.

Route A removes the built-in Warp Agent UI/SDK/execution layer, cloud control plane and retired
`crates/integration` harness. Do not restore these to satisfy old documentation, skills or test paths.
External CLI agents remain ordinary user-launched PTY subprocesses. Product network guards must stay
hard-offline; this does not prohibit networking explicitly requested through the user's shell commands.

Preserve the Term4u runtime identity, real user data, historical migrations and raw legacy data.
Use isolated profiles/copies for tests. Follow the active checklist's authorization boundaries for
commits, pushes, PRs, tags, system installation and privileged capture. M7 branding/release and the
independent full-MIT route are not part of the current localization work.

## Development Commands

### Build and Run
- `cargo run` / `./script/run` - Build and run the GUI desktop app locally
- `./script/run-tui` - Build and run the headless TUI front-end (`crates/warp_tui`)
- `cargo bundle --bin term4u --no-default-features --features local_only` - Bundle the local GUI app

### Local Runtime

There is no supported local `warp-server` backend in the current Route A product. Do not use the old
`WITH_LOCAL_SERVER`, `SERVER_ROOT_URL` or `WS_SERVER_URL` setup to restore removed cloud interfaces.
Use the local runtime and the build/feature matrix in the active checklist instead.

### Testing
- `cargo nextest run --no-fail-fast --workspace --exclude command-signatures-v2` - Run tests with nextest
- `cargo nextest run -p warp_completer --features v2` - Run completer tests with v2 features
- `cargo test --doc` - Run doc tests
- `cargo test` - Run standard tests for individual packages
- `./script/test_inventory` - Verify the current test-ID inventory against the baseline and explicit deletion list

The inventory failure-handling unit tests in `script/presubmit` do not replace the actual inventory
verification. R1/R2's authorized removal of retired integration suites is recorded with its ID audit;
do not restore that harness or treat the authorization as permission to delete future retained-behavior tests.

### Linting and Formatting
- `./script/presubmit` - Run all presubmit checks (fmt, clippy, tests)
- `./script/format` - Format code
- `cargo clippy --workspace --exclude warp_completer --all-targets --tests -- -D warnings` - Workspace Clippy
- `cargo clippy -p warp --all-targets --tests -- -D warnings` - Default GUI Clippy, separate from workspace feature unification
- `cargo clippy -p warp_completer --all-targets --tests -- -D warnings` - Default completer Clippy
- `./script/run-clang-format.py -r --extensions 'c,h,cpp,m' ./crates/warpui/src/ ./app/src/` - Format C/C++/Obj-C code
- `find . -name "*.wgsl" -exec wgslfmt --check {} +` - Check WGSL shader formatting

Use the actual commands in `script/presubmit` and the additional acceptance matrix in chapter 11.
Do not add `--all-features` as a substitute for the supported feature combinations. Restore missing
tools without deleting checks, and record stages that did not execute as unverified.

### Platform Setup
- `./script/bootstrap` - Install platform development dependencies. It does not download or update agent skills.
- `./script/install_cargo_build_deps` - Install Cargo build dependencies
- `./script/install_cargo_test_deps` - Install Cargo test dependencies

Only macOS is a supported product/acceptance platform. Linux cleanup is deferred; remaining Windows
exclusive files/dependencies are tracked by R4 with explicit shared-code/migration exceptions.
Repository-local and bundled skills are used exactly as checked in. Runtime and build scripts never download or update them.

## Architecture Overview

This is a Rust-based terminal emulator with a custom UI framework called **WarpUI**. It has **two front-ends** that share a common core.

### Front-ends: GUI and TUI

Term4u has two front-ends that share the `warp_core`/`warpui` Entity/model core (App/Entity/`AppContext`, actions, `Appearance`, `FeatureFlag`, local logging) but differ in UI framework, rendering, input, and verification:
- **GUI desktop app** — the `app/` crate on the WarpUI pixel/GPU framework (`warpui`, `crates/warpui_core`): `Element`/`View` layout, GPU/WGSL rendering, mouse input, `.app` bundles. Run with `cargo run` / `./script/run`; verify on a real macOS display and with the retained local regression tests. The old `crates/integration` framework has been retired.
- **Headless TUI** — the `crates/warp_tui` crate: a console app (run with `./script/run-tui`; no `.app`/GPU) rendered with a parallel cell-grid element library at `crates/warpui_core/src/elements/tui` (the `TuiElement` trait), behind the `tui` cargo feature. Verify by running it in a real terminal and observing output; test with render-to-lines unit tests.

**Skill convention:** a skill specific to one front-end says so in its name and/or description (e.g. `gui-ui-guidelines` / `gui-integration-test` are GUI-only; `tui-ui-guidelines`, `tui-testing`, and `tui-verify-change` are TUI-specific). Skills with no front-end call-out are surface-agnostic and apply to both. For TUI work prefer the `tui-*` skills and ignore GUI-only ones — and vice versa. Historical skill steps that require removed cloud services or the integration harness are obsolete; preserve the real-display/real-PTY validation requirement without restoring those dependencies.

### Key Components

**Shared UI core** (`crates/warpui`, `crates/warpui_core`) — used by **both** front-ends:
- Entity-Component-Handle pattern: a global `App` object owns all views/models (entities); views hold `ViewHandle<T>` references to other views; `AppContext` provides temporary access to handles during render/events.
- Actions system for event handling.
- `crates/warpui_core` also hosts the TUI cell-grid element library under `src/elements/tui` (behind the `tui` feature).

**GUI rendering** (WarpUI GUI elements — GUI-specific):
- `Element`s describe visual layout (Flutter-inspired), rendered on the GPU (WGSL).
- Mouse input uses `MouseStateHandle`: create it once during construction and reference/clone it wherever mouse input is tracked. An inline `MouseStateHandle::default()` while rendering means no mouse interactions work. (The TUI's hover/click elements — `TuiHoverable`, `tui_collapsible` — also build on `MouseStateHandle`, so the same ownership rule applies there.)

**TUI rendering** (`crates/warp_tui` + `crates/warpui_core/src/elements/tui` — TUI-specific):
- Headless console front-end. The `TuiElement` trait lays out and paints into a cell-grid `TuiBuffer`; crossterm input is converted to `TuiEvent`. No GPU/WGSL, pixel geometry, or `.app` bundle.

**Main app / shared surfaces** (`app/`) — the GUI desktop app plus feature surfaces the TUI reuses:
- Terminal emulation and shell management (`terminal/`)
- Local workflows, notebooks and persistence (`workflows/`, `notebooks/`, `local_objects/`, `persistence/`)
- Local settings and preferences (`settings/`, `settings_view/`)
- Workspace and session management (`workspace/`)

The former built-in Agent, Drive/cloud synchronization, server authentication and GraphQL interfaces
are not supported capabilities. Audit any remaining files by consumers and module reachability;
an unmounted legacy file is cleanup work, not proof of an active feature or a compiler error.

**Core Libraries**:
- `crates/warp_core/` - Core utilities and platform abstractions (shared)
- `crates/warp_tui/` - Headless TUI front-end
- `crates/editor/` - Text editing functionality
- `crates/warpui/` and `crates/warpui_core/` - Custom UI framework (shared core plus the GUI and TUI element libraries)
- `crates/ipc/` - Inter-process communication
- `crates/persistence/` - SQLite persistence, historical migrations and legacy fixtures

### Key Architectural Patterns

1. **Entity-Handle System**: Views reference other views via handles, not direct ownership
2. **Modular Structure**: Workspace contains multiple workspace configurations, each with terminals, notebooks, etc.
3. **macOS Scope**: Remaining platform conditionals do not imply support for Linux, Windows or WASM products
4. **Local Runtime**: Retained terminal/editor/data capabilities do not depend on a cloud backend
5. **Offline Boundaries**: Preserve guarded product networking and the separate boundary for user-launched shell subprocesses

### Development Guidelines

**Workspace Structure**:
- This is a Cargo workspace; inspect the current manifests/metadata rather than relying on a historical member count
- Main binary is in `app/`, UI framework in `crates/warpui/`
- Platform-specific code is conditionally compiled
- Local regression tests live with their modules; legacy SQLite fixtures are in `crates/persistence/fixtures/legacy/`

**Coding Style Preferences**:
- Avoid unnecessary type annotations, especially in closure params.
- Avoid using too many Rust path qualifiers and use imports for concision. Place import statements at the top of the file as per convention.
  An exception to this is inside cfg-guarded code branches. In those cases, you can either embed the import into the relevant scope or just use an absolute path for one-offs.
- If a function takes a context parameter (`AppContext`, `ViewContext`, or `ModelContext`), it should be named `ctx` and go last. The one exception is for
  functions that take a closure parameter, in which case the closure should be last.
- Always remove unused parameters completely rather than prefixing them with `_`. Update the function signature and all call sites accordingly.
- Prefer inline format arguments in macros like `println!`, `eprintln!`, and `format!` (for example, `eprintln!("{message}")` instead of `eprintln!("{}", message)`) to satisfy Clippy's `uninlined_format_args` lint.
- Do not pass `Itertools::format` results directly to logging macros (`log::*`, `safe_*`, etc.). `Itertools::format` produces a single-use formatter, while logging implementations may format a message more than once. Use a reusable `String` such as `iter.join(", ")` for logging arguments instead. Direct use in `format!` or `write!` is fine.
- When adding a toggleable setting, also add the matching Command Palette enable/disable entry and any required context flags so the setting is discoverable outside Settings.

**Comments**:
Comments have a cost. They carry a maintenance burden, because they must be kept in sync
with the code they describe. It is tempting to assume that more comments is always better,
but be judicious about when a comment is actually necessary because the code cannot speak
for itself.
- **Minimalist Comments**: Assume the reader is a Senior Software Engineer. Never comment
  to explain WHAT or HOW code works if self-documenting names accomplish that.
- **Strictly "Why" Only**: Reserve inline comments strictly for non-obvious business
  rationale, workarounds for third-party bugs, complex algorithms, unidiomatic code, or
  unexpected edge cases.
- **No Line-by-Line Narrations**: Never add comments restating the syntax (e.g., omit
  `# Initialize array`, `# Loop over users`).
- **Clean Docstrings**: Keep doc comments concise. Document public APIs, arguments, types,
  and returns. Do not narrate the method's internal implementation steps.
- **Single-source of documentation**: For items/members that have a doc comment explaining
  their purpose, you do not need to repeat that explanation anywhere else. A good example
  is a float const specifying an amount of spacing. You may use a doc comment on the
  declaration if necessary, but do not repeat that where the const is *referenced*. Another
  example is function call sites. Function doc comments explain what they do. Do not repeat
  the explanation at the call site.
- **Container docs describe the whole, member docs describe the parts**: A field's, variant's,
  or parameter's own doc comment is where that member gets explained. A container's item-level
  doc comment (struct, enum, trait) describes the item as a whole and must not enumerate or
  re-explain its members. Do not name members in the container's doc comment just to describe
  them, and do not restate in a member's doc comment what the container's doc comment already
  said. Behavior genuinely shared by several members belongs in exactly one place, not both.
- **Don't enumerate function call sites in doc comments**: Function doc comments should
  document their behavior and NOT their callers, e.g. it should never say things like,
  "this is used by [certain callers]" or "this is used when...".
- **No "transformation comments"**: Do not add comments that explain *your edits*. Comments
  only need explain the *current state* of the code. Explanations of edits belong in pull
  request comments instead. You shouldn't add comments with phrases like, "this used to do
  so-and-so".
- Do not remove existing comments when making unrelated changes. Only remove or modify a
  comment if the logic it describes has changed.
- The formatter (`./script/format`) is configured with a `max_width` (max line length) of
  100. Flow (reflow) comment line-wrapping to fill that full width rather than wrapping
  early at a narrower column, so comments span as few lines as possible.

**Terminal Model Locking**:
- Be extremely careful when calling `model.lock()` on the terminal model (`TerminalModel`). Acquiring multiple locks on the same model from different call sites can cause a deadlock, resulting in a UI freeze (beach ball on macOS).
- Before adding a new `model.lock()` call, verify that no caller in the current call stack already holds the lock.
- Prefer passing already-locked model references down the call stack rather than acquiring new locks.
- If you must lock the model, keep the lock scope as short as possible and avoid calling other functions that might also attempt to lock.

**Testing**:
- Use `cargo nextest` for parallel test execution
- Use the retained local GUI regression tests and real macOS GUI scenarios. The retired `crates/integration/` framework is not an executable test entry. TUI elements/screens use render-to-lines unit tests and real PTY checks (see the `tui-testing` skill).
- Tests should be run via presubmit script before submitting
- Unit tests should be placed in separate files using the naming convention `${filename}_tests.rs` or `mod_test.rs`
- Test files should be included at the end of their corresponding module with:
  ```rust
  #[cfg(test)]
  #[path = "filename_tests.rs"]  // or "mod_test.rs"
  mod tests;
  ```

**Pull Request Workflow**:
- **ALWAYS** run `./script/format` and `cargo clippy` (the versions specified in ./script/presubmit) before opening a PR or pushing updates to an existing PR branch
- Those commands must pass completely before creating or updating a pull request
- Specifically, ensure `./script/format` and `cargo clippy` checks pass
- If they fail, fix all issues before proceeding with the PR
- Do not create public pull requests or public issues that disclose a non-public security vulnerability. Refer users to `SECURITY.md` for the proper disclosure methods instead.
- This applies to:
  - Opening new pull requests
  - Pushing new commits to existing PR branches
  - Any branch updates that will be reviewed
 - When opening PRs, use the PR template at `.github/pull_request_template.md`
 - Add changelog entries when appropriate using the format at the bottom of the PR template. Use the following prefixes (without the `{{}}` brackets):
   - `CHANGELOG-NEW-FEATURE:` for new, relatively sizable features (use sparingly - these may get marketing/docs)
   - `CHANGELOG-IMPROVEMENT:` for new functionality of existing features
   - `CHANGELOG-BUG-FIX:` for fixes related to known bugs or regressions
   - `CHANGELOG-IMAGE:` for GCP-hosted image URLs
   - Leave changelog lines blank or remove them if no changelog entry is needed

**Database**:
- Uses Diesel ORM with SQLite
- Migrations in `crates/persistence/migrations/`
- Schema defined in `crates/persistence/src/schema.rs`
- Preserve historical migrations/raw data and run compatibility checks on isolated fixture copies

**Removed Cloud Interfaces**:
- Do not regenerate or restore the removed GraphQL/server interfaces from historical instructions
- Remaining local value types must have an actual local consumer and must not reintroduce cloud providers

### Feature Flags

Cargo feature combinations and runtime `FeatureFlag` settings are distinct. Local runtime flags must
not re-enable deleted cloud interfaces or weaken `offline_hard` boundaries.

How to add a feature flag:
- Add a new variant to `warp_core/src/features.rs` in the `FeatureFlag` enum
- (Optional) Enable it by default for dogfood builds by listing it in `DOGFOOD_FLAGS`
- Gate code paths with `FeatureFlag::YourFlag.is_enabled()`
- For preview or release rollout, add to `PREVIEW_FLAGS` or `RELEASE_FLAGS` respectively (as appropriate)

Best practices:
- **Prefer runtime checks over cfg directives**: Prefer `FeatureFlag::YourFlag.is_enabled()` over `#[cfg(...)]` compile-time directives so flags can be toggled without recompilation and are easier to clean up later. Use `#[cfg(...)]` only when the code cannot compile without them (for example, platform-specific code or dependencies that do not exist when the feature is disabled).
- Keep flags high-level and product-focused rather than per-call-site
- Remove the flag and dead branches after launch has stabilized
- For UI sections that expose a new feature, hide the UI behind the same flag

Example:
```rust
#[derive(Sequence)]
pub enum FeatureFlag {
    YourNewFeature,
}

// Default-on for dogfood builds
pub const DOGFOOD_FLAGS: &[FeatureFlag] = &[
    FeatureFlag::YourNewFeature,
];

// Use in code
if FeatureFlag::YourNewFeature.is_enabled() {
    // gated behavior
}
```

### Exhaustive Matching

When adding/editing match statements, avoid using the wildcard _ when at all possible. Exhaustive matching is helpful for ensuring that all variants are handled, especially when adding new variants to enums in the future.
