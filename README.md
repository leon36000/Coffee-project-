# HermesClaw

HermesClaw is a Rust-first agent runtime and mission cockpit being built from the useful behavioral ideas of Hermes Agent and OpenClaw without keeping either product as the final runtime boundary.

## Vertical proof v0.1

The first executable slice proves one complete, bounded path:

1. a user submits a mission;
2. the Rust `TurnCoordinator` calls a model provider;
3. the model emits a canonical `workspace.list` capability request;
4. the Policy Kernel evaluates the request under the visible `Observe` autonomy profile;
5. the capability broker executes only inside the configured workspace boundary;
6. ordered policy and execution evidence is stored in SQLite;
7. the mission transitions through verification to `completed`;
8. the result, mission state, trace ID and evidence are rendered in the React cockpit.

The deterministic provider makes this slice reproducible. An OpenAI-compatible provider adapter is also present but is not enabled by default in the proof executable.

## Repository layout

```text
apps/web/                 React + TypeScript mission cockpit
apps/desktop/src-tauri/   Tauri desktop shell using direct Rust IPC
crates/hc-agent/          bounded agent turn coordinator
crates/hc-api/            web HTTP API and proof service
crates/hc-domain/         canonical domain contracts
crates/hc-mission/        mission state machine
crates/hc-models/         deterministic + OpenAI-compatible model adapters
crates/hc-policy/         policy decisions
crates/hc-state/          SQLite evidence store
crates/hc-tools/          capability broker + safe workspace.list
```

## Run the proof API

Requires Rust 1.97.1 or a compatible toolchain.

```bash
cargo run -p hc-api
```

Environment variables:

- `HERMESCLAW_WORKSPACE` — workspace exposed to `workspace.list`; defaults to the current directory.
- `HERMESCLAW_DB` — SQLite evidence database path; defaults to `hermesclaw.db`.
- `HERMESCLAW_BIND` — HTTP bind address; defaults to `127.0.0.1:7777`.

Then run the web cockpit:

```bash
cd apps/web
npm ci
npm run dev
```

Vite proxies `/api` to the proof API during web development.

## Desktop

The Tauri shell uses `invoke` commands directly against the same Rust `TurnCoordinator` and SQLite evidence store, so the desktop path does not require a second HermesClaw backend process.

```bash
cd apps/web
npm ci
cd ../..
npm --prefix apps/web run build
npm --prefix apps/web exec tauri -- --config ../desktop/src-tauri/tauri.conf.json
```

Linux desktop builds require the normal Tauri GTK/WebKit development packages.

## Verification

```bash
cargo fmt --all --check
cargo test --workspace --exclude hermesclaw-desktop
cargo clippy --workspace --exclude hermesclaw-desktop --all-targets -- -D warnings
npm --prefix apps/web ci
npm run test:web
npm run build:web
```

The desktop crate is additionally compile-checked in an environment with Tauri system libraries.

## Canonical project memory

Durable project facts, decisions, current state, security rules, and handoff information live in [`source/`](source/README_SOURCE.md). Read `source/00_PROJECT_CONSTITUTION.md` through `source/03_CURRENT_STATE.md` in order before substantive work. RAGLite under `source/raglite/` is a disposable retrieval index; the Markdown corpus remains canonical.

## Upstream provenance

The frozen reference baselines are documented in `THIRD_PARTY_NOTICES.md` and the design specification. No Hermes Agent or OpenClaw source file has been copied into this vertical proof; the current implementation is clean-room integration code guided by documented behavior and public interfaces.
