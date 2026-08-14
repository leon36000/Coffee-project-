# HermesClaw Canonical Decisions

**Authority:** active product and architecture decisions  
**Last updated:** 2026-08-13

## D-001 — New product, not integration shell

HermesClaw is a third product with its own identity and architecture. The desired end state is not “Hermes process + OpenClaw process + bridge.” Temporary compatibility processes are allowed only during migration.

## D-002 — Rust-first backend

Rust is the destination architecture from the first implementation wave. Do not translate Hermes Python to TypeScript and then translate the result again to Rust. Port behavior directly into Rust contracts while using upstream runtimes only as temporary references/adapters.

## D-003 — React/TypeScript UI remains

Do not rewrite ordinary UI logic in Rust. React + TypeScript remains the canonical frontend; Tauri hosts the desktop app.

## D-004 — Capability-based migration

For each upstream behavior:

`inventory -> classify -> contract -> fixture -> failing test -> Rust implementation -> differential/security tests -> benchmark -> cutover`

Classifications: `KEEP-AS-REFERENCE`, `ADAPT-TEMPORARILY`, `PORT`, `MERGE`, `REDESIGN`, `DROP`, `NEW`.

## D-005 — One owner per concern

Canonical owners:

- agent lifecycle -> Agent Engine
- durable work -> Mission Engine
- capability invocation -> Capability Engine
- permissions/approvals -> Policy Kernel
- memory -> Memory Engine
- model/provider access -> Model Fabric
- automation -> Automation Engine
- channels/delivery -> Gateway Engine
- devices/workers -> Node Engine
- evidence/audit -> Evidence Engine
- UI state -> projection of canonical backend state

## D-006 — Chat-first new interface

Chat is the default interaction surface. Complex work promotes into inspectable Missions showing status, agents, actions, tests, approvals, evidence, artifacts, and budgets without exposing private chain-of-thought.

## D-007 — Evidence over assertions

No agent may declare a port, fix, merge, test, build, or release successful solely because generated code looks correct. Evidence must be executable and current.

## D-008 — Source memory is Markdown-first

The project source files are canonical durable memory. RAGLite is retrieval infrastructure only. Rebuild the RAG index when canonical source changes; never write new project truth only into the RAG database.

## D-009 — Repository isolation

`Coffee-project-` is the HermesClaw GitHub target. GitSpace is explicitly excluded from HermesClaw work unless the user overrides this decision in the current conversation.
