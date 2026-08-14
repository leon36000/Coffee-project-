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

- canonical `workspace.read` merge commit: `ec3cfff0e938d4bc5e17d36d30a6d84f52cbdd19`;
- merged pull request: #8;
- feature implementation commit before squash: `67026ab4eff7d7c895d746dcf0a641f85a73a44b`;
- design: `docs/superpowers/specs/2026-08-14-workspace-read-design.md`;
- plan: `docs/superpowers/plans/2026-08-14-workspace-read-implementation.md`;
- evidence: `docs/evidence/2026-08-14-workspace-read-verification.md`.

## Canonical capability

HermesClaw now includes constrained `workspace.read`:

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

- run `31844182703` failed because `ToolCall::workspace_read` did not yet exist, proving the domain test was red for the intended reason.

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

- branch run `31846047390` — Rust, Web, and Tauri compile jobs succeeded;
- PR run `31846845864` — Rust, Web, and Tauri compile jobs succeeded;
- post-merge `main` run `31847082077` — Rust, Web, and Tauri compile jobs succeeded.

## Independent review

No material mismatch with the approved design was found.

Unavailable checks are recorded honestly:

- Fallow CLI is not installed;
- SonarQube scanner/project configuration is not present.

Residual limitation: the current filesystem boundary is not an OS-specific race-free open broker. Do not claim TOCTOU hardening. Binary, partial, streaming, write, patch, delete, and process capabilities remain unsupported.

## Blockers

None for the merged `workspace.read` checkpoint.

## Exact next action

Brainstorm and approve a separate Assist-profile `workspace.write` design covering approval, atomic replacement, bounded input, overwrite semantics, rollback/error evidence, and secret-safe audit records. Do not combine process execution into that slice.
