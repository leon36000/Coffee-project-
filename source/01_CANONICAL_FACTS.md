# HermesClaw Canonical Facts

**Authority:** canonical facts  
**Last updated:** 2026-08-13

## Identity

- Product working name: **HermesClaw**.
- Product goal: a single coherent application combining the strongest useful agent/cognition behaviors of Hermes Agent with the strongest action/gateway/device/automation behaviors of OpenClaw.
- The user explicitly rejected a final architecture that remains visibly or conceptually two products.
- The interface must be redesigned and improved rather than merely reusing/reskinning OpenClaw.

## Official repository

- Intended official GitHub repository: `https://github.com/leon36000/Coffee-project-`.
- As verified on 2026-08-13 during creation of this source pack, that repository is currently empty.
- `leon36000/GitSpace` is unrelated prior work and must not be used for HermesClaw.

## Technology direction

- Backend target: Rust.
- UI target: React + TypeScript.
- Preferred desktop shell: Tauri.
- Canonical local product state target: SQLite behind Rust repository traits in the product architecture.
- The separate **project-memory RAG** in this source pack uses RAGLite 1.1.1 + DuckDB; that database is not HermesClaw product state.

## Frozen upstream behavioral baselines

These are intentionally frozen migration references, not claims about the latest upstream versions:

- Hermes Agent baseline: `NousResearch/hermes-agent` version `0.20.0`, commit `ee472a7fdbbc55924f91ab122dbaa29bd07668b0`.
- OpenClaw baseline: `openclaw/openclaw` version `2026.8.1`, commit `42ad83142ee97b387c3d8ca7ee38dd1eadb668d6`.

Both baselines were selected for reproducible behavioral analysis. Upstream updates must be evaluated explicitly before changing a frozen baseline.

## Previous local prototype

A prior session created a local prototype at `/mnt/data/HermesClaw` with Rust crates, an Axum API, React UI, Tauri shell, design/spec files, and commits through `4e63056`.

This path and commit history are **recovery context, not official GitHub truth**. A future session must verify whether the local workspace still exists and must not assume the prototype has been transferred to `Coffee-project-` until GitHub proves it.
