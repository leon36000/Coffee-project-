# HermesClaw Canonical Facts

**Authority:** canonical facts
**Last updated:** 2026-08-14

## Identity

- Product working name: **HermesClaw**.
- Product goal: a single coherent application combining the strongest useful agent/cognition behaviors of Hermes Agent with the strongest action/gateway/device/automation behaviors of OpenClaw.
- The user explicitly rejected a final architecture that remains visibly or conceptually two products.
- The interface must be redesigned and improved rather than merely reusing/reskinning OpenClaw.

## Official repository

- Official GitHub repository: `https://github.com/leon36000/Coffee-project-`.
- Default branch: `main`.
- The exact recovered 85-file source baseline was established at commit `c3fd965d438a5a471c969f3fa1b447e3b6e34b31` from local source commit `d79d2ebb3c7a0092f833d5883f1522954566e031`.
- The canonical Web CI lockfile fix was merged through PR #6 at commit `2dff93bd73eda777a6e305ee5a658d146e5aa6c6`.
- GitHub Actions run `31837639423` verified the current `main` commit with green Rust, Web, and Tauri compile jobs.
- `leon36000/GitSpace` is unrelated prior work and must not be used for HermesClaw.

## Technology direction

- Backend target: Rust.
- UI target: React + TypeScript.
- Preferred desktop shell: Tauri.
- Canonical local product state target: SQLite behind Rust repository traits in the product architecture.
- The separate **project-memory RAG** uses RAGLite 1.1.1 + DuckDB; that database is not HermesClaw product state.

## Frozen upstream behavioral baselines

These are intentionally frozen migration references, not claims about the latest upstream versions:

- Hermes Agent baseline: `NousResearch/hermes-agent` version `0.20.0`, commit `ee472a7fdbbc55924f91ab122dbaa29bd07668b0`.
- OpenClaw baseline: `openclaw/openclaw` version `2026.8.1`, commit `42ad83142ee97b387c3d8ca7ee38dd1eadb668d6`.

Both baselines were selected for reproducible behavioral analysis. Upstream updates must be evaluated explicitly before changing a frozen baseline.

## Recovery history

The previous local prototype and its full Git history remain recovery provenance. They are no longer the canonical repository truth: current implementation claims must be checked against `leon36000/Coffee-project-` and its CI.
