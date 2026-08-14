# HermesClaw Handoff

**Authority:** current handoff checkpoint  
**Updated:** 2026-08-13

## Canonical direction

- Official HermesClaw GitHub target: `leon36000/Coffee-project-`.
- GitSpace is explicitly excluded and was not touched during this checkpoint.
- Backend destination: Rust.
- UI: React + TypeScript.
- Desktop shell: Tauri.
- Markdown in `source/` is canonical memory; RAGLite/DuckDB is derived retrieval infrastructure.

## Recovered implementation

The local workspace `/mnt/data/HermesClaw` contains the vertical architectural proof. The canonical source pack and recovery provenance were added at local commit `e5479d3`; the verified checkpoint documentation was committed immediately afterward.

Recovered prototype history prior to the memory commit ends at `4e63056`. The commit list is recorded in `docs/recovery/LOCAL_HISTORY.md`.

## Current executable evidence

Verified on 2026-08-13 against the recovered tree:

- Rust formatting: pass;
- Rust non-desktop test suite: 14 passed, 0 failed;
- Rust non-desktop Clippy with `-D warnings`: pass;
- web test suite: 1 passed, 0 failed;
- web production build: pass;
- TypeScript no-emit lint/typecheck: pass;
- source-manifest integrity: 19 documents validated;
- Tauri pkg-config prerequisites: resolved from a 412-package checksum-verified local sysroot.

The full desktop compile did not finish within tool execution windows and remains unverified until official CI runs.

## Blocking condition

`Coffee-project-` is still empty. The current GitHub connector is read-only for this project, and the container cannot reach GitHub over the network. No remote mutation was attempted against another repository.

Prepared transfer artifacts:

- `/mnt/data/HermesClaw-official-baseline.bundle` — full Git history;
- `/mnt/data/HermesClaw-official-baseline.tar.gz` — tracked source snapshot;
- `/mnt/data/HermesClaw-official-baseline.zip` — tracked source snapshot;
- accompanying SHA-256 files.

## Exact next step

Populate `leon36000/Coffee-project-` from the prepared bundle or enable GitHub write operations, then run `.github/workflows/ci.yml`. Once CI is green, update `03_CURRENT_STATE.md` and this file with the official commit SHA, workflow run IDs, exact gate results, and the first next capability contract.
