# HermesClaw Roadmap

**Authority:** current intended sequence; goals are not implementation claims
**Last updated:** 2026-08-14

## Phase 0 — Establish canonical project home

**Status: verified complete.**

- Official repository: `leon36000/Coffee-project-`.
- Exact recovered baseline established at `c3fd965d438a5a471c969f3fa1b447e3b6e34b31`.
- Canonical baseline documentation merged at `be10fe31fc2f61d75bec97293c8034927dcbee98`.
- GitHub Actions verified Rust, Web, and Tauri compile jobs on `main`.

## Phase 1 — Architectural spine

**Status: minimal architectural proof verified; production breadth not complete.**

The repository includes minimal implementations of:

- domain/state/policy contracts;
- model fabric and bounded agent loop;
- read-only capability broker;
- durable evidence and mission transitions;
- Axum API;
- new chat-first React UI and Tauri shell.

This proves the spine only. Each domain still requires capability-by-capability hardening and migration evidence.

## Phase 3 checkpoint — bounded file reads

**Status: implementation and branch CI verified; merge pending.**

`workspace.read` on `agent/workspace-read` adds:

- shared canonical workspace-boundary enforcement with `workspace.list`;
- a `65_536` byte limit;
- strict UTF-8 and NUL rejection;
- traversal, absolute escape, external symlink escape, directory, binary, and oversize tests;
- separate model result and sanitized evidence payloads;
- deterministic and OpenAI-compatible tool codecs;
- agent, SQLite evidence, Axum, React, and Tauri integration.

Local verification recorded 30 Rust tests and 2 React tests with zero failures. GitHub Actions run `31846047390` passed Rust, Web, and Tauri compile gates. This checkpoint is not canonical until its PR is merged and `main` passes again.

## Next bounded checkpoint after merge

Design an Assist-profile `workspace.write` capability as a separate slice. Its design must cover approval semantics, atomic replacement, path confinement, bounded input, overwrite policy, rollback/error evidence, and secret-safe audit records before implementation starts. File mutation and process execution must not be introduced implicitly through the read capability.

## Migration waves

1. Core domain/events/state/policy — initial proof present; continue hardening as required.
2. Model Fabric and minimal Agent Engine — initial proof present; continue provider/error/cancellation work incrementally.
3. File/process capability engine — **active**; `workspace.list` is canonical and `workspace.read` is verified pending merge. Mutation and process execution remain deferred.
4. Mission durability, cancellation, recovery.
5. Hermes memory/context/compression behaviors.
6. Hermes skills/delegation/self-improvement controls.
7. Automation scheduler/heartbeat.
8. Gateway/session/delivery primitives.
9. Initial messaging channels.
10. Browser actions.
11. Computer-use and remote nodes.
12. Voice/media and advanced channels.
13. Compatibility retirement and runtime simplification.

Each wave must be split into small capability contracts. “Wave complete” is never a single unbounded coding-agent task.
