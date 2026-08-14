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

## Repository state

- canonical `main` before feature merge: `be10fe31fc2f61d75bec97293c8034927dcbee98`;
- active branch: `agent/workspace-read`;
- feature implementation commit: `67026ab4eff7d7c895d746dcf0a641f85a73a44b`;
- clean branch checkpoint after transfer cleanup: `2d336a43c0d0e4c8eeb796fa8d3f3f9637e56623`;
- design: `docs/superpowers/specs/2026-08-14-workspace-read-design.md`;
- plan: `docs/superpowers/plans/2026-08-14-workspace-read-implementation.md`;
- evidence: `docs/evidence/2026-08-14-workspace-read-verification.md`.

## Implemented behavior

The branch adds a constrained `workspace.read` capability:

- shared canonical boundary with `workspace.list`;
- maximum `65_536` bytes;
- regular UTF-8 files only, no NUL;
- traversal, absolute escape, external symlink, directory, oversize, and invalid-encoding rejection;
- internal symlink acceptance with canonical relative path;
- full text returned only to the active model turn;
- SQLite evidence limited to path, byte count, and SHA-256;
- deterministic/OpenAI codecs and Axum/React/Tauri integration.

## Fresh evidence

TDD:

- GitHub Actions run `31844182703` failed because `ToolCall::workspace_read` did not yet exist, proving the domain test was red for the intended reason.

Local final verification:

- Rust formatting — pass;
- Rust non-desktop tests — **30 passed, 0 failed**;
- Rust non-desktop Clippy with `-D warnings` — pass;
- React tests — **2 passed, 0 failed**;
- TypeScript lint/typecheck — pass;
- React production build — pass;
- Tauri desktop compile check — pass;
- source validation — **19 documents validated**;
- diff whitespace check — pass.

GitHub Actions:

- hash-locked patch application run `31845995476` — success;
- independent branch CI run `31846047390` — Rust, Web, and Tauri compile jobs all succeeded.

## Independent review

No material mismatch with the approved design was found.

Unavailable checks are recorded honestly:

- Fallow CLI is not installed;
- SonarQube scanner/project configuration is not present.

Residual limitation: the current filesystem boundary is not an OS-specific race-free open broker. Do not claim TOCTOU hardening. Binary, partial, streaming, write, patch, delete, and process capabilities remain unsupported.

## Blockers

No technical blocker is known. The remaining release path is procedural: open PR, obtain green PR CI, merge, obtain green post-merge `main` CI, then update canonical state with the final merge commit and run identifiers.

## Exact next action

Open the `workspace.read` PR against `main`, complete review and PR CI, merge only while all required checks are green, run post-merge CI, then create the next design checkpoint for Assist-profile `workspace.write` without combining it with process execution.
