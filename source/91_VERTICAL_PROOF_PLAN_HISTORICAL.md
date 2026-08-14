> **STATUS: HISTORICAL IMPLEMENTATION PLAN FROM A PREVIOUS SESSION.** Do not interpret checkbox state or described commands as current repository truth. Consult `03_CURRENT_STATE.md` and live GitHub/CI evidence.

# HermesClaw Vertical Proof v1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first executable HermesClaw architectural spine: user request -> Rust agent loop -> deterministic model -> typed safe tool call -> centralized policy -> Rust tool execution -> SQLite evidence -> mission completion -> HTTP API -> React mission/evidence UI.

**Architecture:** A small Rust workspace owns all canonical backend state and policy. The first slice intentionally implements one model protocol, one safe workspace-list capability, one mission lifecycle, one SQLite evidence repository, and one API route. A React UI consumes the API; Tauri is only the desktop shell. Hermes/OpenClaw remain frozen references and are not runtime dependencies for this slice.

**Tech Stack:** Rust 2021, Tokio, Axum, Serde, async-trait, UUID, rusqlite (bundled SQLite), reqwest (OpenAI-compatible adapter), React 19, TypeScript, Vite, Tauri 2, GitHub Actions.

## Global Constraints

- Rust is the canonical backend destination from day one.
- React/TypeScript remains the canonical UI stack.
- Python and Node backend runtimes are temporary compatibility/reference dependencies only.
- HermesClaw owns one canonical state/security/runtime architecture.
- Migration is capability-based and differential-test-driven.
- A new HermesClaw UI is built rather than reskinning OpenClaw.
- The first implementation target is the minimal end-to-end architectural proof in Section 29 of the design.
- Upstream code is reused only with preserved license notices and tracked provenance.
- No crate may bypass `hc-policy` to execute an externally consequential capability.
- A capability is not “ported” merely because code compiles; executable tests and evidence gates are required.

---

## File Map

- `Cargo.toml` — Rust workspace and shared dependency versions.
- `crates/hc-domain` — IDs, provenance, mission/tool/evidence schemas.
- `crates/hc-policy` — observe/assist/autonomous policy decision for typed capabilities.
- `crates/hc-state` — SQLite evidence repository.
- `crates/hc-tools` — capability registry and safe `workspace.list` implementation.
- `crates/hc-models` — model provider trait, deterministic provider, OpenAI-compatible adapter.
- `crates/hc-mission` — minimal validated mission state machine.
- `crates/hc-agent` — turn coordinator binding model, policy, tools, evidence, mission.
- `crates/hc-api` — HTTP service with chat, evidence, and health endpoints.
- `apps/web` — new chat-first React interface with mission status and evidence disclosure.
- `apps/desktop/src-tauri` — minimal Tauri 2 shell for the web UI.
- `.github/workflows/ci.yml` — Rust and web proof gates.

---

### Task 1: Canonical domain contracts and policy gate

**Files:**
- Create: `Cargo.toml`
- Create: `crates/hc-domain/Cargo.toml`
- Create: `crates/hc-domain/src/lib.rs`
- Create: `crates/hc-policy/Cargo.toml`
- Create: `crates/hc-policy/src/lib.rs`

**Interfaces:**
- Produces: `TraceId`, `MissionId`, `TrustLevel`, `Provenance`, `MissionState`, `RiskClass`, `SideEffectClass`, `ToolCall`, `ToolResult`, `EvidenceRecord`, `AutonomyProfile`, `PolicyDecision`.
- Produces: `PolicyKernel::evaluate(profile: AutonomyProfile, call: &ToolCall) -> PolicyDecision`.

- [ ] **Step 1: Write failing domain/policy tests**

In `hc-domain`, require JSON round-trip for `ToolCall`. In `hc-policy`, require Observe to allow `workspace.list` with `RiskClass::Low` + `SideEffectClass::None` and deny a mutating call.

```rust
#[test]
fn observe_allows_low_risk_read_only_tool() {
    let call = ToolCall::workspace_list("call-1", ".");
    assert_eq!(PolicyKernel::evaluate(AutonomyProfile::Observe, &call), PolicyDecision::Allow);
}
```

- [ ] **Step 2: Run RED**

Run: `cargo test -p hc-domain -p hc-policy`
Expected: compile/test failure because the contracts and policy implementation do not yet exist.

- [ ] **Step 3: Implement minimal contracts and policy**

Implement strongly typed serde structures. `ToolCall` carries `capability_id`, `arguments`, `risk`, `side_effect`, and `provenance`; the policy decision is pure and deterministic.

- [ ] **Step 4: Run GREEN**

Run: `cargo test -p hc-domain -p hc-policy`
Expected: all tests pass.

- [ ] **Step 5: Commit**

`git commit -m "feat: establish domain contracts and policy kernel"`

---

### Task 2: Mission state machine and SQLite evidence repository

**Files:**
- Create: `crates/hc-mission/Cargo.toml`
- Create: `crates/hc-mission/src/lib.rs`
- Create: `crates/hc-state/Cargo.toml`
- Create: `crates/hc-state/src/lib.rs`

**Interfaces:**
- Produces: `Mission::new(objective)`, `Mission::transition(next) -> Result<(), MissionError>`.
- Produces: `EvidenceStore::open(path)`, `EvidenceStore::in_memory()`, `append(record)`, `list_by_trace(trace_id)`.

- [ ] **Step 1: Write failing mission/state tests**

Require `Created -> Executing -> Verifying -> Completed` to succeed, `Completed -> Executing` to fail, and an evidence record appended to SQLite to be retrievable by trace ID after serialization.

- [ ] **Step 2: Run RED**

Run: `cargo test -p hc-mission -p hc-state`
Expected: failure because mission/store behavior is missing.

- [ ] **Step 3: Implement minimal mission transitions and SQLite schema**

Create `evidence` table with trace ID, mission ID, kind, capability ID, policy decision, status, sanitized JSON payload, and timestamp. Use transactions for writes.

- [ ] **Step 4: Run GREEN**

Run: `cargo test -p hc-mission -p hc-state`
Expected: all tests pass.

- [ ] **Step 5: Commit**

`git commit -m "feat: add durable mission evidence state"`

---

### Task 3: Capability registry and safe workspace listing

**Files:**
- Create: `crates/hc-tools/Cargo.toml`
- Create: `crates/hc-tools/src/lib.rs`

**Interfaces:**
- Produces: async `Capability` trait.
- Produces: `CapabilityRegistry::register`, `CapabilityRegistry::execute`.
- Produces: `WorkspaceListCapability::new(workspace_root)` implementing capability ID `workspace.list`.

- [ ] **Step 1: Write failing tests**

Create a temporary workspace with two files. Require `workspace.list` on `.` to return sorted relative entries. Require `../` and symlink escapes outside the workspace to be rejected.

- [ ] **Step 2: Run RED**

Run: `cargo test -p hc-tools`
Expected: failure because capability registry/list implementation is missing.

- [ ] **Step 3: Implement minimal capability broker**

Canonicalize target paths, require them to remain beneath the canonical workspace root, reject escape attempts, list entries without reading file contents, and return typed JSON output.

- [ ] **Step 4: Run GREEN**

Run: `cargo test -p hc-tools`
Expected: all tests pass.

- [ ] **Step 5: Commit**

`git commit -m "feat: add policy-ready workspace capability"`

---

### Task 4: Model Fabric seed adapters

**Files:**
- Create: `crates/hc-models/Cargo.toml`
- Create: `crates/hc-models/src/lib.rs`

**Interfaces:**
- Produces: async `ModelProvider::next_turn(&self, request: ModelRequest) -> Result<ModelOutput, ModelError>`.
- Produces: `DeterministicProvider` for repeatable tests.
- Produces: `OpenAiCompatibleProvider::new(base_url, api_key, model)`.

- [ ] **Step 1: Write failing tests**

Require the deterministic provider to emit exactly one `workspace.list` tool call followed by a final response after receiving the tool result. Require the OpenAI codec to parse a fixture containing `tool_calls[0].function.name == "workspace.list"` into the same canonical `ToolCall`.

- [ ] **Step 2: Run RED**

Run: `cargo test -p hc-models`
Expected: failure because providers/codecs do not yet exist.

- [ ] **Step 3: Implement model contracts and adapters**

Use provider-neutral internal messages. The real adapter uses `reqwest` for `/chat/completions`, validates HTTP status, parses structured tool calls, and never executes tools itself.

- [ ] **Step 4: Run GREEN**

Run: `cargo test -p hc-models`
Expected: all tests pass without requiring external API credentials.

- [ ] **Step 5: Commit**

`git commit -m "feat: seed model fabric with deterministic and openai adapters"`

---

### Task 5: Minimal Agent Engine vertical coordinator

**Files:**
- Create: `crates/hc-agent/Cargo.toml`
- Create: `crates/hc-agent/src/lib.rs`

**Interfaces:**
- Produces: `TurnCoordinator::run(ChatInput) -> Result<ChatOutcome, AgentError>`.
- Consumes: `ModelProvider`, `PolicyKernel`, `CapabilityRegistry`, `EvidenceStore`, `Mission`.

- [ ] **Step 1: Write failing integration test**

Given a deterministic provider and a workspace containing `alpha.txt`, require one chat turn to: create a mission, transition to executing, obtain `workspace.list`, pass Observe policy, execute it, record evidence, transition through verifying to completed, and return a response naming `alpha.txt`.

- [ ] **Step 2: Run RED**

Run: `cargo test -p hc-agent`
Expected: failure because the coordinator does not exist.

- [ ] **Step 3: Implement the smallest bounded loop**

Maximum four model iterations. Each tool call must pass schema/risk metadata and policy before registry dispatch. Record policy + execution evidence before continuing. No direct filesystem access from `hc-agent`.

- [ ] **Step 4: Run GREEN**

Run: `cargo test -p hc-agent`
Expected: the end-to-end core test passes.

- [ ] **Step 5: Commit**

`git commit -m "feat: implement first hermesclaw agent turn"`

---

### Task 6: HTTP API and executable proof service

**Files:**
- Create: `crates/hc-api/Cargo.toml`
- Create: `crates/hc-api/src/lib.rs`
- Create: `crates/hc-api/src/main.rs`
- Create: `crates/hc-api/tests/vertical_slice.rs`

**Interfaces:**
- `GET /health -> {"status":"ok"}`.
- `POST /api/chat` body `{ "message": string }` -> `{ trace_id, mission_id, mission_state, response }`.
- `GET /api/evidence/:trace_id` -> ordered evidence records.

- [ ] **Step 1: Write failing HTTP contract test**

Use Axum router `oneshot` with a deterministic provider and temporary workspace. Assert 200 response, `mission_state == "completed"`, then query evidence and assert at least one `workspace.list` execution record.

- [ ] **Step 2: Run RED**

Run: `cargo test -p hc-api --test vertical_slice`
Expected: failure because API routes/service wiring are missing.

- [ ] **Step 3: Implement router/service**

Build application state from injected coordinator/evidence store. Return structured errors; never expose secrets or internal panic text.

- [ ] **Step 4: Run GREEN**

Run: `cargo test -p hc-api --test vertical_slice`
Expected: pass.

- [ ] **Step 5: Commit**

`git commit -m "feat: expose vertical proof through hermesclaw api"`

---

### Task 7: New React UI and Tauri shell

**Files:**
- Create: `package.json`
- Create: `apps/web/package.json`
- Create: `apps/web/tsconfig.json`
- Create: `apps/web/vite.config.ts`
- Create: `apps/web/index.html`
- Create: `apps/web/src/main.tsx`
- Create: `apps/web/src/App.tsx`
- Create: `apps/web/src/app.css`
- Create: `apps/web/src/api.ts`
- Create: `apps/web/src/App.test.tsx`
- Create: `apps/desktop/src-tauri/Cargo.toml`
- Create: `apps/desktop/src-tauri/build.rs`
- Create: `apps/desktop/src-tauri/src/main.rs`
- Create: `apps/desktop/src-tauri/tauri.conf.json`

**Interfaces:**
- UI posts to `/api/chat`, renders response, mission status, trace ID, and an expandable evidence panel.
- Autonomy state is visibly `Observe` in the first slice.

- [ ] **Step 1: Write failing UI test**

Mock the API boundary only. Render `App`, submit “List this workspace”, return a completed response and evidence fixture, and assert the UI shows `Completed`, `Observe`, `workspace.list`, and the returned response.

- [ ] **Step 2: Run RED**

Run: `npm ci && npm run test:web`
Expected: failing test because UI components do not exist.

- [ ] **Step 3: Implement minimal premium chat-first UI**

Use semantic HTML, keyboard-accessible form controls, visible autonomy state, mission status, and progressive-disclosure evidence. Avoid dashboard clutter.

- [ ] **Step 4: Run GREEN and build**

Run: `npm run test:web && npm run build:web`
Expected: tests and production build pass.

- [ ] **Step 5: Commit**

`git commit -m "feat: add hermesclaw mission cockpit ui"`

---

### Task 8: CI proof gates and final verification

**Files:**
- Create: `.github/workflows/ci.yml`
- Modify: `README.md`
- Modify: `THIRD_PARTY_NOTICES.md` only if derived upstream code was actually introduced.

**Interfaces:**
- CI is authoritative for this environment because local network isolation prevents installing the Rust toolchain and fetching crates.

- [ ] **Step 1: Add CI workflow**

Rust job: stable toolchain, `cargo fmt --check`, `cargo test --workspace --exclude hermesclaw-desktop`, `cargo clippy --workspace --exclude hermesclaw-desktop --all-targets -- -D warnings`.

Web job: Node 22, `npm ci`, `npm run test:web`, `npm run build:web`.

- [ ] **Step 2: Run/observe CI**

Push the isolated feature branch to the private CI mirror and wait for both jobs. Any failure blocks merge.

- [ ] **Step 3: Repair until green**

For each failure: reproduce from logs, fix root cause, push a new commit, rerun complete gates.

- [ ] **Step 4: Verify merged result**

Merge the feature branch locally into `main`, then run every locally available check and confirm the exact feature commit passed the remote Rust/web CI before branch cleanup.

- [ ] **Step 5: Final commit/branch cleanup**

Delete the merged local feature branch only after the green merged state is confirmed.

