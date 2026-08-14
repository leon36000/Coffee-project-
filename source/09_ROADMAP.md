# HermesClaw Roadmap

**Authority:** current intended sequence; goals are not implementation claims
**Last updated:** 2026-08-14

## Phase 0 — Establish canonical project home

**Status: verified complete.**

- Official repository: `leon36000/Coffee-project-`.
- Exact recovered baseline established at `c3fd965d438a5a471c969f3fa1b447e3b6e34b31`.
- Canonical CI fix merged at `2dff93bd73eda777a6e305ee5a658d146e5aa6c6`.
- GitHub Actions run `31837639423` verified Rust, Web, and Tauri compile jobs on `main`.
- Canonical state and handoff are recorded in `03_CURRENT_STATE.md` and `11_HANDOFF.md`.

## Phase 1 — Architectural spine

**Status: minimal architectural proof verified; production breadth not complete.**

The repository currently includes minimal implementations of:

- domain/state/policy contracts;
- model fabric and bounded agent loop;
- read-only capability broker;
- durable evidence and mission transitions;
- Axum API;
- new chat-first React UI and Tauri shell.

This proves the spine only. Each domain still requires capability-by-capability hardening and migration evidence.

## Active migration checkpoint

The next bounded slice is `workspace.read`, extending the file capability family without adding process execution or mutation. It must introduce shared workspace-boundary enforcement, explicit byte/UTF-8 limits, security tests, tool codecs, evidence, and end-to-end proof.

## Migration waves

1. Core domain/events/state/policy — initial proof present; continue hardening as required.
2. Model Fabric and minimal Agent Engine — initial proof present; continue provider/error/cancellation work incrementally.
3. File/process capability engine — **active**, beginning with `workspace.read`; process execution remains deferred.
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
