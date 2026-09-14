# Contributing to Term4u

Read [docs/DESIGN.md](docs/DESIGN.md) before changing the product. It is the only source for the
requirements, architecture, current completion state, implementation order and acceptance criteria.
Engineering conventions are in [AGENTS.md](AGENTS.md).

## Proposing a change

Describe the affected section or requirement/acceptance ID in the existing design. Update that design
in the same PR when behavior, scope or completion state changes. Do not add a parallel spec, roadmap,
status report, handoff or versioned design folder. Superseded content is replaced; Git keeps history.

Use this repository's issues and PRs for ordinary collaboration, not upstream Warp labels, Slack,
cloud agents or review workflows. Keep implementation changes scoped, preserve user data and the
inherited license boundaries, and obtain permission for destructive or privileged operations.

## Verification and review

Run the formatting and strict Clippy commands required by `script/presubmit`, along with the applicable
tests and the complete acceptance requirements in [the design](docs/DESIGN.md#verification).
Do not drop a gate because a tool is missing. State the exact source revision, commands, exit codes,
platform and limitations; historical results do not prove a new revision. Do not mark an interrupted
run or an unexecuted stage as passing.

Use [.github/pull_request_template.md](.github/pull_request_template.md). Record observed results and
links to evidence, but do not copy the design into the PR as a new authority. Runtime screenshots,
logs and packet captures must be sanitized before publication. Never commit real user databases or
credentials. Merging, tagging, releasing and privileged operations require their own authorization.

## Attribution and security

Keep the existing AGPL/MIT and third-party copyright and license notices. Moving or renaming code does
not change its license. Only contribute material you are entitled to contribute under the applicable
license; the directory and dependency boundaries are defined by the design.

Do not report non-public vulnerabilities in a public issue or PR. Confirm a private reporting channel
with the repository owner before sharing sensitive details; an upstream vendor's address is not a
Term4u reporting channel.
