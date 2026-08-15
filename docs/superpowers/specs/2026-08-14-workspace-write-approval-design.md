# HermesClaw `workspace.write` + Durable Approval Design

**Status:** approved design baseline  
**Date:** 2026-08-14  
**Scope:** one create-only UTF-8 workspace mutation plus the minimum durable Approval Engine required to authorize, resume, execute, and audit it safely

## 1. Goal

Add a first mutating HermesClaw capability without weakening the central policy boundary.

The slice introduces:

- `workspace.write` in `create_new` mode only;
- Assist-profile approval before any mutation;
- a durable approval request that survives application restart;
- encrypted pending action/continuation state;
- exact, single-use approval binding by action digest;
- atomic no-overwrite publication;
- secret-safe evidence;
- restart recovery for an action interrupted during execution.

It does **not** add overwrite, patch, delete, rename, process execution, or a second policy/runtime/state owner.

## 2. Why the approval is durable but the grant is single-use

The approval request is durable product state. Closing and reopening HermesClaw must not discard it. A pending request remains inspectable until it is approved, denied, or expires.

The execution grant is intentionally narrow:

- bound to one exact tool call;
- usable once;
- time-limited;
- created only by a trusted human decision path;
- impossible for model output to manufacture.

This distinction avoids both undesirable extremes:

- ephemeral approvals that disappear when the app closes;
- broad approvals that silently become permanent write permission.

## 3. Alternatives considered

### 3.1 Direct write with no durable approval

The model proposes a write and the server either executes immediately or returns an error while the UI is open.

Rejected because it cannot survive restart, does not provide an externally authoritative human decision, and would make a consequential capability depend on transient UI state.

### 3.2 Create and replace in one capability

Support both new-file creation and replacement guarded by an expected SHA-256.

Deferred because replacement adds permission preservation, optimistic concurrency, rollback, platform-specific rename behavior, and recovery semantics. The first write slice must be narrow enough to prove authorization and durability before destructive or replacing behavior is introduced.

### 3.3 Patch/diff engine first

Apply line-oriented patches instead of whole-file content.

Deferred because patch parsing, line-ending preservation, conflicts, encoding, and partial application are a separate capability family.

### 3.4 Chosen approach

Create-only mutation with durable encrypted approval and no-overwrite publication. Replacement and patching will be separate approved slices.

## 4. Canonical ownership

The existing one-owner rule remains:

- approval policy and approval lifecycle -> `hc-policy`;
- approval persistence and transactions -> `hc-state` implementing a policy-owned repository trait;
- mission/turn pause and resume -> `hc-agent` using `hc-mission`;
- write execution -> `hc-tools`;
- transport -> `hc-api` and Tauri adapters;
- UI state -> React projection of canonical backend state;
- audit -> the existing Evidence Engine path.

No `hc-approvals` sidecar runtime or second database is introduced.

## 5. Capability contract

### 5.1 Canonical tool call

```json
{
  "id": "call-write-1",
  "capability_id": "workspace.write",
  "arguments": {
    "path": "notes/new-file.txt",
    "content": "hello",
    "mode": "create_new"
  },
  "risk": "medium",
  "side_effect": "mutation",
  "provenance": {
    "source": "model",
    "trust": "model_generated"
  }
}
```

Add:

```rust
pub fn workspace_write_create(
    id: impl Into<String>,
    path: impl Into<String>,
    content: impl Into<String>,
) -> ToolCall;
```

The constructor always sets:

- capability ID `workspace.write`;
- risk `Medium`;
- side effect `Mutation`;
- mode `create_new`;
- model-generated provenance.

The model cannot choose a lower risk or side-effect class.

### 5.2 Result

```json
{
  "path": "notes/new-file.txt",
  "mode": "create_new",
  "bytes": 5,
  "sha256": "2cf24dba...",
  "created": true,
  "atomic": true
}
```

### 5.3 Persisted capability evidence

The same metadata may be persisted, but never `content`:

```json
{
  "path": "notes/new-file.txt",
  "mode": "create_new",
  "bytes": 5,
  "sha256": "2cf24dba...",
  "created": true,
  "atomic": true
}
```

## 6. Write constraints

The first version accepts only:

- a non-empty relative path;
- `mode == "create_new"`;
- an existing parent directory;
- a parent directory whose canonical path remains under the configured workspace root;
- a final target that does not exist;
- UTF-8 text represented by a JSON string;
- no NUL character;
- at most exactly `65_536` UTF-8 bytes.

It rejects:

- absolute paths;
- `.` or `..` path components;
- missing parent directories;
- parent traversal or symlink escape;
- an existing target of any kind, including a symlink;
- a target path ending in an empty filename;
- content of `65_537` bytes or more;
- NUL-containing content;
- any mode other than `create_new`.

The target is resolved by extending the shared `WorkspaceBoundary`, not by creating independent write-path logic.

## 7. Shared create-target boundary

Add:

```rust
pub struct CreateTarget {
    pub canonical_parent: PathBuf,
    pub relative_path: String,
    pub target_path: PathBuf,
}

impl WorkspaceBoundary {
    pub fn resolve_create_target(
        &self,
        requested: &str,
    ) -> Result<CreateTarget, CapabilityError>;
}
```

Resolution rules:

1. reject empty and absolute paths;
2. reject `ParentDir`, `CurDir`, `RootDir`, and platform prefix components;
3. split the final filename from its parent;
4. canonicalize the parent directory;
5. enforce that the canonical parent starts with the canonical workspace root;
6. require the parent to be a directory;
7. construct the final target from the canonical parent and filename;
8. use `symlink_metadata` to require the target to be absent;
9. return the normalized path relative to the workspace using `/` separators.

If the target appears after approval but before publication, execution fails with `TargetAlreadyExists`; it never overwrites.

## 8. Policy and approval behavior

### 8.1 Initial policy decision

- Observe -> `Deny("observe profile forbids side effects")`.
- Assist -> `RequiresApproval("assist profile requires approval for consequential actions")`.
- Autonomous Scoped -> this first slice still requires approval for `workspace.write`; automatic scoped mutation is deferred until explicit grants and workspace-write policy are designed.

The current generic Autonomous Scoped rule is therefore tightened for this capability. `workspace.write` cannot become autonomous merely because it is non-critical.

### 8.2 Verified approval token

Introduce an opaque type whose constructor is not public outside the policy module:

```rust
pub struct VerifiedApproval {
    approval_id: ApprovalId,
    action_digest: ActionDigest,
}
```

`PolicyKernel::evaluate` receives a `PolicyContext`:

```rust
pub struct PolicyContext<'a> {
    pub profile: AutonomyProfile,
    pub verified_approval: Option<&'a VerifiedApproval>,
}
```

For `workspace.write`, Allow is possible only when:

- profile is Assist;
- a `VerifiedApproval` exists;
- its digest exactly equals the current call digest;
- the Approval Engine has atomically moved the request into `executing` state.

The opaque token is constructed only after trusted approval-store validation. It is never deserialized from model or client JSON.

## 9. Approval domain model

Add canonical IDs and status:

```rust
pub struct ApprovalId(Uuid);

pub enum ApprovalStatus {
    Pending,
    Executing,
    Consumed,
    Denied,
    Expired,
    Failed,
}
```

Persisted approval metadata:

```rust
pub struct ApprovalRequest {
    pub approval_id: ApprovalId,
    pub trace_id: TraceId,
    pub mission_id: MissionId,
    pub capability_id: String,
    pub action_digest: String,
    pub summary: serde_json::Value,
    pub status: ApprovalStatus,
    pub requested_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub decided_at: Option<DateTime<Utc>>,
    pub decision_actor: Option<String>,
    pub failure_code: Option<String>,
}
```

The public summary for `workspace.write` is:

```json
{
  "path": "notes/new-file.txt",
  "mode": "create_new",
  "bytes": 5,
  "sha256": "2cf24dba..."
}
```

No plaintext content is stored in the summary.

Default approval lifetime is exactly 24 hours. Expiration is evaluated against the server clock. A pending request past `expires_at` is atomically marked `expired`, its encrypted payload is erased, and it cannot be revived.

## 10. Exact action digest

Approval is bound to the exact action through SHA-256 over RFC 8785/JCS canonical JSON.

Digest material is a typed envelope:

```rust
struct ActionDigestMaterial<'a> {
    schema_version: u8,
    call_id: &'a str,
    capability_id: &'a str,
    arguments: &'a serde_json::Value,
    risk: RiskClass,
    side_effect: SideEffectClass,
    provenance: &'a Provenance,
}
```

Rules:

- `schema_version == 1`;
- serialize with JSON Canonicalization Scheme;
- hash exact UTF-8 bytes with SHA-256;
- encode as 64 lowercase hexadecimal characters.

Changing any path byte, content byte, mode, call ID, risk, side-effect class, or provenance produces a different digest.

## 11. Durable encrypted continuation

A durable approval must contain enough state to resume the same agent turn after restart. The plaintext checkpoint is never stored in SQLite.

```rust
pub struct PendingTurnCheckpoint {
    pub schema_version: u8,
    pub trace_id: TraceId,
    pub mission_id: MissionId,
    pub objective: String,
    pub autonomy: AutonomyProfile,
    pub messages: Vec<ModelMessage>,
    pub pending_call: ToolCall,
    pub remaining_model_iterations: usize,
}
```

The checkpoint is serialized, encrypted, and stored in the approval row as:

- ciphertext bytes;
- 24-byte XChaCha20-Poly1305 nonce;
- key identifier;
- authenticated associated data containing approval ID, trace ID, mission ID, action digest, and checkpoint schema version.

XChaCha20-Poly1305 provides confidentiality and authentication. Swapping ciphertext between rows fails authentication because the associated data differs.

On terminal states `consumed`, `denied`, `expired`, or unrecoverable `failed`, ciphertext and nonce are nulled in the same transaction to minimize retained sensitive data.

## 12. Key management

Define a policy-owned key-provider interface:

```rust
pub trait ApprovalKeyProvider: Send + Sync {
    fn active_key(&self) -> Result<ApprovalKey, ApprovalError>;
    fn key_by_id(&self, key_id: &str) -> Result<ApprovalKey, ApprovalError>;
}
```

Initial product adapters:

1. **Desktop/local:** a 256-bit random master key stored through the operating-system credential store. The SQLite database never contains the key.
2. **Headless/server:** a base64-encoded 32-byte key supplied through `HERMESCLAW_APPROVAL_KEY`; startup validates length and fails closed for mutation-capable mode if absent.
3. **Tests:** an injected deterministic in-memory key provider.

Read-only Observe operation may still start without an approval key. `workspace.write` registration and Assist mutation endpoints remain unavailable until a valid key provider is configured.

The slice does not claim protection from an attacker who already controls the same operating-system user account or process memory.

## 13. Approval repository and SQLite transaction model

`hc-policy` defines `ApprovalRepository`; `hc-state` implements it with SQLite.

Required operations:

```rust
create_pending(request, encrypted_checkpoint)
list_pending(now)
load_public(approval_id, now)
begin_execution(approval_id, actor, now) -> ExecutingApproval
mark_consumed(approval_id, now)
mark_denied(approval_id, actor, now)
mark_failed(approval_id, code, now, retain_for_recovery)
expire_due(now)
load_executing_for_recovery()
```

`begin_execution` is a single SQLite transaction using a conditional update from `pending` to `executing`. Exactly one caller can succeed. Repeated approval attempts return conflict and never create a second grant.

The repository stores approval state in the canonical product SQLite database. No second database is introduced.

## 14. Agent pause and resume

### 14.1 Initial turn

When the Policy Kernel returns `RequiresApproval`:

1. mission transitions `executing -> waiting_approval`;
2. the coordinator computes the action digest and sanitized summary;
3. it builds and encrypts `PendingTurnCheckpoint`;
4. it persists the approval request;
5. it appends `approval_requested` evidence;
6. it returns a normal `ChatOutcome` with `mission_state == waiting_approval` and an `ApprovalPrompt`.

This is not an API error.

### 14.2 Approval

`resume_approved(approval_id, actor)`:

1. atomically calls `begin_execution`;
2. decrypts and authenticates the checkpoint;
3. recomputes and compares the action digest;
4. restores the mission with the original IDs and `waiting_approval` state;
5. transitions `waiting_approval -> executing`;
6. evaluates policy with the opaque `VerifiedApproval`;
7. executes the write;
8. appends capability evidence;
9. sends the tool result back to the configured Model Provider;
10. continues within the saved iteration budget;
11. transitions through `verifying -> completed`;
12. marks approval `consumed` and erases ciphertext.

### 14.3 Denial

Denial atomically changes `pending -> denied`, erases ciphertext, appends `approval_decision` evidence, and returns the mission as `cancelled`. No capability is executed and no model continuation occurs.

### 14.4 Provider continuity

The current configured provider instance is used after restart with the saved provider-neutral `ModelMessage` history. A provider configuration change may cause a normal model error, but it cannot alter the approved tool call because that call is encrypted and digest-bound.

## 15. Recovery after interruption

An approval may remain `executing` if the process exits after consuming approval but before final state recording.

On startup, recovery inspects each executing create-only action:

1. decrypt and verify the checkpoint/digest;
2. resolve the target under the workspace boundary;
3. if the target exists and its SHA-256 equals the approved content hash, record recovered success, continue the model, and mark consumed;
4. if the target does not exist, return the approval to a retryable execution path without issuing a second human grant;
5. if the target exists with a different hash, mark failed with `target_conflict`, erase ciphertext only after preserving the sanitized failure evidence;
6. if decryption/authentication fails, mark failed with `checkpoint_authentication_failed` and execute nothing.

This recovery is safe specifically because the first write mode is create-only and content-hash bound. Replacement recovery is deferred.

## 16. Atomic create-new execution

The write executor performs:

1. validate arguments and content limits;
2. resolve a create target through `WorkspaceBoundary`;
3. create a uniquely named temporary file in the same canonical parent directory with exclusive creation;
4. write the full byte sequence;
5. flush and synchronize the temporary file;
6. publish with a no-clobber primitive equivalent to `persist_noclobber`/atomic link semantics;
7. synchronize the parent directory when supported;
8. remove any temporary name;
9. verify the final file length and SHA-256;
10. return success metadata.

If the target appears before publication, publication fails with `TargetAlreadyExists`. The existing file remains unchanged.

If any pre-publication step fails, the temporary file is removed. Temporary filenames are never surfaced as successful artifacts.

Platform adapters may use different no-clobber primitives, but must satisfy the same behavioral tests. Unsupported filesystems fail closed rather than falling back to overwrite-prone rename.

## 17. Evidence model

New evidence kinds:

### `approval_requested`

```json
{
  "approval_id": "...",
  "action_digest": "...",
  "summary": {
    "path": "notes/new-file.txt",
    "mode": "create_new",
    "bytes": 5,
    "sha256": "..."
  },
  "expires_at": "..."
}
```

### `approval_decision`

```json
{
  "approval_id": "...",
  "decision": "approved",
  "action_digest": "...",
  "decided_by": "local_user"
}
```

### `capability_execution`

```json
{
  "path": "notes/new-file.txt",
  "mode": "create_new",
  "bytes": 5,
  "sha256": "...",
  "created": true,
  "atomic": true
}
```

Plaintext content must not appear in approval metadata, evidence rows, logs, error messages, or UI telemetry.

## 18. API

### 18.1 Chat request

Extend the request with an explicit user-selected profile:

```json
{
  "message": "Create notes/new-file.txt",
  "autonomy": "assist"
}
```

The server never infers elevated autonomy from model output.

### 18.2 Waiting-approval response

HTTP 200:

```json
{
  "trace_id": "...",
  "mission_id": "...",
  "mission_state": "waiting_approval",
  "response": "Approval required",
  "approval": {
    "approval_id": "...",
    "capability_id": "workspace.write",
    "reason": "assist profile requires approval for consequential actions",
    "summary": {
      "path": "notes/new-file.txt",
      "mode": "create_new",
      "bytes": 5,
      "sha256": "..."
    },
    "expires_at": "..."
  }
}
```

The full action is not returned to the client and does not need to be resubmitted.

### 18.3 Pending approvals

```text
GET /api/approvals/pending
GET /api/approvals/{approval_id}
```

Only public metadata is returned.

### 18.4 Decision

```text
POST /api/approvals/{approval_id}/decision
```

```json
{
  "decision": "approve"
}
```

or:

```json
{
  "decision": "deny"
}
```

The trusted local API adapter supplies actor `local_user`. Remote authenticated actor identity is deferred until the Gateway authentication system exists; the current loopback/Tauri surface must not claim remote multi-user identity.

## 19. Tauri and React

### 19.1 Tauri

Add commands matching the API semantics:

- `chat(message, autonomy)`;
- `list_pending_approvals()`;
- `decide_approval(approval_id, decision)`.

Tauri uses the same Policy Kernel, SQLite repository, encrypted checkpoint vault, and capability registry as Axum.

### 19.2 React

The topbar becomes an explicit Observe/Assist selector. Assist is never enabled implicitly.

For a pending write, show an approval card containing:

- target path;
- create-only mode;
- byte count;
- abbreviated SHA-256 with full value accessible;
- expiration;
- reason;
- Deny and Approve buttons.

The UI must:

- restore pending approvals on load;
- never display plaintext content from the pending encrypted checkpoint;
- disable repeat decisions after submission;
- show `completed` only after capability execution and evidence storage;
- show `expired`, `denied`, or `failed` explicitly;
- keep Observe as the default profile.

## 20. Error handling

Stable public error categories include:

- `approval_not_found`;
- `approval_expired`;
- `approval_already_decided`;
- `approval_checkpoint_invalid`;
- `approval_key_unavailable`;
- `action_digest_mismatch`;
- `invalid_write_arguments`;
- `target_already_exists`;
- `parent_not_found`;
- `path_escapes_workspace`;
- `content_too_large`;
- `content_contains_nul`;
- `atomic_publish_unsupported`;
- `write_failed`;
- `target_conflict`.

Public responses do not include plaintext content, encryption details, filesystem temporary names, or sensitive internal paths.

## 21. Tests

### Domain and policy

- canonical write constructor is Medium/Mutation/create_new;
- Observe denies;
- Assist requires approval without token;
- model/client data cannot deserialize a `VerifiedApproval`;
- exact verified token allows only the matching call;
- changed path/content/mode/call/provenance is rejected;
- Autonomous Scoped still requires approval for this capability.

### Digest and encryption

- JCS digest is stable across object key order;
- one-byte content change changes digest;
- encrypt/decrypt round-trip;
- wrong key, nonce, associated data, or row swap fails;
- plaintext sentinel is absent from the SQLite file bytes after pending creation;
- terminal transition erases ciphertext and nonce.

### Repository concurrency

- one of two concurrent approve calls succeeds;
- reuse returns conflict;
- denial and expiration are terminal;
- list pending excludes terminal states;
- restart reopens pending requests.

### Write capability

- create success;
- target path normalization;
- parent/absolute/symlink escape rejection;
- missing parent rejection;
- existing file/directory/symlink rejection;
- 65,537-byte and NUL rejection;
- temporary cleanup after failure;
- no-clobber race test;
- final file hash/length verification;
- evidence contains no content.

### Agent and recovery

- initial Assist turn returns `waiting_approval` and writes nothing;
- denial cancels and writes nothing;
- approval resumes and completes;
- exact action executes once;
- restart can approve pending request;
- executing recovery recognizes matching completed file;
- absent target retries safely;
- conflicting target fails closed;
- model continuation sees the write result;
- evidence ordering is request -> decision -> execution.

### API, React, and Tauri

- autonomy must be explicit and validated;
- pending approval returns HTTP 200;
- list/get/decision endpoints;
- duplicate and expired decisions;
- UI restores pending request after reload;
- UI does not render pending content;
- Approve/Deny behavior;
- Tauri compile and command tests.

## 22. Verification gates

Merge eligibility requires current evidence for:

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

Add targeted security tests to the Rust suite. Run Fallow if installed/configured for changed TypeScript. Run SonarQube only if a real project is configured; otherwise record that it was unavailable.

## 23. Temporary compatibility boundary and retirement criterion

The encrypted `PendingTurnCheckpoint` lives in the approval row because the current Mission Engine does not yet have a general durable checkpoint repository.

This is an explicit temporary boundary, not a competing mission store.

Retirement criterion:

- when Phase 4 introduces canonical durable mission checkpoints, move encrypted conversation/continuation state into the Mission repository;
- replace the approval ciphertext columns with a `mission_checkpoint_id` reference;
- migrate pending rows transactionally;
- remove approval-owned continuation serialization only after restart/resume differential tests pass.

Approval status, digest, summary, actor, and decision history remain Policy Kernel state.

## 24. Non-goals and honest limitations

This slice does not provide:

- overwrite, patch, append, delete, rename, or directory creation;
- process/shell execution;
- autonomous write grants;
- remote multi-user authentication;
- key rotation UI;
- protection from an attacker controlling the current OS user or process memory;
- full mission durability beyond the encrypted pending approval continuation;
- universal atomic semantics on filesystems that lack a safe no-clobber primitive.

Unsupported environments fail closed for mutation.

## 25. Acceptance criteria

The slice is accepted only when:

- pending approvals survive restart;
- plaintext pending content is encrypted at rest and absent from evidence/logs;
- a trusted human decision is required;
- approval is exact, expiring, and single-use;
- the write is create-only and no-clobber;
- interrupted execution recovery is proven;
- result/evidence/model/API/UI behavior matches this document;
- all security and regression tests pass;
- branch, PR, and post-merge CI are green;
- canonical state, roadmap, handoff, and evidence documents are updated with exact identifiers and limitations.
