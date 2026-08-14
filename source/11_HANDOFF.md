# HermesClaw Handoff

**Authority:** current handoff checkpoint
**Updated:** 2026-08-14

## Canonical direction

- Official HermesClaw GitHub target: `leon36000/Coffee-project-`.
- GitSpace is explicitly excluded.
- Backend destination: Rust.
- UI: React + TypeScript.
- Desktop shell: Tauri.
- Markdown in `source/` is canonical memory; RAGLite/DuckDB is derived retrieval infrastructure.

## Official repository state

- `main`: `2dff93bd73eda777a6e305ee5a658d146e5aa6c6`.
- Exact recovered baseline: `c3fd965d438a5a471c969f3fa1b447e3b6e34b31`.
- Baseline provenance source: local commit `d79d2ebb3c7a0092f833d5883f1522954566e031`.
- PR #6 fixed the canonical Web CI install command.
- Canonical baseline tree contains 85 source files and excludes build/bootstrap artifacts.

## Fresh evidence

GitHub Actions:

- branch run `31833887410` — success;
- PR run `31837393708` — success;
- post-merge `main` run `31837639423` — success;
- all three runs completed the Rust, Web, and Tauri compile jobs successfully.

Fresh local checks on the exact recovered source tree:

- Rust formatting — pass;
- Rust non-desktop tests — 14 passed, 0 failed;
- Rust non-desktop Clippy with `-D warnings` — pass;
- React test — 1 passed, 0 failed;
- TypeScript lint/typecheck — pass;
- React production build — pass.

## Current product capability

The repository contains the first architectural vertical proof: chat request -> deterministic/model-provider seam -> typed `workspace.list` call -> Observe policy -> workspace-confined execution -> SQLite evidence -> completed mission -> Axum/React/Tauri result presentation.

## Blockers

None for repository establishment or canonical CI.

## Exact next action

Design and implement the first roadmap Phase 3 increment: a constrained read-only `workspace.read` capability with shared workspace-boundary enforcement, explicit UTF-8 and byte limits, deterministic security tests, model-tool codecs, evidence, and end-to-end proof. Do not add process execution or file mutation in the same change.
