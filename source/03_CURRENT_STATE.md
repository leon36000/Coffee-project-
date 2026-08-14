# HermesClaw Current State

**Authority:** current operational state
**Snapshot date:** 2026-08-14
**Volatility:** high — reverify before repository/code actions

## Official repository checkpoint

The official repository is `leon36000/Coffee-project-`, default branch `main`.

Verified live on 2026-08-14:

- current `main` commit: `2dff93bd73eda777a6e305ee5a658d146e5aa6c6`;
- recovered source baseline commit: `c3fd965d438a5a471c969f3fa1b447e3b6e34b31`;
- exact baseline tree: 85 tracked files;
- no bootstrap payloads, `target/`, `node_modules/`, `dist/`, or migration workflows are present in the canonical baseline tree;
- PR #6 corrected the Web dependency install command without changing application behavior;
- `leon36000/GitSpace` was not used for HermesClaw work.

## Current executable evidence

GitHub Actions run `31837639423` against `main` commit `2dff93bd73eda777a6e305ee5a658d146e5aa6c6` completed successfully:

- **Rust proof gates:** formatting, locked workspace tests excluding the desktop crate, and Clippy with `-D warnings` — pass;
- **Web proof gates:** locked dependency install, cockpit tests, and production build — pass;
- **Tauri desktop compile gate:** Linux system dependencies plus `cargo check -p hermesclaw-desktop --locked` — pass.

Additional verification rerun locally on the recovered exact source tree with Rust 1.97.1 and checksum-verified vendored dependencies:

- `cargo fmt --all --check` — pass;
- `cargo test --workspace --exclude hermesclaw-desktop --locked --offline` — 14 passed, 0 failed;
- `cargo clippy --workspace --exclude hermesclaw-desktop --all-targets --locked --offline -- -D warnings` — pass;
- `npm test` in `apps/web` — 1 passed, 0 failed;
- `npm run lint` in `apps/web` — pass;
- `npm run build` in `apps/web` — pass.

## Implemented architectural proof

The official baseline currently proves:

- typed domain, mission, policy, model, capability, evidence, and API boundaries;
- a bounded Rust agent turn loop;
- `workspace.list` as a low-risk read-only capability;
- canonical path and symlink escape rejection for that capability;
- Observe-profile policy evaluation before execution;
- ordered evidence persistence in SQLite;
- Axum API, React mission cockpit, and Tauri desktop shell integration;
- deterministic and OpenAI-compatible model-provider seams.

This is an architectural vertical proof, not a claim that HermesClaw feature migration is complete.

## Current blockers

No Phase 0 repository or CI blocker remains.

## Next safe capability checkpoint

The next roadmap-aligned slice is a bounded **`workspace.read`** capability:

1. specify its typed contract and size/encoding limits;
2. centralize workspace-boundary resolution shared with `workspace.list`;
3. write failing traversal, symlink, directory, binary, and oversize tests;
4. implement the smallest read-only capability;
5. connect deterministic/OpenAI tool codecs, agent evidence, API, and UI behavior;
6. run Rust, Web, security, integration, and Tauri compile gates;
7. update this file and `11_HANDOFF.md` with exact evidence.
