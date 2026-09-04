# M5g · local-only LSP discovery

- Implementation commit: commit containing this record
- Verification UTC: 2026-09-04T10:44:00Z
- Result: PASS

| Check | Command | Exit | Result |
|---|---|---:|---|
| LSP compile | `cargo check -p lsp --all-targets --tests` | 0 | PASS |
| GUI local-only compile | `cargo check -p warp --all-targets --tests --no-default-features --features local_only` | 0 | PASS |
| LSP tests | `cargo nextest run -p lsp` | 0 | 18 passed |
| LSP clippy | `cargo clippy -p lsp --all-targets --tests -- -D warnings` | 0 | PASS |
| Download API scan | `rg 'fetch_latest_server_metadata|install_from_github|fetch_npm_package_version|GITHUB_API_URL' crates/lsp app/src` | 1 | No matches |
| Test inventory | `./script/test_inventory` | 0 | 9,505 current; 297 approved deletions |

The candidate protocol now performs repository heuristics and executable checks only. Server startup uses
only the interactive PATH. Missing binaries return deterministic manual-installation guidance and never
query versions, invoke package managers, download archives, or inspect a managed installation directory.
`supported_servers_tests.rs` verifies PATH discovery and the exact missing-server guidance.
