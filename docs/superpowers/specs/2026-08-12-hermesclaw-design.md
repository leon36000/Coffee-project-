# HermesClaw — Architecture & Product Design Specification

**Status:** Design baseline for review  
**Date:** 2026-08-12  
**Working product name:** HermesClaw  
**Target:** A single autonomous agent application that absorbs the strongest capabilities of Hermes Agent and OpenClaw into a new Rust-first core with a new React/TypeScript interface.

## 1. Executive decision

HermesClaw will **not** ship as “Hermes + OpenClaw connected together.” It will be a new product with one identity, one state model, one security kernel, one agent runtime, one scheduler, one tool system, one UI, and one distribution.

The implementation strategy is a **progressive behavioral migration**:

1. Preserve Hermes Agent and OpenClaw temporarily as reference implementations and compatibility sources.
2. Define stable HermesClaw behavioral contracts in Rust.
3. Build a thin compatibility boundary around each upstream capability only while it is still needed.
4. Reimplement capabilities in the Rust core by vertical slice.
5. Run differential tests against the upstream implementation.
6. Remove each compatibility dependency only after behavior, safety, and performance gates pass.
7. Keep the desktop/web interface in React + TypeScript; do not rewrite UI logic in Rust without a measured reason.

This avoids the wasteful path `Python -> TypeScript -> Rust` for Hermes and avoids rewriting OpenClaw wholesale before HermesClaw can run.

## 2. Source baseline

Initial upstream snapshots used for behavioral analysis:

| Upstream | Version | Snapshot commit | Primary role in migration |
|---|---:|---|---|
| NousResearch/hermes-agent | 0.20.0 | `ee472a7fdbbc55924f91ab122dbaa29bd07668b0` | cognition, agent loop, memory, skills, delegation, learning, tool behavior |
| openclaw/openclaw | 2026.8.1 | `42ad83142ee97b387c3d8ca7ee38dd1eadb668d6` | gateway, channels, plugins, nodes, browser/computer action, sessions, automation, delivery |

Both upstream projects declare MIT licensing in their package/project metadata. HermesClaw must preserve applicable copyright and license notices for reused or derived code and maintain a `THIRD_PARTY_NOTICES.md` inventory.

The baseline commits are frozen for reproducibility. Upstream updates are evaluated explicitly rather than silently changing the migration target.

## 3. Product vision

HermesClaw is a **persistent autonomous computer agent** with a conversational center and an inspectable mission cockpit.

From the user's point of view there is only one product:

- one installer;
- one application;
- one account/profile;
- one memory;
- one skill library;
- one agent runtime;
- one permission model;
- one mission history;
- one automation system;
- one device/network view;
- one UI;
- one update mechanism.

Internal compatibility processes may exist during migration, but they are implementation details and are progressively eliminated.

## 4. Non-goals

HermesClaw v1 must not:

- mechanically translate every upstream source file;
- preserve duplicate schedulers, memories, session stores, agent loops, or tool registries;
- require Python for the final core runtime;
- require Node.js for the final backend runtime;
- expose raw upstream concepts to users when a unified HermesClaw concept can replace them;
- chase 100% obscure upstream feature parity before the core product is useful;
- treat model output as trusted authorization;
- hide failed tests behind “best effort” completion claims.

## 5. Architecture principles

### 5.1 One canonical owner per concern

Every durable concern has exactly one canonical owner:

- agent lifecycle -> HermesClaw Agent Engine;
- tool invocation -> Capability Engine;
- permissions -> Policy Kernel;
- durable missions -> Mission Engine;
- memory -> Memory Engine;
- schedules -> Automation Engine;
- model access -> Model Fabric;
- channels/delivery -> Gateway Engine;
- observability/evidence -> Evidence Engine;
- device execution -> Node Engine;
- UI state -> projections from canonical backend state.

Compatibility adapters may translate, never own canonical state.

### 5.2 Behavior before source syntax

Migration unit = capability/behavior, not file.

For each upstream feature:

`inventory -> contract -> fixtures -> Rust implementation -> differential test -> security test -> benchmark -> cutover`

### 5.3 Typed boundaries

All cross-component messages are typed Rust data structures with stable serialization schemas. Unstructured model text is never allowed to become an executable action without parsing, validation, policy evaluation, and capability dispatch.

### 5.4 Provenance is first-class

Every external input is tagged with provenance and trust level, including:

- user-authored text;
- web content;
- email/message content;
- voice transcript;
- file content;
- tool output;
- model-generated content;
- agent-generated instructions.

Policy decisions can use provenance. Untrusted content cannot silently escalate privileges.

### 5.5 Local-first, remote-capable

The same Rust core runs as:

- desktop application backend;
- headless daemon;
- server/gateway instance;
- remote node worker with constrained capabilities.

The UI communicates through stable application APIs instead of importing backend internals.

## 6. Target technology stack

### Canonical backend

**Rust** for:

- agent runtime;
- mission state machines;
- provider/model abstraction;
- tool/capability registry;
- scheduler;
- memory orchestration;
- policy/approval engine;
- secure process execution broker;
- browser/device action broker;
- gateway core;
- node protocol;
- event/audit infrastructure;
- persistence services;
- plugin host contracts;
- telemetry.

### UI

**React + TypeScript** for:

- chat;
- mission cockpit;
- agents;
- memory;
- skills;
- browser/computer views;
- automations;
- devices;
- channels;
- models;
- approvals;
- evidence/traces;
- settings.

### Desktop shell

**Tauri** is the preferred desktop shell unless a later proof demonstrates a better fit.

### Persistent storage

Initial local canonical store: **SQLite** behind repository traits. Human-readable memory/skill files can be exported/projected, but they are not competing canonical databases.

A server-grade storage implementation may later add PostgreSQL without changing domain contracts.

### Temporary migration runtimes

- Python: only for Hermes compatibility/reference execution.
- Node/TypeScript: only for OpenClaw compatibility/reference execution and the React build toolchain.

The final backend must not require either runtime for ordinary operation.

## 7. Top-level target architecture

```text
                         HERMESCLAW
  ----------------------------------------------------------------
  React/TypeScript UI                CLI / Remote API / Channels
            |                                  |
            +----------------+-----------------+
                             |
                      HermesClaw API
                             |
                    +--------v---------+
                    |  Mission Engine  |
                    +--------+---------+
                             |
       +---------------------+----------------------+
       |                     |                      |
+------v------+       +------v------+        +------v------+
| Agent Engine|       | Capability  |        | Memory      |
|             |       | Engine      |        | Engine      |
+------+------+       +------+------+        +------+------+
       |                     |                      |
       +----------+----------+----------+-----------+
                  |                     |
           +------v------+       +------v------+
           | Model Fabric|       | Policy Kernel|
           +------+------+       +------+------+
                  |                     |
       +----------+----------+----------+-----------+
       |                     |                      |
+------v------+       +------v------+        +------v------+
| Gateway     |       | Automation  |        | Node Engine |
| Engine      |       | Engine      |        |             |
+------+------+       +------+------+        +------+------+
       |                     |                      |
       +---------------------+----------------------+
                             |
                      Evidence Engine
                             |
                  Canonical State + Audit
```

## 8. Core Rust workspace

Proposed crate boundaries:

```text
crates/
  hc-domain/          # identifiers, schemas, domain events, trust/provenance
  hc-agent/           # agent turn loop, delegation, context orchestration
  hc-mission/         # durable mission lifecycle and state machines
  hc-models/          # model/provider traits, streaming, usage, tool-call codecs
  hc-tools/           # tool contracts, registry, execution lifecycle
  hc-policy/          # capabilities, approvals, risk policy, provenance checks
  hc-memory/          # memory retrieval/write policy, sessions, summaries
  hc-skills/          # skill format, discovery, validation, lifecycle
  hc-automation/      # schedules, heartbeats, durable recurring work
  hc-gateway/         # inbound/outbound routing and channel-neutral delivery
  hc-nodes/           # remote worker/device protocol and capability advertisement
  hc-browser/         # browser action abstraction
  hc-computer/        # desktop/computer-use abstraction
  hc-process/         # shell/process execution broker
  hc-state/           # SQLite repositories, migrations, transactions
  hc-events/          # typed internal event bus and durable audit events
  hc-evidence/        # proofs, test/action evidence, trace correlation
  hc-observability/   # logs, metrics, traces, OpenTelemetry
  hc-api/             # local/remote API contracts
  hc-compat-hermes/   # TEMPORARY Python reference/compatibility adapter
  hc-compat-openclaw/ # TEMPORARY OpenClaw reference/compatibility adapter
```

No crate may bypass `hc-policy` to execute an externally consequential capability.

## 9. Unified Agent Engine

HermesClaw absorbs the behavioral strengths of Hermes rather than copying its large Python `AIAgent` class as one Rust monolith.

The Rust agent loop is decomposed into explicit services:

1. `TurnCoordinator`
2. `ContextAssembler`
3. `ProviderDispatcher`
4. `ToolCallParser`
5. `CapabilityDispatcher`
6. `IterationBudget`
7. `FallbackPolicy`
8. `CompressionPolicy`
9. `MemoryFlushPolicy`
10. `DelegationCoordinator`
11. `CancellationController`
12. `TurnEvidenceRecorder`

Canonical turn lifecycle:

```text
receive input
  -> establish provenance
  -> load mission/session state
  -> retrieve relevant memory/skills
  -> assemble bounded context
  -> choose provider/model
  -> stream model response
  -> parse typed tool calls
  -> policy evaluate each capability
  -> execute approved capabilities
  -> record results/evidence
  -> repeat until completion/budget/cancel
  -> validate completion
  -> persist state and memory
  -> deliver response
```

Provider-specific message formats are adapters. Internal conversation and tool-call structures stay provider-neutral.

## 10. Model Fabric

The Model Fabric presents one model contract regardless of provider.

Minimum capabilities:

- OpenAI-compatible chat/responses endpoints;
- Anthropic-style messages;
- local OpenAI-compatible inference servers such as llama.cpp/vLLM/SGLang gateways;
- streaming;
- reasoning metadata where available;
- tool/function calling;
- structured output;
- usage/cost accounting;
- cancellation;
- retry/fallback;
- model capability profiles;
- per-task routing policy.

Model selection is policy-driven, not hardcoded throughout the runtime.

## 11. Mission Engine

A **Mission** is the primary durable unit of autonomous work.

Mission states:

`created -> planning -> executing -> waiting_approval | waiting_external | verifying -> completed | failed | cancelled`

A mission owns:

- objective;
- user constraints;
- workspace;
- parent/child agents;
- budgets;
- allowed capabilities;
- artifacts;
- actions;
- approvals;
- evidence;
- tests;
- final result;
- resumable checkpoint state.

A chat request may remain a simple conversation or be promoted into a Mission automatically when it requires multi-step durable execution.

## 12. Tool and Capability Engine

HermesClaw separates **what an agent wants to do** from **whether and how it can do it**.

A tool declaration contains:

- stable capability ID;
- typed input schema;
- typed output schema;
- risk class;
- side-effect class;
- reversibility metadata;
- timeout;
- concurrency policy;
- required grants;
- provenance constraints;
- executor.

Execution pipeline:

```text
model tool call
 -> schema validation
 -> canonicalization
 -> risk classification
 -> policy evaluation
 -> optional human approval
 -> isolated execution
 -> output sanitation
 -> evidence capture
 -> typed result
```

Initial capability families:

- filesystem read/write/patch;
- process/shell;
- browser observe/act;
- computer observe/act;
- network fetch/search;
- messages/send;
- schedules/automation;
- device/node execution;
- code-agent delegation;
- model invocation;
- memory read/write;
- skill read/execute/create/update.

## 13. Policy Kernel

Policy is mandatory and centralized.

Three default autonomy profiles:

### Observe

Read/search/analyze only. No consequential external side effects.

### Assist

Reversible low-risk actions are allowed; sensitive actions require approval.

### Autonomous Scoped

Actions may execute automatically only within explicitly granted scopes, budgets, machines, directories, repositories, channels, and time windows.

Policy inputs include:

- actor;
- mission;
- capability;
- arguments;
- target resource;
- provenance/trust;
- risk class;
- current autonomy profile;
- inherited grants;
- budget;
- prior approvals;
- environment.

The LLM is never the authority that approves its own privilege escalation.

## 14. Memory Engine

HermesClaw uses one canonical memory system with explicit categories:

- user profile/preference memory;
- project/workspace memory;
- semantic long-term memory;
- episodic mission memory;
- procedural/skill memory;
- session transcript;
- temporary working context.

Principles:

- every write has source/provenance;
- memories may have confidence and expiry;
- retrieval is budget-aware;
- sensitive memories can be scoped/encrypted;
- user-editable projections are supported;
- compaction never silently destroys canonical evidence;
- session summaries and long-term facts are distinct entities.

Hermes-style human-readable `MEMORY.md`/`USER.md` may be supported as import/export/projection formats rather than parallel truth stores.

## 15. Skill System

A HermesClaw Skill is a versioned, inspectable, testable package.

A skill contains:

- metadata and purpose;
- trigger/discovery descriptors;
- instructions;
- allowed capabilities;
- optional scripts/assets;
- tests/evaluations;
- provenance;
- version lineage;
- quality score;
- approval status.

Self-created or self-modified skills are staged, evaluated, and promoted through policy. Autonomous learning cannot silently overwrite trusted production skills.

Long-term extension direction: language-neutral plugin boundary using MCP/A2A and/or a WASI component interface, while TypeScript/Python plugin compatibility hosts remain temporary where useful.

## 16. Automation Engine

One scheduler only.

Supports:

- cron/interval schedules;
- one-shot schedules;
- heartbeat checks;
- condition watches;
- durable background missions;
- retry/backoff;
- missed-run policy;
- concurrency policy;
- idempotency keys;
- cancellation;
- approval-aware resume.

Scheduled work launches normal HermesClaw Missions. It does not run a second agent runtime.

## 17. Gateway and Channels

The Gateway Engine owns transport-neutral inbound/outbound messaging.

A channel adapter is responsible only for:

- authentication;
- translating inbound events into canonical HermesClaw envelopes;
- delivery capabilities;
- provider-specific media/reply semantics;
- transport retries/deduplication.

The Gateway owns:

- routing;
- session/mission association;
- identity resolution;
- replay protection;
- bot-loop protection;
- delivery queue;
- rate policy;
- trust/provenance labelling.

OpenClaw channel adapters can remain behind the compatibility layer until individually migrated.

## 18. Browser, Computer, Shell, and Nodes

All action surfaces are capability brokers behind the Policy Kernel.

### Browser

- inspect DOM/accessibility tree;
- screenshots;
- navigation;
- form actions;
- downloads/uploads;
- browser profiles;
- explicit credential boundaries.

### Computer

- screen observation;
- pointer/keyboard actions;
- app/window selection;
- screenshot evidence;
- guarded clipboard.

### Process/Shell

- explicit cwd/environment;
- process tree ownership;
- timeout/cancellation;
- sandbox profiles;
- allow/deny policies;
- output truncation with evidence references;
- secret redaction.

### Nodes

Remote nodes advertise capabilities and platform facts. The central runtime grants only a subset to a mission. Node identity and capability grants are cryptographically authenticated.

## 19. Evidence and observability

HermesClaw distinguishes regular logs from **evidence**.

Every mission receives a trace ID. Every consequential action records:

- who/what requested it;
- model and agent identity;
- input hash / sanitized parameters;
- policy decision;
- approval if any;
- executor;
- start/end status;
- relevant output/artifact hashes;
- errors;
- verification result.

Observability supports structured logs, metrics, distributed tracing, and OpenTelemetry export.

The UI can show a human-readable Evidence view without exposing hidden model chain-of-thought.

## 20. New HermesClaw interface

The UI is redesigned from first principles. It is not an OpenClaw reskin.

### Primary navigation

- Chat
- Missions
- Agents
- Memory
- Skills
- Computer
- Browser
- Automations
- Devices
- Channels
- Models
- Approvals
- Evidence
- Settings

### Chat remains the center

A normal user can operate HermesClaw entirely through chat.

When work becomes complex, the conversation exposes a Mission panel containing:

- current objective;
- plan/status;
- active agents;
- current action;
- touched files/resources;
- tests;
- approvals;
- evidence;
- artifacts;
- cost/token/resource budget;
- completion criteria.

### UI principles

- premium desktop feel;
- low visual noise;
- progressive disclosure;
- live streaming state without dashboard overload;
- strong distinction between “thinking/working”, “waiting approval”, “failed”, and “verified”;
- always-visible autonomy/security state;
- keyboard-first power-user paths;
- responsive web-compatible layout;
- accessibility as a release gate.

## 21. Desktop and headless distribution

One codebase supports:

```text
hermesclaw desktop   # Tauri application
hermesclaw chat      # CLI
hermesclaw serve     # headless/local network gateway
hermesclaw node      # constrained remote worker mode
```

These are modes of one product/core, not independent agents.

## 22. Compatibility strategy

### Hermes compatibility

Temporary reference runner executes frozen Hermes behaviors for:

- golden trace capture;
- differential testing;
- capability fallback while a Rust replacement is unfinished.

No new canonical state is allowed to originate solely inside the compatibility runtime without being projected back through HermesClaw contracts.

### OpenClaw compatibility

Temporary adapter hosts selected OpenClaw capabilities while Rust replacements are incomplete, especially channels, browser/computer integration, and specialized device surfaces.

The compatibility host is intentionally narrow. HermesClaw does not expose OpenClaw's internal state model as its own API.

## 23. Migration taxonomy

Every upstream capability is classified:

- **KEEP-AS-REFERENCE** — behavior remains only as a test oracle during migration.
- **ADAPT-TEMPORARILY** — run through a compatibility boundary.
- **PORT** — direct behavioral reimplementation in Rust.
- **MERGE** — combine overlapping Hermes/OpenClaw features into one superior HermesClaw component.
- **REDESIGN** — preserve user value but replace the architecture.
- **DROP** — duplicate, obsolete, low-value, unsafe, or out-of-scope behavior.
- **NEW** — HermesClaw-native capability.

A capability inventory must state its classification and evidence before implementation.

## 24. Differential migration harness

This is a core project asset, not an optional test script.

For each migrated capability:

```text
fixture / scenario
      +-------------------+
      |                   |
      v                   v
upstream reference     Rust HermesClaw
      |                   |
      +---------+---------+
                v
         semantic comparator
                |
     +----------+----------+
     |                     |
 equivalent             mismatch
     |                     |
 candidate gate        repair/retest
```

Comparison levels:

1. schema/API parity;
2. deterministic state transition parity;
3. observable output parity;
4. side-effect parity;
5. safety-policy parity or improvement;
6. failure/retry/cancel parity;
7. performance/resource regression threshold.

Exact byte equality is required only where it is meaningful. LLM-facing output uses semantic/structural assertions and recorded invariants.

## 25. Test strategy

### Unit

Pure domain logic, parsing, policy, reducers, state transitions.

### Contract

Provider, tool, channel, node, storage, plugin, and API contracts.

### Differential

Upstream vs HermesClaw scenarios.

### Integration

Multiple Rust components with real persistence and controlled external services.

### End-to-end

Message -> agent -> capability -> verification -> delivery through the actual app stack.

### Security

- prompt injection;
- path traversal;
- shell injection;
- SSRF;
- secret exfiltration;
- cross-session leakage;
- approval bypass;
- malicious tool output;
- untrusted content escalation;
- plugin compromise boundaries;
- replay/bot loops;
- node impersonation.

### Resilience

- process crash;
- forced restart;
- network loss;
- provider rate limits;
- partial tool failure;
- duplicate messages;
- cancelled missions;
- corrupt/nonexistent compatibility runtime.

### Performance

Benchmarks for idle memory, startup, message throughput, event latency, tool concurrency, mission recovery, database operations, and gateway fanout.

## 26. Definition of “ported”

A capability is not “ported” when code compiles.

It is ported only when:

- contract is documented;
- tests exist;
- Rust implementation passes tests;
- differential comparison passes where applicable;
- security checks pass;
- cancellation/error paths are proven;
- telemetry/evidence is present;
- migration/state compatibility is proven;
- performance is acceptable;
- the compatibility fallback can be disabled for that capability.

## 27. AI-agent development model

State-of-the-art coding agents are used heavily, but inside a constrained engineering loop.

Recommended roles:

- **Architect agent** — boundaries, contracts, ADRs.
- **Source analyst agents** — independent Hermes/OpenClaw capability extraction.
- **Rust implementation agents** — small isolated crates/features.
- **Test-oracle agent** — creates behavioral fixtures before implementation.
- **Security reviewer** — threat-models capability boundaries.
- **Differential verifier** — compares reference vs Rust.
- **Integration agent** — connects proven components.
- **UI agent** — React/Tauri UX implementation.
- **Independent judge** — rejects unsupported “done” claims.

Agents do not receive carte blanche to rewrite multiple domains at once. Work is partitioned by small contracts that fit within context and can be tested independently.

## 28. Engineering workflow guardrails

1. Freeze source snapshot for each migration wave.
2. Read upstream tests before touching implementation.
3. Capture behavioral fixtures.
4. Write HermesClaw contract tests.
5. Implement smallest Rust slice.
6. Run Rust unit/contract tests.
7. Run differential suite.
8. Run security checks.
9. Run integration/E2E tests.
10. Record evidence.
11. Cut over one capability.
12. Delete/disable compatibility path only after proof.

No “translated successfully” claim without executable evidence.

## 29. First vertical proof target

The first proof should be intentionally small but end-to-end:

> A user starts HermesClaw, sends a request, the Rust agent loop calls a model, obtains one typed low-risk tool call, passes policy, executes it through the Rust capability broker, records evidence in SQLite, updates mission state, streams progress to the React UI, and returns the verified result.

This slice deliberately proves the architectural spine before browser, channels, complex memory, or autonomous multi-agent behavior.

Required proof components:

- `hc-domain`
- `hc-events`
- `hc-state`
- `hc-policy`
- `hc-models`
- `hc-tools`
- minimal `hc-agent`
- minimal `hc-mission`
- `hc-api`
- minimal React/Tauri UI
- one deterministic test provider + one real OpenAI-compatible provider adapter
- one safe tool such as workspace file read/list
- trace/evidence view

## 30. Migration waves after architectural proof

The intended order is driven by dependency structure, not by upstream folders:

1. Core domain/events/state/policy.
2. Model Fabric and minimal Agent Engine.
3. Tool/Process/File capability engine.
4. Mission durability, cancellation, recovery.
5. Hermes memory/context/compression behavior.
6. Hermes skills/delegation/self-improvement controls.
7. Automation scheduler/heartbeat.
8. Gateway/session/delivery primitives.
9. Initial messaging channels.
10. Browser actions.
11. Computer-use and remote nodes.
12. Voice/media and advanced channels.
13. Compatibility retirement and final runtime simplification.

Each wave can be split further; no wave is accepted as a single giant agent task.

## 31. Risks and mitigations

### Risk: semantic drift during Python -> Rust migration

Mitigation: frozen reference snapshots, differential fixtures, upstream test mining.

### Risk: OpenClaw breadth is too large

Mitigation: capability inventory and value-based parity; migrate by vertical slice, not full repository.

### Risk: compatibility layers become permanent

Mitigation: every adapter has a retirement criterion and a visible dependency dashboard.

### Risk: AI agents produce plausible but incorrect ports

Mitigation: tests precede implementation, independent verifier, strict proof gates, small context-bounded tasks.

### Risk: plugin ecosystems force Node/Python forever

Mitigation: support compatibility hosts while introducing language-neutral interfaces; backend independence is a staged objective.

### Risk: autonomous agent security

Mitigation: centralized capabilities, provenance, deny-by-default sensitive operations, approvals, sandboxing, audit evidence.

### Risk: UI becomes an overloaded operations dashboard

Mitigation: chat-first UX, progressive disclosure, mission-centric views, user testing.

## 32. Feasibility judgement

**Feasible:** yes.

The project is difficult because it combines a large Python agent codebase and a much larger TypeScript/multi-platform gateway codebase, but the problem is decomposable. Modern coding agents are particularly strong at bounded source analysis, contract extraction, Rust porting, test generation, and iterative repair when supplied with executable feedback.

The project becomes unsafe/unreliable only if attempted as a single bulk translation or if agents are allowed to declare parity without differential and integration proof.

Confidence judgement:

- architectural proof/MVP spine: **high**;
- production-quality unified core for the highest-value capabilities: **high with disciplined staged migration**;
- literal 100% feature-for-feature replacement of every upstream edge case: **possible but not a sensible first success criterion**.

Success is defined by a superior coherent HermesClaw product, not by maximizing translated line count.

## 33. Design acceptance criteria

This design is accepted when the following decisions are approved:

1. Rust is the canonical backend destination from day one.
2. React/TypeScript remains the canonical UI stack.
3. Python and Node backend runtimes are temporary compatibility/reference dependencies only.
4. HermesClaw owns one canonical state/security/runtime architecture.
5. Migration is capability-based and differential-test-driven.
6. A new HermesClaw UI is built rather than reskinning OpenClaw.
7. The first implementation target is the minimal end-to-end architectural proof in Section 29.
8. Upstream code is reused only with preserved license notices and tracked provenance.

## 34. Immediate next artifact after approval

After this design is accepted, the next artifact is a **detailed implementation plan** that decomposes the first vertical proof into test-first, commit-sized tasks with exact files, commands, expected test results, and verification gates.
