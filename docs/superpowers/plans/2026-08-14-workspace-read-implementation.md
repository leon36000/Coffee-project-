# HermesClaw `workspace.read` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a workspace-confined, 64 KiB UTF-8 file-read capability whose full content is returned to the model while only path, byte count, and SHA-256 are persisted as evidence.

**Architecture:** Reuse one canonical `WorkspaceBoundary` for `workspace.list` and `workspace.read`. Change the capability return contract to separate model-visible `ToolResult` from sanitized `evidence_payload`, then wire the new tool through the domain constructor, Model Fabric, Agent Engine, Axum, Tauri, React, and canonical project memory.

**Tech Stack:** Rust 1.97.1, Tokio, serde/serde_json, sha2 0.10, hex 0.4, SQLite/rusqlite, Axum, React 19, TypeScript, Vitest, Tauri 2, GitHub Actions.

## Global Constraints

- Official repository: `leon36000/Coffee-project-`; implementation branch: `agent/workspace-read`.
- Rust `1.97.1` remains the declared minimum.
- `workspace.read` is `Low` risk, `None` side effect, and allowed by the existing Observe policy.
- Maximum accepted file size is exactly `65_536` bytes; `65_537` bytes must fail.
- Accepted content must be a regular file, valid UTF-8, and contain no NUL byte.
- Canonical path resolution must prevent parent traversal, absolute-path escape, and symlink escape.
- Model-visible output is `{path, content, bytes}`; persisted evidence is `{path, bytes, sha256}` and must never contain the file text.
- No process execution, writes, patches, deletion, partial reads, streaming, binary-file support, or OS-specific race hardening in this change.
- Every production behavior begins with a failing test that is observed to fail for the expected reason.
- Do not modify `02_CANONICAL_DECISIONS.md`; the approved design changes no durable architecture decision.
- At completion, update `03_CURRENT_STATE.md`, `09_ROADMAP.md`, `11_HANDOFF.md`, `SOURCE_MANIFEST.json`, and add one dated evidence document.

---

## File Map

- `crates/hc-domain/src/lib.rs` — canonical `ToolCall::workspace_read` constructor and contract test.
- `crates/hc-tools/src/lib.rs` — shared boundary, result/evidence split, read implementation, security tests.
- `crates/hc-tools/Cargo.toml`, root `Cargo.toml`, `Cargo.lock` — direct SHA-256/hex dependencies.
- `crates/hc-models/src/lib.rs` — deterministic scenarios, OpenAI tool declaration and codec tests.
- `crates/hc-agent/src/lib.rs` — sanitized evidence persistence and read mission test.
- `crates/hc-api/src/main.rs`, `crates/hc-api/tests/vertical_slice.rs` — registry and Axum E2E.
- `apps/desktop/src-tauri/src/lib.rs` — Tauri registry.
- `apps/web/src/App.tsx`, `apps/web/src/App.test.tsx` — read-focused copy and UI test.
- `source/03_CURRENT_STATE.md`, `source/09_ROADMAP.md`, `source/11_HANDOFF.md`, `source/SOURCE_MANIFEST.json` — durable memory write-back.
- `docs/evidence/2026-08-14-workspace-read-verification.md` — exact executable evidence.

---

### Task 1: Add the canonical domain constructor

**Files:**
- Modify: `crates/hc-domain/src/lib.rs`
- Test: `crates/hc-domain/src/lib.rs`

**Interfaces:**
- Consumes: existing `ToolCall::new`, `RiskClass`, `SideEffectClass`, `Provenance`, `TrustLevel`.
- Produces: `ToolCall::workspace_read(id, path) -> ToolCall` for models, tests, and callers.

- [ ] **Step 1: Write the failing constructor test**

Add to the existing `tests` module:

```rust
#[test]
fn workspace_read_constructor_is_low_risk_read_only() {
    let call = ToolCall::workspace_read("call-read", "docs/notes.md");

    assert_eq!(call.id, "call-read");
    assert_eq!(call.capability_id, "workspace.read");
    assert_eq!(call.arguments, serde_json::json!({"path": "docs/notes.md"}));
    assert_eq!(call.risk, RiskClass::Low);
    assert_eq!(call.side_effect, SideEffectClass::None);
    assert_eq!(call.provenance.source, "model");
    assert_eq!(call.provenance.trust, TrustLevel::ModelGenerated);
}
```

- [ ] **Step 2: Run the focused test and observe RED**

```bash
cargo test -p hc-domain workspace_read_constructor_is_low_risk_read_only
```

Expected: compilation failure because `ToolCall::workspace_read` does not exist.

- [ ] **Step 3: Implement the minimal constructor**

Add beside `workspace_list`:

```rust
pub fn workspace_read(id: impl Into<String>, path: impl Into<String>) -> Self {
    Self::new(
        id,
        "workspace.read",
        serde_json::json!({ "path": path.into() }),
        RiskClass::Low,
        SideEffectClass::None,
        Provenance::new("model", TrustLevel::ModelGenerated),
    )
}
```

- [ ] **Step 4: Run domain tests and observe GREEN**

```bash
cargo test -p hc-domain
```

Expected: all `hc-domain` tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/hc-domain/src/lib.rs
git commit -m "feat(domain): add workspace read tool contract"
```

---

### Task 2: Separate model results from persisted evidence

**Files:**
- Modify: `crates/hc-tools/src/lib.rs`
- Modify: `crates/hc-agent/src/lib.rs`
- Test: `crates/hc-tools/src/lib.rs`
- Test: `crates/hc-agent/src/lib.rs`

**Interfaces:**
- Consumes: `ToolResult`, `CapabilityRegistry`, current list capability and coordinator.
- Produces: `CapabilityExecution { result, evidence_payload }`; `CapabilityRegistry::execute(...) -> Result<CapabilityExecution, CapabilityError>`.

- [ ] **Step 1: Change the list test to require the new split**

Replace the final assertion in `workspace_list_returns_sorted_relative_entries` with:

```rust
assert_eq!(
    result.result.output,
    json!({"entries": ["alpha.txt", "zeta.txt"]})
);
assert_eq!(result.evidence_payload, result.result.output);
```

Add a focused structural test:

```rust
#[test]
fn capability_execution_can_hold_different_result_and_evidence() {
    let execution = CapabilityExecution {
        result: ToolResult {
            call_id: "call-1".into(),
            capability_id: "workspace.read".into(),
            output: json!({"content": "secret"}),
        },
        evidence_payload: json!({"sha256": "digest"}),
    };

    assert_ne!(execution.result.output, execution.evidence_payload);
}
```

- [ ] **Step 2: Run tools tests and observe RED**

```bash
cargo test -p hc-tools
```

Expected: compilation failure because `CapabilityExecution` and `result.result` do not exist.

- [ ] **Step 3: Implement the minimal capability return contract**

Add before the trait:

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct CapabilityExecution {
    pub result: ToolResult,
    pub evidence_payload: serde_json::Value,
}
```

Change the trait and registry signatures:

```rust
#[async_trait]
pub trait Capability: Send + Sync {
    fn id(&self) -> &'static str;
    async fn execute(&self, call: &ToolCall)
        -> Result<CapabilityExecution, CapabilityError>;
}

pub async fn execute(
    &self,
    call: &ToolCall,
) -> Result<CapabilityExecution, CapabilityError> {
    let capability = self
        .capabilities
        .get(&call.capability_id)
        .ok_or_else(|| CapabilityError::UnknownCapability(call.capability_id.clone()))?;
    capability.execute(call).await
}
```

Return this from `WorkspaceListCapability`:

```rust
let output = json!({ "entries": entries });
Ok(CapabilityExecution {
    result: ToolResult {
        call_id: call.id.clone(),
        capability_id: call.capability_id.clone(),
        output: output.clone(),
    },
    evidence_payload: output,
})
```

- [ ] **Step 4: Update the coordinator to compile against the split**

Replace the existing execution block with:

```rust
let execution = self.registry.execute(&call).await?;
self.evidence.append(&EvidenceRecord {
    trace_id,
    mission_id: mission.id(),
    kind: "capability_execution".into(),
    capability_id: Some(call.capability_id.clone()),
    policy_decision: Some(decision),
    status: "succeeded".into(),
    payload: execution.evidence_payload,
    recorded_at: Utc::now(),
})?;
messages.push(ModelMessage::ToolResult(execution.result));
```

- [ ] **Step 5: Run list/agent regression tests and observe GREEN**

```bash
cargo test -p hc-tools
cargo test -p hc-agent deterministic_turn_completes_with_policy_tool_and_evidence
```

Expected: existing list behavior remains green; list evidence still contains entries.

- [ ] **Step 6: Commit**

```bash
git add crates/hc-tools/src/lib.rs crates/hc-agent/src/lib.rs
git commit -m "refactor(tools): separate results from evidence"
```

---

### Task 3: Add the shared workspace boundary and secure read capability

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/hc-tools/Cargo.toml`
- Modify: `crates/hc-tools/src/lib.rs`
- Test: `crates/hc-tools/src/lib.rs`

**Interfaces:**
- Consumes: `CapabilityExecution`, `ToolCall::workspace_read`, Tokio file APIs.
- Produces: `WorkspaceBoundary`, `WorkspaceReadCapability::new`, `MAX_WORKSPACE_READ_BYTES`, stable read errors.

- [ ] **Step 1: Add dependencies but no production behavior**

Add to root `[workspace.dependencies]`:

```toml
hex = "0.4"
sha2 = "0.10"
```

Add to `crates/hc-tools/Cargo.toml` dependencies:

```toml
hex.workspace = true
sha2.workspace = true
```

Do not hand-edit registry package checksums. Cargo will update the existing `hc-tools` dependency list in `Cargo.lock`; `hex 0.4.3` and `sha2 0.10.9` are already present transitively.

- [ ] **Step 2: Add test helpers and all failing read/security tests**

Add imports:

```rust
use hc_domain::ToolCall;
use serde_json::{json, Value};
use std::fs;
use tempfile::tempdir;
```

Add a helper:

```rust
fn read_call(path: &str) -> ToolCall {
    ToolCall::workspace_read("call-read", path)
}
```

Add the success/evidence test:

```rust
#[tokio::test]
async fn workspace_read_returns_text_and_sanitized_evidence() {
    let workspace = tempdir().expect("workspace");
    fs::create_dir(workspace.path().join("docs")).unwrap();
    fs::write(workspace.path().join("docs/notes.md"), "hello").unwrap();

    let mut registry = CapabilityRegistry::new();
    registry.register(WorkspaceReadCapability::new(workspace.path()).unwrap());

    let execution = registry.execute(&read_call("docs/notes.md")).await.unwrap();

    assert_eq!(
        execution.result.output,
        json!({"path": "docs/notes.md", "content": "hello", "bytes": 5})
    );
    assert_eq!(
        execution.evidence_payload,
        json!({
            "path": "docs/notes.md",
            "bytes": 5,
            "sha256": "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        })
    );
    assert!(execution.evidence_payload.get("content").is_none());
}
```

Add parent and absolute escape tests:

```rust
#[tokio::test]
async fn workspace_read_rejects_parent_escape() {
    let workspace = tempdir().unwrap();
    let registry = registry_with_read(workspace.path());
    let error = registry.execute(&read_call("../secret.txt")).await.unwrap_err();
    assert_eq!(error.to_string(), "path escapes workspace root");
}

#[tokio::test]
async fn workspace_read_rejects_absolute_escape() {
    let workspace = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let secret = outside.path().join("secret.txt");
    fs::write(&secret, "secret").unwrap();
    let registry = registry_with_read(workspace.path());
    let error = registry
        .execute(&read_call(secret.to_str().unwrap()))
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "path escapes workspace root");
}
```

Add file-type/content-limit tests:

```rust
#[tokio::test]
async fn workspace_read_rejects_directory() {
    let workspace = tempdir().unwrap();
    fs::create_dir(workspace.path().join("docs")).unwrap();
    let registry = registry_with_read(workspace.path());
    let error = registry.execute(&read_call("docs")).await.unwrap_err();
    assert_eq!(error.to_string(), "target path is not a regular file");
}

#[tokio::test]
async fn workspace_read_rejects_file_larger_than_limit() {
    let workspace = tempdir().unwrap();
    fs::write(
        workspace.path().join("large.txt"),
        vec![b'a'; MAX_WORKSPACE_READ_BYTES + 1],
    )
    .unwrap();
    let registry = registry_with_read(workspace.path());
    let error = registry.execute(&read_call("large.txt")).await.unwrap_err();
    assert_eq!(
        error.to_string(),
        "file exceeds 65536 byte limit: 65537 bytes"
    );
}

#[tokio::test]
async fn workspace_read_rejects_invalid_utf8() {
    let workspace = tempdir().unwrap();
    fs::write(workspace.path().join("binary.dat"), [0xff, 0xfe]).unwrap();
    let registry = registry_with_read(workspace.path());
    let error = registry.execute(&read_call("binary.dat")).await.unwrap_err();
    assert_eq!(error.to_string(), "file is not valid UTF-8 text");
}

#[tokio::test]
async fn workspace_read_rejects_nul_content() {
    let workspace = tempdir().unwrap();
    fs::write(workspace.path().join("nul.txt"), b"a\0b").unwrap();
    let registry = registry_with_read(workspace.path());
    let error = registry.execute(&read_call("nul.txt")).await.unwrap_err();
    assert_eq!(error.to_string(), "file contains a NUL byte");
}
```

Add Unix symlink tests:

```rust
#[cfg(unix)]
#[tokio::test]
async fn workspace_read_rejects_symlink_escape() {
    use std::os::unix::fs::symlink;
    let workspace = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let secret = outside.path().join("secret.txt");
    fs::write(&secret, "secret").unwrap();
    symlink(&secret, workspace.path().join("outside-link")).unwrap();
    let registry = registry_with_read(workspace.path());
    let error = registry.execute(&read_call("outside-link")).await.unwrap_err();
    assert_eq!(error.to_string(), "path escapes workspace root");
}

#[cfg(unix)]
#[tokio::test]
async fn workspace_read_accepts_internal_symlink_and_reports_canonical_path() {
    use std::os::unix::fs::symlink;
    let workspace = tempdir().unwrap();
    fs::write(workspace.path().join("real.txt"), "hello").unwrap();
    symlink(workspace.path().join("real.txt"), workspace.path().join("alias.txt")).unwrap();
    let registry = registry_with_read(workspace.path());
    let execution = registry.execute(&read_call("alias.txt")).await.unwrap();
    assert_eq!(execution.result.output["path"], "real.txt");
}
```

Use this helper:

```rust
fn registry_with_read(workspace: &Path) -> CapabilityRegistry {
    let mut registry = CapabilityRegistry::new();
    registry.register(WorkspaceReadCapability::new(workspace).unwrap());
    registry
}
```

- [ ] **Step 3: Run focused tools tests and observe RED**

```bash
cargo test -p hc-tools workspace_read
```

Expected: compilation failures because the boundary, read capability, limit, and errors do not exist.

- [ ] **Step 4: Implement `WorkspaceBoundary` and migrate list**

Add:

```rust
#[derive(Clone, Debug)]
pub struct WorkspaceBoundary {
    workspace_root: PathBuf,
}

impl WorkspaceBoundary {
    pub fn new(workspace_root: impl AsRef<Path>) -> Result<Self, CapabilityError> {
        let workspace_root = std::fs::canonicalize(workspace_root)?;
        if !workspace_root.is_dir() {
            return Err(CapabilityError::WorkspaceRootNotDirectory);
        }
        Ok(Self { workspace_root })
    }

    pub fn resolve_existing(&self, requested: &str) -> Result<PathBuf, CapabilityError> {
        if requested.is_empty() {
            return Err(CapabilityError::InvalidArguments);
        }
        let canonical = std::fs::canonicalize(self.workspace_root.join(requested))?;
        if !canonical.starts_with(&self.workspace_root) {
            return Err(CapabilityError::PathEscapesWorkspace);
        }
        Ok(canonical)
    }

    pub fn relative_path(&self, canonical: &Path) -> Result<String, CapabilityError> {
        let relative = canonical
            .strip_prefix(&self.workspace_root)
            .map_err(|_| CapabilityError::PathEscapesWorkspace)?;
        Ok(relative.to_string_lossy().replace('\\', "/"))
    }
}
```

Change list to store `boundary: WorkspaceBoundary`, resolve through it, and call `boundary.relative_path(&entry.path())` for entries.

- [ ] **Step 5: Implement `WorkspaceReadCapability`**

Add imports and implementation:

```rust
use sha2::{Digest, Sha256};

pub const MAX_WORKSPACE_READ_BYTES: usize = 64 * 1024;

pub struct WorkspaceReadCapability {
    boundary: WorkspaceBoundary,
}

impl WorkspaceReadCapability {
    pub fn new(workspace_root: impl AsRef<Path>) -> Result<Self, CapabilityError> {
        Ok(Self {
            boundary: WorkspaceBoundary::new(workspace_root)?,
        })
    }
}

#[async_trait]
impl Capability for WorkspaceReadCapability {
    fn id(&self) -> &'static str {
        "workspace.read"
    }

    async fn execute(&self, call: &ToolCall) -> Result<CapabilityExecution, CapabilityError> {
        let requested = call
            .arguments
            .get("path")
            .and_then(|value| value.as_str())
            .ok_or(CapabilityError::InvalidArguments)?;
        let target = self.boundary.resolve_existing(requested)?;
        let metadata = tokio::fs::metadata(&target).await?;
        if !metadata.is_file() {
            return Err(CapabilityError::TargetNotFile);
        }
        let metadata_bytes = usize::try_from(metadata.len()).unwrap_or(usize::MAX);
        enforce_read_limit(metadata_bytes)?;

        let bytes = tokio::fs::read(&target).await?;
        enforce_read_limit(bytes.len())?;
        if bytes.contains(&0) {
            return Err(CapabilityError::FileContainsNul);
        }
        let content = String::from_utf8(bytes.clone()).map_err(|_| CapabilityError::InvalidUtf8)?;
        let path = self.boundary.relative_path(&target)?;
        let sha256 = hex::encode(Sha256::digest(&bytes));
        let result = json!({"path": path, "content": content, "bytes": bytes.len()});
        let evidence_payload = json!({
            "path": result["path"],
            "bytes": bytes.len(),
            "sha256": sha256
        });

        Ok(CapabilityExecution {
            result: ToolResult {
                call_id: call.id.clone(),
                capability_id: call.capability_id.clone(),
                output: result,
            },
            evidence_payload,
        })
    }
}

fn enforce_read_limit(actual_bytes: usize) -> Result<(), CapabilityError> {
    if actual_bytes > MAX_WORKSPACE_READ_BYTES {
        Err(CapabilityError::FileTooLarge {
            max_bytes: MAX_WORKSPACE_READ_BYTES,
            actual_bytes,
        })
    } else {
        Ok(())
    }
}
```

Add error variants:

```rust
#[error("target path is not a regular file")]
TargetNotFile,
#[error("file exceeds {max_bytes} byte limit: {actual_bytes} bytes")]
FileTooLarge {
    max_bytes: usize,
    actual_bytes: usize,
},
#[error("file contains a NUL byte")]
FileContainsNul,
#[error("file is not valid UTF-8 text")]
InvalidUtf8,
```

- [ ] **Step 6: Run tools tests, inspect failures, and reach GREEN**

```bash
cargo test -p hc-tools
```

Expected: all list/read/security tests pass and Cargo updates `Cargo.lock` with direct `hc-tools` dependencies.

- [ ] **Step 7: Run Clippy for the affected crate**

```bash
cargo clippy -p hc-tools --all-targets --locked -- -D warnings
```

Expected: pass without warnings. Refactor only after tests are green.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml Cargo.lock crates/hc-tools/Cargo.toml crates/hc-tools/src/lib.rs
git commit -m "feat(tools): add bounded workspace read capability"
```

---

### Task 4: Add deterministic and OpenAI-compatible model codecs

**Files:**
- Modify: `crates/hc-models/src/lib.rs`
- Test: `crates/hc-models/src/lib.rs`
- Modify call sites that construct the old unit provider only after tests expose them.

**Interfaces:**
- Consumes: `ToolCall::workspace_read`, model messages/results.
- Produces: `DeterministicProvider::default()`, `DeterministicProvider::workspace_read(path)`, dual-tool OpenAI schema/parser.

- [ ] **Step 1: Add failing deterministic read test**

```rust
#[tokio::test]
async fn deterministic_read_scenario_calls_read_then_finishes() {
    let provider = DeterministicProvider::workspace_read("alpha.txt");
    let first = provider
        .next_turn(ModelRequest::user("Read alpha.txt"))
        .await
        .unwrap();
    let calls = match first {
        ModelOutput::ToolCalls(calls) => calls,
        other => panic!("expected tool calls, got {other:?}"),
    };
    assert_eq!(calls[0].capability_id, "workspace.read");
    assert_eq!(calls[0].arguments, json!({"path": "alpha.txt"}));

    let second = provider
        .next_turn(ModelRequest::with_tool_result(
            "Read alpha.txt",
            ToolResult {
                call_id: calls[0].id.clone(),
                capability_id: "workspace.read".into(),
                output: json!({"path": "alpha.txt", "content": "alpha", "bytes": 5}),
            },
        ))
        .await
        .unwrap();
    assert_eq!(
        second,
        ModelOutput::FinalText("Contents of alpha.txt:\nalpha".into())
    );
}
```

- [ ] **Step 2: Add failing OpenAI schema/parser tests**

Inside the module, test the private request builder:

```rust
#[test]
fn openai_request_declares_list_and_read_tools() {
    let provider = OpenAiCompatibleProvider::new("http://localhost", "", "test");
    let body = provider.request_body(&ModelRequest::user("Read alpha.txt")).unwrap();
    let names = body["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["function"]["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["workspace.list", "workspace.read"]);
}
```

Add parser fixture:

```rust
#[test]
fn openai_codec_maps_workspace_read_to_canonical_form() {
    let fixture = json!({
        "choices": [{"message": {
            "content": null,
            "tool_calls": [{
                "id": "call_read",
                "type": "function",
                "function": {
                    "name": "workspace.read",
                    "arguments": "{\"path\":\"alpha.txt\"}"
                }
            }]
        }}]
    });
    let output = parse_openai_chat_completion(fixture).unwrap();
    let ModelOutput::ToolCalls(calls) = output else { panic!("expected tool call") };
    assert_eq!(calls[0].capability_id, "workspace.read");
    assert_eq!(calls[0].arguments, json!({"path": "alpha.txt"}));
}
```

- [ ] **Step 3: Run model tests and observe RED**

```bash
cargo test -p hc-models
```

Expected: missing scenario constructor and missing read schema/parser support.

- [ ] **Step 4: Implement scenario-driven deterministic provider**

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeterministicScenario {
    WorkspaceList { path: String },
    WorkspaceRead { path: String },
}

pub struct DeterministicProvider {
    scenario: DeterministicScenario,
}

impl Default for DeterministicProvider {
    fn default() -> Self {
        Self {
            scenario: DeterministicScenario::WorkspaceList { path: ".".into() },
        }
    }
}

impl DeterministicProvider {
    pub fn workspace_read(path: impl Into<String>) -> Self {
        Self {
            scenario: DeterministicScenario::WorkspaceRead { path: path.into() },
        }
    }
}
```

On the first turn, match `scenario` and emit the corresponding call. On a tool result, match `result.capability_id`; preserve the existing list summary and add the read summary using strict string extraction for `path` and `content`. Missing read fields return `ModelError::InvalidResponse("workspace.read result requires path and content")`.

- [ ] **Step 5: Declare and parse both OpenAI tools**

Make the `tools` array contain two function definitions with identical strict path schemas and distinct descriptions. Extend the parser match:

```rust
"workspace.read" => {
    let path = arguments.get("path").and_then(Value::as_str).ok_or(
        ModelError::InvalidResponse("workspace.read requires string path"),
    )?;
    canonical.push(ToolCall::workspace_read(id, path));
}
```

- [ ] **Step 6: Update unit-provider call sites**

Replace `DeterministicProvider` values with `DeterministicProvider::default()` in:

- `crates/hc-agent/src/lib.rs` tests;
- `crates/hc-api/src/main.rs`;
- `crates/hc-api/tests/vertical_slice.rs` list test;
- `apps/desktop/src-tauri/src/lib.rs`.

- [ ] **Step 7: Run models and affected compile/tests**

```bash
cargo test -p hc-models
cargo test -p hc-agent --no-run
cargo test -p hc-api --no-run
cargo check -p hermesclaw-desktop --locked
```

Expected: all commands pass.

- [ ] **Step 8: Commit**

```bash
git add crates/hc-models/src/lib.rs crates/hc-agent/src/lib.rs crates/hc-api/src/main.rs crates/hc-api/tests/vertical_slice.rs apps/desktop/src-tauri/src/lib.rs
git commit -m "feat(models): support workspace read tool calls"
```

---

### Task 5: Prove sanitized evidence through the agent loop

**Files:**
- Modify: `crates/hc-agent/src/lib.rs`
- Test: `crates/hc-agent/src/lib.rs`

**Interfaces:**
- Consumes: `DeterministicProvider::workspace_read`, `WorkspaceReadCapability`, result/evidence split.
- Produces: completed read mission with text in response and sanitized evidence in SQLite.

- [ ] **Step 1: Add the failing agent read/evidence test**

```rust
#[tokio::test]
async fn read_turn_returns_text_without_persisting_file_content() {
    let workspace = tempdir().expect("workspace");
    fs::write(workspace.path().join("alpha.txt"), "alpha secret text").unwrap();

    let mut registry = CapabilityRegistry::new();
    registry.register(WorkspaceReadCapability::new(workspace.path()).unwrap());
    let coordinator = TurnCoordinator::new(
        DeterministicProvider::workspace_read("alpha.txt"),
        registry,
        EvidenceStore::in_memory().unwrap(),
    );

    let outcome = coordinator
        .run(ChatInput::new("Read alpha.txt", AutonomyProfile::Observe))
        .await
        .unwrap();

    assert_eq!(outcome.mission_state, MissionState::Completed);
    assert_eq!(outcome.response, "Contents of alpha.txt:\nalpha secret text");
    assert_eq!(outcome.evidence.len(), 2);
    let execution = &outcome.evidence[1];
    assert_eq!(execution.capability_id.as_deref(), Some("workspace.read"));
    assert_eq!(execution.payload["path"], "alpha.txt");
    assert_eq!(execution.payload["bytes"], 17);
    assert_eq!(execution.payload["sha256"].as_str().unwrap().len(), 64);
    assert!(execution.payload.get("content").is_none());
    assert!(!execution.payload.to_string().contains("alpha secret text"));
}
```

- [ ] **Step 2: Run focused test and observe RED or wrong evidence**

```bash
cargo test -p hc-agent read_turn_returns_text_without_persisting_file_content
```

Expected before Task 2 integration is complete: compile or assertion failure. After Task 2, this may already pass; if so, temporarily alter the evidence assertion to prove it can fail, restore it, then rerun. Do not accept a never-failed test.

- [ ] **Step 3: Make only the minimal agent adjustments required**

The expected implementation is the Task 2 execution block. Do not add a second evidence store or redact the model-visible result. Fix only mismatches revealed by the test.

- [ ] **Step 4: Run full agent tests**

```bash
cargo test -p hc-agent
```

Expected: list regression and read/evidence test both pass.

- [ ] **Step 5: Commit if the task required code changes**

```bash
git add crates/hc-agent/src/lib.rs
git commit -m "test(agent): prove sanitized workspace read evidence"
```

If no production change was required because Task 2 already implemented the contract, commit the new test alone with the same message.

---

### Task 6: Wire Axum and Tauri, then prove the HTTP vertical slice

**Files:**
- Modify: `crates/hc-api/src/main.rs`
- Modify: `crates/hc-api/tests/vertical_slice.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Test: `crates/hc-api/tests/vertical_slice.rs`

**Interfaces:**
- Consumes: both workspace capabilities and deterministic read scenario.
- Produces: HTTP E2E read proof and both capabilities registered in server/desktop startup.

- [ ] **Step 1: Add a failing Axum read test**

Add a second integration test:

```rust
#[tokio::test]
async fn chat_endpoint_reads_text_and_exposes_only_sanitized_evidence() {
    let workspace = tempdir().expect("workspace");
    fs::write(workspace.path().join("alpha.txt"), "alpha secret text").unwrap();

    let mut registry = CapabilityRegistry::new();
    registry.register(WorkspaceListCapability::new(workspace.path()).unwrap());
    registry.register(WorkspaceReadCapability::new(workspace.path()).unwrap());
    let coordinator = TurnCoordinator::new(
        DeterministicProvider::workspace_read("alpha.txt"),
        registry,
        EvidenceStore::in_memory().unwrap(),
    );
    let app = build_router(coordinator);

    let response = app
        .clone()
        .oneshot(
            Request::post("/api/chat")
                .header("content-type", "application/json")
                .body(Body::from(json!({"message": "Read alpha.txt"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let chat: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(chat["response"], "Contents of alpha.txt:\nalpha secret text");
    let trace_id = chat["trace_id"].as_str().unwrap();

    let response = app
        .oneshot(
            Request::get(format!("/api/evidence/{trace_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let evidence: Vec<Value> = serde_json::from_slice(&body).unwrap();
    let execution = evidence
        .iter()
        .find(|row| row["capability_id"] == "workspace.read")
        .unwrap();
    assert_eq!(execution["payload"]["path"], "alpha.txt");
    assert!(execution["payload"].get("content").is_none());
    assert!(!execution.to_string().contains("alpha secret text"));
}
```

- [ ] **Step 2: Run the focused integration test and observe RED**

```bash
cargo test -p hc-api chat_endpoint_reads_text_and_exposes_only_sanitized_evidence
```

Expected: missing imports/registration or read behavior until integration is complete.

- [ ] **Step 3: Register both capabilities in runtime entry points**

In `hc-api/src/main.rs` and the Tauri `setup` closure:

```rust
let list = WorkspaceListCapability::new(&workspace)?;
let read = WorkspaceReadCapability::new(&workspace)?;
let mut registry = CapabilityRegistry::new();
registry.register(list);
registry.register(read);
```

Import `WorkspaceReadCapability`. Keep default deterministic provider list behavior in production entry points.

- [ ] **Step 4: Run API and desktop checks**

```bash
cargo test -p hc-api
cargo check -p hermesclaw-desktop --locked
```

Expected: both pass.

- [ ] **Step 5: Commit**

```bash
git add crates/hc-api/src/main.rs crates/hc-api/tests/vertical_slice.rs apps/desktop/src-tauri/src/lib.rs
git commit -m "feat(apps): register workspace read capability"
```

---

### Task 7: Extend the mission cockpit without exposing evidence content

**Files:**
- Modify: `apps/web/src/App.tsx`
- Modify: `apps/web/src/App.test.tsx`
- Test: `apps/web/src/App.test.tsx`

**Interfaces:**
- Consumes: unchanged `ChatResponse` and `EvidenceRecord` API types.
- Produces: UI wording and test coverage for `workspace.read` response/evidence.

- [ ] **Step 1: Add the failing UI test**

```typescript
it('renders a workspace read response and sanitized evidence', async () => {
  sendChat.mockResolvedValue({
    trace_id: 'read-trace',
    mission_id: 'read-mission',
    mission_state: 'completed',
    response: 'Contents of alpha.txt:\nalpha secret text',
  });
  getEvidence.mockResolvedValue([
    {
      kind: 'policy_decision',
      capability_id: 'workspace.read',
      status: 'allowed',
    },
    {
      kind: 'capability_execution',
      capability_id: 'workspace.read',
      status: 'succeeded',
      payload: {
        path: 'alpha.txt',
        bytes: 17,
        sha256: 'a'.repeat(64),
      },
    },
  ]);

  const user = userEvent.setup();
  render(<App />);
  await user.type(screen.getByLabelText('Message'), 'Read alpha.txt');
  await user.click(screen.getByRole('button', { name: 'Run mission' }));

  expect(await screen.findByText('Completed')).toBeInTheDocument();
  expect(screen.getByText(/Contents of alpha.txt/)).toBeInTheDocument();
  expect(screen.getByText(/alpha secret text/)).toBeInTheDocument();
  await user.click(screen.getByText(/Evidence/));
  expect(screen.getAllByText('workspace.read').length).toBeGreaterThan(0);
  expect(screen.queryByText('alpha secret text', { selector: '.evidence-row *' })).not.toBeInTheDocument();
});
```

- [ ] **Step 2: Run web tests and observe RED if copy/selector support is missing**

```bash
npm --prefix apps/web ci
npm run test:web
```

The new behavior test must be observed failing before UI changes. If the generic UI already makes it pass, first assert the new approved placeholder/copy, observe that failure, then implement that copy while keeping the security assertion.

- [ ] **Step 3: Update approved UI copy only**

Change the introductory paragraph to mention listing and reading authorized text files. Change the textarea placeholder to:

```text
List this workspace or read alpha.txt
```

Keep `Observe · read-only capabilities`; do not render raw evidence payloads.

- [ ] **Step 4: Run tests, typecheck, and build**

```bash
npm run test:web
npm run lint:web
npm run build:web
```

Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/App.tsx apps/web/src/App.test.tsx
git commit -m "test(web): cover workspace read mission evidence"
```

---

### Task 8: Run full proof gates and write canonical evidence

**Files:**
- Create: `docs/evidence/2026-08-14-workspace-read-verification.md`
- Modify: `source/03_CURRENT_STATE.md`
- Modify: `source/09_ROADMAP.md`
- Modify: `source/11_HANDOFF.md`
- Modify: `source/SOURCE_MANIFEST.json`

**Interfaces:**
- Consumes: all completed implementation and exact command/CI output.
- Produces: current, reproducible project memory and handoff.

- [ ] **Step 1: Run all local verification gates**

```bash
cargo fmt --all --check
cargo test --workspace --exclude hermesclaw-desktop --locked
cargo clippy --workspace --exclude hermesclaw-desktop --all-targets --locked -- -D warnings
npm --prefix apps/web ci
npm run test:web
npm run lint:web
npm run build:web
cargo check -p hermesclaw-desktop --locked
python source/raglite/validate_source.py
git diff --check
```

Expected: every command exits `0`. Record exact test counts and failures (`0`) from output.

- [ ] **Step 2: Create the evidence document from actual output**

The file must state:

- branch and commit SHA tested;
- exact commands;
- exact Rust/Web test counts;
- security cases covered;
- Tauri compile status;
- evidence-redaction assertion;
- known limitation: no adversarial filesystem race hardening;
- no claim of write/process/binary support.

Do not use “passing” without the command output from Step 1.

- [ ] **Step 3: Update canonical state and roadmap**

In `03_CURRENT_STATE.md`, add `workspace.read` only after the local gates are green. In `09_ROADMAP.md`, mark this Phase 3 increment verified and name the next single bounded checkpoint rather than declaring the whole file/process wave complete. In `11_HANDOFF.md`, record exact branch, commit, evidence file, blockers, and next action.

- [ ] **Step 4: Recalculate manifest hashes**

Run this exact script after source edits:

```bash
python - <<'PY'
from __future__ import annotations

import hashlib
import json
from datetime import date
from pathlib import Path

source = Path('source')
manifest_path = source / 'SOURCE_MANIFEST.json'
payload = json.loads(manifest_path.read_text(encoding='utf-8'))
for document in payload['documents']:
    path = source / document['path']
    document['sha256'] = hashlib.sha256(path.read_bytes()).hexdigest()
payload['generated_at'] = date.today().isoformat()
manifest_path.write_text(json.dumps(payload, indent=2) + '\n', encoding='utf-8')
PY
python source/raglite/validate_source.py
```

Expected: `OK: 19 source documents validated`.

- [ ] **Step 5: Commit documentation/evidence**

```bash
git add docs/evidence/2026-08-14-workspace-read-verification.md \
  source/03_CURRENT_STATE.md source/09_ROADMAP.md source/11_HANDOFF.md \
  source/SOURCE_MANIFEST.json
git commit -m "docs: record workspace read verification"
```

- [ ] **Step 6: Push, open a draft PR, and verify GitHub Actions**

```bash
git push -u origin agent/workspace-read
```

Open a draft PR targeting `main`. The PR body must summarize the typed contract, boundary reuse, redacted evidence, tests, and limitations. Wait for Rust, Web, and Tauri jobs to finish. Record the concrete run ID and job conclusions in the evidence/handoff through a final documentation commit if they differ from local evidence.

- [ ] **Step 7: Independent code review**

Run the available Codex Engineering Guardrails verification workflow against the final diff. Run Fallow on changed TypeScript/JavaScript if available. Run SonarQube analysis if the repository has a configured Sonar project; otherwise record “not configured” rather than fabricating a result.

Address only findings that are supported by source or executable evidence, rerun affected gates, and keep the PR draft until every required check is green.

---

## Plan Self-Review

- Spec coverage: domain, boundary, capability, evidence split, model codecs, agent, Axum, Tauri, React, security cases, verification, and memory write-back are each assigned to a task.
- Type consistency: every later task uses `CapabilityExecution { result, evidence_payload }`, `WorkspaceReadCapability::new`, `DeterministicProvider::workspace_read`, and `ToolCall::workspace_read` exactly as defined earlier.
- Scope: no process execution, mutation, binary support, streaming, partial reads, or broad file framework is included.
- Placeholder scan: the plan contains no `TBD`, `TODO`, “similar to”, or unspecified test step. Dynamic commit/run identifiers must be copied from actual command/tool output and are explicitly forbidden from being guessed.
