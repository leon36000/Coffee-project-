# HermesClaw Current State

**Authority:** current operational state
**Snapshot date:** 2026-08-14
**Volatility:** high — reverify before repository/code actions

## Official repository checkpoint

The official repository is `leon36000/Coffee-project-`, default branch `main`.

Verified live on 2026-08-14:

- `workspace.read` merge commit: `ec3cfff0e938d4bc5e17d36d30a6d84f52cbdd19`;
- merged pull request: #8;
- exact feature implementation commit before squash: `67026ab4eff7d7c895d746dcf0a641f85a73a44b`;
- the canonical repository contains no bootstrap payload, generated transfer workflow, `target/`, `node_modules/`, or `dist/` artifact;
- `leon36000/GitSpace` was not used for HermesClaw work.

## Current executable evidence

Local verification on the implemented source tree with Rust `1.97.1`, locked dependencies, and the checksum-verified Tauri Linux package set:

- `cargo fmt --all --check` — pass;
- `cargo test --workspace --exclude hermesclaw-desktop --locked --offline` — **30 passed, 0 failed**;
- `cargo clippy --workspace --exclude hermesclaw-desktop --all-targets --locked --offline -- -D warnings` — pass;
- `npm --prefix apps/web test` — **2 passed, 0 failed**;
- `npm --prefix apps/web run lint` — pass;
- `npm --prefix apps/web run build` — pass;
- `cargo check -p hermesclaw-desktop --locked --offline` — pass;
- `python source/raglite/validate_source.py` — **19 documents validated**;
- `git diff --check` — pass.

GitHub Actions evidence:

- TDD red run `31844182703` failed for the intended missing `ToolCall::workspace_read` constructor;
- independent branch CI run `31846047390` succeeded;
- PR CI run `31846845864` succeeded;
- post-merge `main` CI run `31847082077` succeeded;
- each green run completed Rust formatting/tests/Clippy, Web install/tests/build, and the Tauri desktop compile gate.

## Canonical `workspace.read` capability

The merged product now proves:

- canonical `ToolCall::workspace_read` domain construction;
- one shared `WorkspaceBoundary` used by `workspace.list` and `workspace.read`;
- regular-file-only UTF-8 reads capped at exactly `65_536` bytes;
- rejection of parent traversal, absolute escape, external symlink escape, directories, `65_537`-byte files, invalid UTF-8, and NUL-containing content;
- acceptance of an internal symlink with the canonical target path returned;
- model-visible output `{path, content, bytes}`;
- persisted evidence `{path, bytes, sha256}` with no file content;
- deterministic and OpenAI-compatible model codecs;
- Observe policy, agent, SQLite evidence, Axum, React, and Tauri integration;
- preservation of existing `workspace.list` behavior.

## Review status and limitations

Independent specification/engineering review found no material requirement violation.

Tooling status:

- Fallow CLI was not installed in the active verification environment, so no Fallow graph verdict is claimed;
- no SonarQube project configuration or scanner was present, so no SonarQube result is claimed.

Known bounded limitation:

- canonicalization, metadata inspection, and file open are not protected by an OS-specific race-free broker. A hostile local actor could replace or grow a filesystem object between checks. The post-read byte limit prevents oversized content from being returned, but the implementation does not claim adversarial TOCTOU hardening.

The capability does not support binary files, partial reads, streaming, writes, patches, deletion, or process execution.

## Current blockers

No repository, implementation, or CI blocker is known for the merged `workspace.read` checkpoint.

## Next safe checkpoint

Design — as a separate approved slice — an Assist-profile `workspace.write` capability with explicit policy/approval behavior, atomic replacement, bounded input, overwrite semantics, rollback/error evidence, and secret-safe audit records. Process execution remains deferred.
