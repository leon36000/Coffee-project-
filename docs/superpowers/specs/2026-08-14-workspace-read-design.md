# HermesClaw `workspace.read` Capability Design

**Status:** approved design baseline  
**Date:** 2026-08-14  
**Scope:** one bounded read-only file capability; no process execution, file mutation, streaming, or binary-file support

## 1. Goal

Add a production-shaped `workspace.read` capability to the existing HermesClaw architectural spine. A model may request one UTF-8 text file inside the configured workspace, the Policy Kernel must authorize the read, the Capability Engine must enforce the workspace boundary and content limits, the model must receive the text, and the Evidence Engine must persist only a sanitized proof rather than a second durable copy of the file contents.

The slice extends the existing `workspace.list` path without creating a second agent runtime, policy path, state store, or file-security implementation.

## 2. User-visible behavior

A successful canonical tool call has this shape:

```json
{
  "id": "call-1",
  "capability_id": "workspace.read",
  "arguments": {
    "path": "docs/notes.md"
  },
  "risk": "low",
  "side_effect": "none"
}
```

The tool result delivered to the model is:

```json
{
  "path": "docs/notes.md",
  "content": "file text",
  "bytes": 9
}
```

The capability-execution evidence persisted in SQLite is deliberately different:

```json
{
  "path": "docs/notes.md",
  "bytes": 9,
  "sha256": "<64 lowercase hexadecimal characters>"
}
```

`content` must never be copied into the evidence payload. The model-visible result remains available only in the active turn history.

## 3. Limits and rejection rules

`workspace.read` accepts exactly one path and applies these fixed v1 limits:

- the path argument must be a non-empty string;
- the resolved target must exist;
- canonical resolution must remain under the canonical workspace root;
- the resolved target must be a regular file;
- the file may contain at most `65_536` bytes;
- the bytes must be valid UTF-8;
- the bytes must not contain `0x00`;
- the returned path uses forward slashes and is relative to the canonical workspace root.

The capability rejects:

- parent traversal and absolute paths resolving outside the workspace;
- symbolic links resolving outside the workspace;
- directories and other non-regular targets;
- nonexistent or inaccessible paths;
- files of `65_537` bytes or more;
- invalid UTF-8;
- NUL-containing content.

A symbolic link resolving to a regular file that remains inside the workspace is accepted. The returned path identifies the canonical target, not the alias used to reach it.

The implementation performs canonical resolution before reading and rechecks the byte count after reading. This slice protects against deterministic traversal and symlink escape. Hardening against a hostile actor replacing filesystem objects between canonicalization and open is a later OS-specific broker concern and is not falsely claimed here.

## 4. Shared workspace boundary

The current `WorkspaceListCapability` owns private path-resolution logic. That logic becomes one focused reusable component:

```rust
pub struct WorkspaceBoundary {
    workspace_root: PathBuf,
}

impl WorkspaceBoundary {
    pub fn new(workspace_root: impl AsRef<Path>) -> Result<Self, CapabilityError>;
    pub fn resolve_existing(&self, requested: &str) -> Result<PathBuf, CapabilityError>;
    pub fn relative_path(&self, canonical: &Path) -> Result<String, CapabilityError>;
}
```

Rules:

- `new` canonicalizes the root and requires a directory;
- `resolve_existing` rejects an empty request, canonicalizes `root.join(requested)`, and enforces containment;
- `relative_path` accepts only a canonical path under the same root and normalizes separators to `/`.

`WorkspaceListCapability` and `WorkspaceReadCapability` each own a `WorkspaceBoundary`. They do not duplicate containment logic.

## 5. Result/evidence separation

The existing `Capability` trait returns only `ToolResult`, which forces the agent to persist the complete model-visible output as evidence. Replace that return contract with:

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct CapabilityExecution {
    pub result: ToolResult,
    pub evidence_payload: serde_json::Value,
}

#[async_trait]
pub trait Capability: Send + Sync {
    fn id(&self) -> &'static str;
    async fn execute(&self, call: &ToolCall)
        -> Result<CapabilityExecution, CapabilityError>;
}
```

`CapabilityRegistry::execute` returns `CapabilityExecution`.

The agent:

1. evaluates policy exactly as it does now;
2. executes the capability;
3. persists `execution.evidence_payload`;
4. sends `execution.result` back to the model.

For `workspace.list`, `evidence_payload` remains equal to the existing small `{"entries": [...]}` output, preserving current behavior. For `workspace.read`, the evidence payload is the sanitized path/size/hash object.

## 6. Domain contract

Add one constructor without changing the serialized `ToolCall` shape:

```rust
pub fn workspace_read(
    id: impl Into<String>,
    path: impl Into<String>,
) -> ToolCall;
```

It creates capability ID `workspace.read`, risk `Low`, side effect `None`, and model-generated provenance, matching `workspace.list` policy semantics.

No new autonomy profile or policy exception is needed: the current Policy Kernel already permits low-risk, no-side-effect calls in Observe mode.

## 7. Capability implementation

Add:

```rust
pub const MAX_WORKSPACE_READ_BYTES: usize = 64 * 1024;

pub struct WorkspaceReadCapability {
    boundary: WorkspaceBoundary,
}
```

Execution order:

1. read and validate `arguments.path`;
2. resolve through `WorkspaceBoundary`;
3. require a regular file;
4. check metadata length against the fixed limit;
5. read bytes with Tokio;
6. recheck actual byte length;
7. reject NUL bytes;
8. convert with strict UTF-8 validation;
9. compute SHA-256 over the exact bytes;
10. return separate result and evidence payloads.

Use direct `sha2 = "0.10"` and `hex = "0.4"` dependencies in `hc-tools`. Both packages already exist in the workspace lock graph, but the lockfile's `hc-tools` dependency list must be regenerated or updated by Cargo and committed.

New capability errors are stable and specific:

```rust
TargetNotFile
FileTooLarge { max_bytes: usize, actual_bytes: usize }
FileContainsNul
InvalidUtf8
```

Existing I/O, invalid-arguments, workspace-root, unknown-capability, and path-escape errors remain.

## 8. Model Fabric

### Deterministic provider

Convert the unit struct into a small scenario-driven provider while preserving `Default` as the current workspace-list behavior:

```rust
pub enum DeterministicScenario {
    WorkspaceList { path: String },
    WorkspaceRead { path: String },
}

pub struct DeterministicProvider {
    scenario: DeterministicScenario,
}

impl Default for DeterministicProvider;
impl DeterministicProvider {
    pub fn workspace_read(path: impl Into<String>) -> Self;
}
```

The first turn emits the configured tool call. After a `workspace.read` result, the provider returns:

```text
Contents of <path>:
<content>
```

This is an explicit test oracle, not an attempt to implement natural-language intent classification.

### OpenAI-compatible provider

The request advertises two function tools: `workspace.list` and `workspace.read`, each with a required string `path` and no additional properties.

`parse_openai_chat_completion` accepts `workspace.read` and maps it to `ToolCall::workspace_read`. Unsupported tool names continue to fail closed.

## 9. Agent, API, desktop, and UI integration

### Agent

The coordinator stores only the sanitized evidence payload and sends the full result to the model. A regression test proves `workspace.list` still behaves as before. A new read test proves the final response contains the file content while SQLite evidence does not.

### Axum

`hc-api` remains transport-neutral. Test construction can inject `DeterministicProvider::workspace_read("alpha.txt")`. The production proof binary registers both file capabilities while leaving the deterministic default scenario as list.

### Tauri

The desktop setup registers both capabilities against the same workspace root. No new Tauri command is required because chat and evidence remain the public surface.

### React

The existing generic response/evidence UI remains. Update the prompt example and explanatory copy to mention authorized text-file reads. Add a UI test whose mocked result contains a `workspace.read` response and sanitized evidence metadata; the test must prove the file text is shown in the response while no evidence row renders the text as a label or status.

## 10. Testing strategy

Tests are written before production changes and must demonstrate these behaviors:

### Domain

- `ToolCall::workspace_read` has stable capability, risk, side-effect, provenance, and JSON arguments.

### Capability/security

- successful UTF-8 read returns normalized relative path, content, and byte count;
- evidence contains path, bytes, and correct SHA-256 but no content;
- parent traversal is rejected;
- absolute or symlink escape is rejected;
- an in-workspace symlink is accepted and reports its canonical target path;
- a directory is rejected;
- a `65_537` byte file is rejected;
- invalid UTF-8 is rejected;
- NUL-containing content is rejected;
- `workspace.list` remains sorted and workspace-confined.

### Models

- deterministic read scenario emits `workspace.read` then summarizes its result;
- OpenAI tool schema includes both capabilities;
- OpenAI response parsing maps `workspace.read` correctly;
- unsupported tools still fail closed.

### Agent/evidence

- Observe policy allows the read;
- a read mission reaches `completed`;
- model-visible response contains the file text;
- capability evidence contains the digest metadata;
- capability evidence does not contain `content` or the file text.

### Integration/UI

- Axum chat -> policy -> read -> evidence -> response succeeds;
- evidence endpoint returns sanitized metadata;
- React renders the response, completed state, and `workspace.read` evidence;
- Tauri compiles with both registered capabilities.

## 11. Verification gates

The change is eligible for merge only after current evidence shows:

```text
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

Security tests are part of the Rust test suite, not a manual checklist.

## 12. Acceptance criteria

`workspace.read` is accepted for this bounded slice when:

- all contracts above are implemented;
- all listed success and rejection tests pass;
- result/evidence separation is proven for both read and list capabilities;
- file contents are absent from persisted evidence;
- both deterministic and OpenAI-compatible codecs support the capability;
- Axum, React, and Tauri paths compile/test successfully;
- canonical current-state and handoff files record exact commits, runs, tests, and remaining limitations.

This acceptance does not claim support for binary files, partial reads, streaming, writes, patches, deletion, process execution, or adversarial filesystem race hardening.
