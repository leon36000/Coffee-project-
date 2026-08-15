# HermesClaw `workspace.write` + Durable Approval Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a create-only `workspace.write` capability that can execute only after an exact durable human approval, survives restart through an encrypted pending-turn checkpoint, publishes without overwriting, and records no plaintext file content in SQLite evidence or approval metadata.

**Architecture:** Extend the existing Rust-first spine rather than adding a sidecar. `hc-policy` owns action digests, approval lifecycle, key-provider contracts, authenticated encryption, and the repository trait; `hc-state` implements that trait in the same SQLite database used by evidence; `hc-agent` pauses and resumes one provider-neutral turn; `hc-tools` performs workspace-confined atomic create-new execution; Axum, Tauri, and React project the same canonical state.

**Tech Stack:** Rust 1.97.1, serde/serde_json, RFC 8785 JCS, SHA-256, XChaCha20-Poly1305, zeroize, base64, rusqlite/SQLite, tempfile, Tokio, Axum, React 19, TypeScript, Vitest, Tauri 2, OS credential storage via `keyring`.

## Global Constraints

- Official repository: `leon36000/Coffee-project-`; implementation continues from `agent/workspace-write-design` after the approved design commit `788f0ac8f3c39541420a29893c5fcd465099f41d`.
- Rust `1.97.1` remains the minimum supported toolchain.
- `workspace.write` supports only `mode == "create_new"`; overwrite, append, patch, delete, rename, directory creation, and process execution are out of scope.
- Maximum content is exactly `65_536` UTF-8 bytes; `65_537` bytes fail.
- NUL content fails; no plaintext content may enter approval summaries, evidence, logs, public errors, or UI telemetry.
- The target parent must already exist and canonically remain inside the workspace; the target must not exist, including as a symlink.
- Observe always denies mutation. Assist requires an exact verified human approval. Autonomous Scoped never auto-allows this write; a matching human approval may authorize the exact action, preserving the design rule that scoped autonomy alone is insufficient.
- A model/client cannot deserialize, construct, or supply `VerifiedApproval`.
- Approval requests persist for exactly 24 hours by default, survive restart, and are single-use.
- Pending continuation state is encrypted with XChaCha20-Poly1305 and authenticated row-bound associated data; SQLite never stores the key.
- Desktop/local keys come from the operating-system credential store; headless mutation requires a valid base64 32-byte `HERMESCLAW_APPROVAL_KEY`; tests inject a deterministic key.
- Approval and evidence state share one SQLite connection/database; no duplicate durable store is allowed.
- If a model output contains a consequential call alongside any other call, the Agent Engine fails closed before executing the batch. The first approval slice supports one pending consequential call per model turn.
- All production behavior follows failing test -> observed failure -> minimal implementation -> green test -> refactor -> full verification.
- Do not update canonical facts/current state as if the capability were complete until branch, PR, and post-merge verification are current and green.

## File Map

- `crates/hc-domain/src/lib.rs`, `crates/hc-domain/src/approval.rs` — approval IDs/status/public prompts/decisions and canonical write constructor.
- `crates/hc-mission/src/lib.rs` — restore a mission at `waiting_approval` with original IDs.
- `crates/hc-policy/src/lib.rs`, `crates/hc-policy/src/digest.rs`, `crates/hc-policy/src/approval.rs`, `crates/hc-policy/src/crypto.rs` — policy context, opaque verified token, JCS digest, key providers, encryption, service, and repository contract.
- `crates/hc-state/src/lib.rs`, `crates/hc-state/src/approval.rs` — shared SQLite state handle, approval schema, transactions, concurrency, expiration, ciphertext erasure.
- `crates/hc-tools/src/lib.rs`, `crates/hc-tools/src/workspace_write.rs` — create-target boundary, approval summary, atomic no-clobber write, recovery inspection.
- `crates/hc-models/src/lib.rs` — deterministic/OpenAI `workspace.write` codecs.
- `crates/hc-agent/src/lib.rs`, `crates/hc-agent/src/approval.rs` — encrypted checkpoint, waiting outcome, approval/denial, continuation, restart recovery.
- `crates/hc-api/src/lib.rs`, `crates/hc-api/src/main.rs`, `crates/hc-api/tests/approval_slice.rs` — explicit autonomy and approval endpoints with headless key setup.
- `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src-tauri/src/approval_key.rs`, `apps/desktop/src-tauri/Cargo.toml` — keyring provider and Tauri commands.
- `apps/web/src/api.ts`, `apps/web/src/App.tsx`, `apps/web/src/App.test.tsx`, `apps/web/src/app.css` — profile selector, durable pending approvals, decision UI, security assertions.
- root `Cargo.toml`, `Cargo.lock`, affected crate manifests — dependency graph.
- `docs/evidence/2026-08-14-workspace-write-approval-verification.md`, `source/01_CANONICAL_FACTS.md`, `source/03_CURRENT_STATE.md`, `source/09_ROADMAP.md`, `source/11_HANDOFF.md`, `source/SOURCE_MANIFEST.json`, `README.md` — verified write-back only after all gates.

---

### Task 1: Add canonical approval and write domain contracts

**Files:**
- Create: `crates/hc-domain/src/approval.rs`
- Modify: `crates/hc-domain/src/lib.rs`
- Modify: `crates/hc-mission/src/lib.rs`
- Test: `crates/hc-domain/src/lib.rs`
- Test: `crates/hc-mission/src/lib.rs`

**Interfaces:**
- Consumes: existing UUID-backed `TraceId`/`MissionId`, `ToolCall`, `MissionState`.
- Produces: `ApprovalId`, `ApprovalStatus`, `ApprovalDecision`, `ApprovalRequest`, `ApprovalPrompt`, `ToolCall::workspace_write_create`, and `Mission::restore`.

- [ ] **Step 1: Write failing approval-ID/status serialization tests**

Create `crates/hc-domain/src/approval.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approval_status_and_decision_serialize_to_stable_snake_case() {
        assert_eq!(
            serde_json::to_string(&ApprovalStatus::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&ApprovalDecision::Approve).unwrap(),
            "\"approve\""
        );
    }

    #[test]
    fn approval_id_round_trips_through_string() {
        let id = ApprovalId::new();
        assert_eq!(id.to_string().parse::<ApprovalId>().unwrap(), id);
    }
}
```

The intentional pre-implementation compilation failure for missing types is the RED signal.

- [ ] **Step 2: Write the failing write-constructor test**

Add to `hc-domain` tests:

```rust
#[test]
fn workspace_write_constructor_is_medium_risk_create_only_mutation() {
    let call = ToolCall::workspace_write_create(
        "call-write",
        "notes/new-file.txt",
        "hello",
    );

    assert_eq!(call.capability_id, "workspace.write");
    assert_eq!(call.risk, RiskClass::Medium);
    assert_eq!(call.side_effect, SideEffectClass::Mutation);
    assert_eq!(
        call.arguments,
        serde_json::json!({
            "path": "notes/new-file.txt",
            "content": "hello",
            "mode": "create_new"
        })
    );
    assert_eq!(call.provenance.trust, TrustLevel::ModelGenerated);
}
```

- [ ] **Step 3: Write the failing mission-restore test**

Add to `hc-mission` tests:

```rust
#[test]
fn mission_restores_waiting_approval_with_original_identity() {
    let id = MissionId::new();
    let mission = Mission::restore(id, "Create notes.txt", MissionState::WaitingApproval)
        .expect("restore mission");

    assert_eq!(mission.id(), id);
    assert_eq!(mission.objective(), "Create notes.txt");
    assert_eq!(mission.state(), MissionState::WaitingApproval);
}
```

Also add a test rejecting restore from an empty objective:

```rust
#[test]
fn mission_restore_rejects_empty_objective() {
    let error = Mission::restore(MissionId::new(), "", MissionState::WaitingApproval)
        .unwrap_err();
    assert_eq!(error.to_string(), "mission objective must not be empty");
}
```

- [ ] **Step 4: Run focused tests and observe RED**

```bash
cargo test -p hc-domain approval
cargo test -p hc-domain workspace_write_constructor
cargo test -p hc-mission mission_restore
```

Expected: compilation failures for missing approval module/types, write constructor, and restore API.

- [ ] **Step 5: Implement the domain types**

`crates/hc-domain/src/approval.rs`:

```rust
use crate::{MissionId, TraceId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fmt, str::FromStr};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ApprovalId(Uuid);

impl ApprovalId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for ApprovalId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ApprovalId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl FromStr for ApprovalId {
    type Err = uuid::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(value)?))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    Executing,
    Consumed,
    Denied,
    Expired,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    Approve,
    Deny,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub approval_id: ApprovalId,
    pub trace_id: TraceId,
    pub mission_id: MissionId,
    pub capability_id: String,
    pub action_digest: String,
    pub reason: String,
    pub summary: Value,
    pub status: ApprovalStatus,
    pub requested_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub decided_at: Option<DateTime<Utc>>,
    pub decision_actor: Option<String>,
    pub failure_code: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApprovalPrompt {
    pub approval_id: ApprovalId,
    pub capability_id: String,
    pub reason: String,
    pub summary: Value,
    pub expires_at: DateTime<Utc>,
}
```

Export with `mod approval; pub use approval::*;`.

Add the write constructor beside read/list:

```rust
pub fn workspace_write_create(
    id: impl Into<String>,
    path: impl Into<String>,
    content: impl Into<String>,
) -> Self {
    Self::new(
        id,
        "workspace.write",
        serde_json::json!({
            "path": path.into(),
            "content": content.into(),
            "mode": "create_new"
        }),
        RiskClass::Medium,
        SideEffectClass::Mutation,
        Provenance::new("model", TrustLevel::ModelGenerated),
    )
}
```

- [ ] **Step 6: Implement mission restore**

Add:

```rust
pub fn restore(
    id: MissionId,
    objective: impl Into<String>,
    state: MissionState,
) -> Result<Self, MissionError> {
    let objective = objective.into();
    if objective.trim().is_empty() {
        return Err(MissionError::EmptyObjective);
    }
    Ok(Self { id, objective, state })
}
```

Add:

```rust
#[error("mission objective must not be empty")]
EmptyObjective,
```

Also make `Mission::new` reject no input indirectly at the Agent/API boundary; do not change its signature in this task.

- [ ] **Step 7: Run domain/mission suites and observe GREEN**

```bash
cargo test -p hc-domain
cargo test -p hc-mission
cargo clippy -p hc-domain -p hc-mission --all-targets --locked -- -D warnings
```

Expected: all tests pass without warnings.

- [ ] **Step 8: Commit**

```bash
git add crates/hc-domain/src/approval.rs crates/hc-domain/src/lib.rs crates/hc-mission/src/lib.rs
git commit -m "feat(domain): add durable approval contracts"
```

---

### Task 2: Add exact action digests and approval-aware policy

**Files:**
- Create: `crates/hc-policy/src/digest.rs`
- Create: `crates/hc-policy/src/approval_token.rs`
- Modify: `crates/hc-policy/src/lib.rs`
- Modify: `crates/hc-policy/Cargo.toml`
- Modify: root `Cargo.toml`
- Modify: `Cargo.lock`
- Test: `crates/hc-policy/src/digest.rs`
- Test: `crates/hc-policy/src/lib.rs`

**Interfaces:**
- Consumes: `ToolCall`, risk/side-effect/provenance, approval IDs.
- Produces: `ActionDigest`, opaque `VerifiedApproval`, `PolicyContext`, `PolicyKernel::evaluate_with_context`.

- [ ] **Step 1: Add dependency declarations**

Root workspace dependencies:

```toml
serde_jcs = "0.1"
```

`crates/hc-policy/Cargo.toml`:

```toml
serde.workspace = true
serde_jcs.workspace = true
sha2.workspace = true
hex.workspace = true
thiserror.workspace = true
```

Cargo already contains `sha2` and `hex`; run Cargo rather than hand-writing registry checksums.

- [ ] **Step 2: Write failing JCS digest tests**

`digest.rs` tests:

```rust
#[test]
fn digest_is_stable_across_json_key_order() {
    let left = ToolCall::new(
        "call-1",
        "workspace.write",
        json!({"path": "notes.txt", "content": "hello", "mode": "create_new"}),
        RiskClass::Medium,
        SideEffectClass::Mutation,
        Provenance::new("model", TrustLevel::ModelGenerated),
    );
    let right = ToolCall::new(
        "call-1",
        "workspace.write",
        json!({"mode": "create_new", "content": "hello", "path": "notes.txt"}),
        RiskClass::Medium,
        SideEffectClass::Mutation,
        Provenance::new("model", TrustLevel::ModelGenerated),
    );

    assert_eq!(ActionDigest::for_call(&left).unwrap(), ActionDigest::for_call(&right).unwrap());
}

#[test]
fn any_action_change_changes_digest() {
    let original = ToolCall::workspace_write_create("call-1", "notes.txt", "hello");
    let changed_content = ToolCall::workspace_write_create("call-1", "notes.txt", "Hello");
    let changed_path = ToolCall::workspace_write_create("call-1", "other.txt", "hello");
    let changed_call = ToolCall::workspace_write_create("call-2", "notes.txt", "hello");

    let digest = ActionDigest::for_call(&original).unwrap();
    assert_ne!(digest, ActionDigest::for_call(&changed_content).unwrap());
    assert_ne!(digest, ActionDigest::for_call(&changed_path).unwrap());
    assert_ne!(digest, ActionDigest::for_call(&changed_call).unwrap());
    assert_eq!(digest.as_str().len(), 64);
    assert_eq!(ActionDigest::parse(digest.as_str()).unwrap(), digest);
    assert!(ActionDigest::parse("ABC").is_err());
}
```

- [ ] **Step 3: Write failing approval-aware policy tests**

```rust
#[test]
fn assist_requires_approval_for_workspace_write() {
    let call = ToolCall::workspace_write_create("call-1", "notes.txt", "hello");
    let decision = PolicyKernel::evaluate(AutonomyProfile::Assist, &call);
    assert_eq!(
        decision,
        PolicyDecision::RequiresApproval(
            "assist profile requires approval for consequential actions".into()
        )
    );
}

#[test]
fn matching_verified_approval_allows_exact_write_once_policy_side() {
    let call = ToolCall::workspace_write_create("call-1", "notes.txt", "hello");
    let digest = ActionDigest::for_call(&call).unwrap();
    let approval = VerifiedApproval::for_test(ApprovalId::new(), digest);
    let context = PolicyContext::new(AutonomyProfile::Assist).with_approval(&approval);

    assert_eq!(PolicyKernel::evaluate_with_context(context, &call), PolicyDecision::Allow);
}

#[test]
fn verified_approval_does_not_authorize_changed_call() {
    let approved = ToolCall::workspace_write_create("call-1", "notes.txt", "hello");
    let changed = ToolCall::workspace_write_create("call-1", "notes.txt", "changed");
    let approval = VerifiedApproval::for_test(
        ApprovalId::new(),
        ActionDigest::for_call(&approved).unwrap(),
    );
    let context = PolicyContext::new(AutonomyProfile::Assist).with_approval(&approval);

    assert!(matches!(
        PolicyKernel::evaluate_with_context(context, &changed),
        PolicyDecision::RequiresApproval(_)
    ));
}

#[test]
fn autonomous_scoped_write_never_auto_allows_but_accepts_exact_human_approval() {
    let call = ToolCall::workspace_write_create("call-1", "notes.txt", "hello");
    assert!(matches!(
        PolicyKernel::evaluate(AutonomyProfile::AutonomousScoped, &call),
        PolicyDecision::RequiresApproval(_)
    ));

    let approval = VerifiedApproval::for_test(
        ApprovalId::new(),
        ActionDigest::for_call(&call).unwrap(),
    );
    let context = PolicyContext::new(AutonomyProfile::AutonomousScoped)
        .with_approval(&approval);
    assert_eq!(
        PolicyKernel::evaluate_with_context(context, &call),
        PolicyDecision::Allow
    );
}
```

`for_test` must be `#[cfg(test)] pub(crate)` so production callers cannot mint the token.

- [ ] **Step 4: Run focused tests and observe RED**

```bash
cargo test -p hc-policy digest
cargo test -p hc-policy approval
```

Expected: missing digest/token/context APIs.

- [ ] **Step 5: Implement `ActionDigest`**

```rust
use hc_domain::{Provenance, RiskClass, SideEffectClass, ToolCall};
use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ActionDigest(String);

#[derive(Serialize)]
struct ActionDigestMaterial<'a> {
    schema_version: u8,
    call_id: &'a str,
    capability_id: &'a str,
    arguments: &'a serde_json::Value,
    risk: RiskClass,
    side_effect: SideEffectClass,
    provenance: &'a Provenance,
}

impl ActionDigest {
    pub fn for_call(call: &ToolCall) -> Result<Self, DigestError> {
        let material = ActionDigestMaterial {
            schema_version: 1,
            call_id: &call.id,
            capability_id: &call.capability_id,
            arguments: &call.arguments,
            risk: call.risk,
            side_effect: call.side_effect,
            provenance: &call.provenance,
        };
        let canonical = serde_jcs::to_vec(&material)
            .map_err(|error| DigestError::CanonicalJson(error.to_string()))?;
        Ok(Self(hex::encode(Sha256::digest(canonical))))
    }

    pub fn parse(value: impl Into<String>) -> Result<Self, DigestError> {
        let value = value.into();
        if value.len() != 64
            || !value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(DigestError::InvalidEncoding);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Error)]
pub enum DigestError {
    #[error("action digest serialization failed: {0}")]
    CanonicalJson(String),
    #[error("action digest must be 64 lowercase hexadecimal characters")]
    InvalidEncoding,
}
```

- [ ] **Step 6: Implement the opaque token and context**

```rust
use crate::ActionDigest;
use hc_domain::ApprovalId;

#[derive(Clone, Debug)]
pub struct VerifiedApproval {
    approval_id: ApprovalId,
    action_digest: ActionDigest,
}

impl VerifiedApproval {
    pub fn approval_id(&self) -> ApprovalId {
        self.approval_id
    }

    pub(crate) fn new(approval_id: ApprovalId, action_digest: ActionDigest) -> Self {
        Self { approval_id, action_digest }
    }

    pub(crate) fn matches(&self, digest: &ActionDigest) -> bool {
        &self.action_digest == digest
    }

    #[cfg(test)]
    pub(crate) fn for_test(approval_id: ApprovalId, action_digest: ActionDigest) -> Self {
        Self::new(approval_id, action_digest)
    }
}

pub struct PolicyContext<'a> {
    pub profile: AutonomyProfile,
    verified_approval: Option<&'a VerifiedApproval>,
}

impl<'a> PolicyContext<'a> {
    pub fn new(profile: AutonomyProfile) -> Self {
        Self { profile, verified_approval: None }
    }

    pub fn with_approval(mut self, approval: &'a VerifiedApproval) -> Self {
        self.verified_approval = Some(approval);
        self
    }

    pub(crate) fn verified_approval(&self) -> Option<&'a VerifiedApproval> {
        self.verified_approval
    }
}
```

Do not derive `Serialize`/`Deserialize` for `VerifiedApproval`.

- [ ] **Step 7: Implement policy evaluation**

Keep `evaluate(profile, call)` as the no-token compatibility wrapper. Add `evaluate_with_context`.

For `workspace.write`:

```rust
fn evaluate_workspace_write(context: PolicyContext<'_>, call: &ToolCall) -> PolicyDecision {
    if context.profile == AutonomyProfile::Observe {
        return PolicyDecision::Deny("observe profile forbids side effects".into());
    }

    let Ok(digest) = ActionDigest::for_call(call) else {
        return PolicyDecision::Deny("unable to bind approval to action".into());
    };

    if context
        .verified_approval()
        .is_some_and(|approval| approval.matches(&digest))
    {
        return PolicyDecision::Allow;
    }

    PolicyDecision::RequiresApproval(
        "assist profile requires approval for consequential actions".into(),
    )
}
```

For all other calls, preserve existing profile behavior.

- [ ] **Step 8: Run policy tests and strict Clippy**

```bash
cargo test -p hc-policy
cargo clippy -p hc-policy --all-targets --locked -- -D warnings
```

Expected: existing read-only tests and new digest/policy tests pass.

- [ ] **Step 9: Commit**

```bash
git add Cargo.toml Cargo.lock crates/hc-policy/Cargo.toml crates/hc-policy/src
git commit -m "feat(policy): bind approvals to exact actions"
```

---

### Task 3: Add authenticated checkpoint encryption and key providers

**Files:**
- Create: `crates/hc-policy/src/crypto.rs`
- Create: `crates/hc-policy/src/key_provider.rs`
- Modify: `crates/hc-policy/src/lib.rs`
- Modify: `crates/hc-policy/Cargo.toml`
- Modify: root `Cargo.toml`
- Modify: `Cargo.lock`
- Test: `crates/hc-policy/src/crypto.rs`
- Test: `crates/hc-policy/src/key_provider.rs`

**Interfaces:**
- Consumes: approval/trace/mission IDs and `ActionDigest`.
- Produces: `ApprovalKey`, `ApprovalKeyProvider`, `InMemoryApprovalKeyProvider`, `EnvApprovalKeyProvider`, `EncryptedCheckpoint`, `CheckpointCipher`.

- [ ] **Step 1: Add dependency declarations**

Root:

```toml
base64 = "0.22"
chacha20poly1305 = "0.10"
zeroize = { version = "1", features = ["derive"] }
```

`hc-policy`:

```toml
base64.workspace = true
chacha20poly1305.workspace = true
zeroize.workspace = true
```

- [ ] **Step 2: Write failing authenticated-encryption tests**

```rust
#[test]
fn encrypted_checkpoint_round_trips_and_hides_plaintext() {
    let provider = InMemoryApprovalKeyProvider::new("test-key", [7_u8; 32]);
    let cipher = CheckpointCipher::new(Arc::new(provider));
    let context = test_context();
    let plaintext = b"sentinel pending file content";

    let sealed = cipher.seal(&context, plaintext).unwrap();

    assert!(!sealed.ciphertext.windows(plaintext.len()).any(|window| window == plaintext));
    assert_eq!(cipher.open(&context, &sealed).unwrap(), plaintext);
}

#[test]
fn wrong_associated_data_or_key_fails_closed() {
    let cipher = CheckpointCipher::new(Arc::new(
        InMemoryApprovalKeyProvider::new("key-a", [1_u8; 32]),
    ));
    let context = test_context();
    let sealed = cipher.seal(&context, b"secret").unwrap();

    let mut changed = context.clone();
    changed.action_digest = ActionDigest::for_call(&ToolCall::workspace_write_create(
        "changed", "notes.txt", "secret",
    ))
    .unwrap();
    assert!(matches!(
        cipher.open(&changed, &sealed),
        Err(ApprovalCryptoError::AuthenticationFailed)
    ));

    let wrong_key = CheckpointCipher::new(Arc::new(
        InMemoryApprovalKeyProvider::new("key-a", [2_u8; 32]),
    ));
    assert!(matches!(
        wrong_key.open(&context, &sealed),
        Err(ApprovalCryptoError::AuthenticationFailed)
    ));
}
```

Add a row-swap test using two different approval IDs and the same key.

Use this exact helper in the crypto test module:

```rust
fn test_context() -> CheckpointContext {
    let call = ToolCall::workspace_write_create("call-1", "notes.txt", "secret");
    CheckpointContext {
        approval_id: ApprovalId::new(),
        trace_id: TraceId::new(),
        mission_id: MissionId::new(),
        action_digest: ActionDigest::for_call(&call).unwrap(),
        schema_version: 1,
    }
}
```

Add `Clone` to `CheckpointContext` so tests can alter one AAD field without generating unrelated IDs.

- [ ] **Step 3: Write failing environment-key tests**

Use an injectable string constructor instead of mutating process-global environment in parallel tests:

```rust
#[test]
fn env_provider_accepts_exactly_32_decoded_bytes() {
    let encoded = base64::engine::general_purpose::STANDARD.encode([9_u8; 32]);
    let provider = EnvApprovalKeyProvider::from_encoded("env-v1", &encoded).unwrap();
    assert_eq!(provider.active_key().unwrap().id(), "env-v1");
}

#[test]
fn env_provider_rejects_wrong_length_and_invalid_base64() {
    assert!(matches!(
        EnvApprovalKeyProvider::from_encoded("env-v1", "%%%"),
        Err(ApprovalKeyError::InvalidEncoding)
    ));
    let short = base64::engine::general_purpose::STANDARD.encode([1_u8; 31]);
    assert!(matches!(
        EnvApprovalKeyProvider::from_encoded("env-v1", &short),
        Err(ApprovalKeyError::InvalidLength { actual: 31 })
    ));
}
```

- [ ] **Step 4: Run focused tests and observe RED**

```bash
cargo test -p hc-policy crypto
cargo test -p hc-policy key_provider
```

Expected: missing crypto/key APIs.

- [ ] **Step 5: Implement key-provider types**

```rust
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::sync::Arc;
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct ApprovalKey {
    id: String,
    bytes: [u8; 32],
}

impl ApprovalKey {
    pub fn new(id: impl Into<String>, bytes: [u8; 32]) -> Self {
        Self { id: id.into(), bytes }
    }
    pub fn id(&self) -> &str { &self.id }
    pub(crate) fn bytes(&self) -> &[u8; 32] { &self.bytes }
}

pub trait ApprovalKeyProvider: Send + Sync {
    fn active_key(&self) -> Result<ApprovalKey, ApprovalKeyError>;
    fn key_by_id(&self, key_id: &str) -> Result<ApprovalKey, ApprovalKeyError>;
}
```

Implement deterministic in-memory and base64 environment providers. `EnvApprovalKeyProvider::from_env()` reads `HERMESCLAW_APPROVAL_KEY` and delegates to `from_encoded("env-v1", value)`.

- [ ] **Step 6: Implement XChaCha20-Poly1305 sealing**

```rust
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng, Payload},
    XChaCha20Poly1305, XNonce,
};
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncryptedCheckpoint {
    pub ciphertext: Vec<u8>,
    pub nonce: [u8; 24],
    pub key_id: String,
}

#[derive(Clone, Debug)]
pub struct CheckpointContext {
    pub approval_id: ApprovalId,
    pub trace_id: TraceId,
    pub mission_id: MissionId,
    pub action_digest: ActionDigest,
    pub schema_version: u8,
}

#[derive(Clone)]
pub struct CheckpointCipher {
    keys: Arc<dyn ApprovalKeyProvider>,
}
```

Canonical associated data:

```rust
#[derive(Serialize)]
struct AssociatedData<'a> {
    approval_id: String,
    trace_id: String,
    mission_id: String,
    action_digest: &'a str,
    schema_version: u8,
}
```

Use `serde_jcs::to_vec` for AAD, `XChaCha20Poly1305::generate_nonce(&mut OsRng)`, and `Payload { msg, aad }`. Map all decrypt/auth failures to one public `AuthenticationFailed` variant; do not leak which field differed.

- [ ] **Step 7: Run crypto tests and strict Clippy**

```bash
cargo test -p hc-policy crypto
cargo test -p hc-policy key_provider
cargo clippy -p hc-policy --all-targets --locked -- -D warnings
```

Expected: all pass.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml Cargo.lock crates/hc-policy/Cargo.toml crates/hc-policy/src
git commit -m "feat(policy): encrypt durable approval checkpoints"
```

---

### Task 4: Add the approval repository contract and shared SQLite state

**Files:**
- Create: `crates/hc-policy/src/approval.rs`
- Create: `crates/hc-state/src/approval.rs`
- Modify: `crates/hc-policy/src/lib.rs`
- Modify: `crates/hc-state/src/lib.rs`
- Modify: `crates/hc-state/Cargo.toml`
- Test: `crates/hc-state/src/approval.rs`
- Test: `crates/hc-state/src/lib.rs`

**Interfaces:**
- Consumes: `ApprovalRequest`, encrypted checkpoints, current evidence schema.
- Produces: `ApprovalRepository`, `ApprovalService`, `SqliteState`, `SqliteApprovalRepository`; evidence and approvals share one connection.

- [ ] **Step 1: Define failing repository behavior tests**

Create tests against a file-backed `SqliteState`:

```rust
#[test]
fn pending_approval_survives_database_reopen_without_plaintext() {
    let directory = tempdir().unwrap();
    let database = directory.path().join("hermesclaw.db");
    let plaintext = b"sentinel pending content";

    {
        let state = SqliteState::open(&database).unwrap();
        let repository = state.approval_repository();
        let request = test_request();
        let checkpoint = encrypted_checkpoint_containing_no_plaintext(&request, plaintext);
        repository
            .create_pending(&request, &checkpoint)
            .unwrap();
    }

    let database_bytes = fs::read(&database).unwrap();
    assert!(!database_bytes.windows(plaintext.len()).any(|window| window == plaintext));

    let state = SqliteState::open(&database).unwrap();
    let pending = state.approval_repository().list_pending(Utc::now()).unwrap();
    assert_eq!(pending.len(), 1);
}
```

Concurrency/single-use:

```rust
#[test]
fn begin_execution_is_single_winner_across_connections() {
    let directory = tempdir().unwrap();
    let database = directory.path().join("state.db");
    let request = test_request();
    SqliteState::open(&database)
        .unwrap()
        .approval_repository()
        .create_pending(&request, &test_encrypted_checkpoint())
        .unwrap();

    let barrier = Arc::new(Barrier::new(3));
    let results = (0..2)
        .map(|index| {
            let database = database.clone();
            let barrier = Arc::clone(&barrier);
            let approval_id = request.approval_id;
            thread::spawn(move || {
                let repository = SqliteState::open(database)
                    .unwrap()
                    .approval_repository();
                barrier.wait();
                repository.begin_execution(
                    approval_id,
                    &format!("actor-{index}"),
                    Utc::now(),
                )
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();

    let outcomes = results
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
    assert_eq!(outcomes.iter().filter(|outcome| matches!(
        outcome,
        Err(ApprovalRepositoryError::AlreadyDecided)
    )).count(), 1);
}
```

Expiration and erasure tests:

```rust
#[test]
fn expired_denied_and_consumed_rows_erase_ciphertext() {
    let now = Utc::now();
    let state = SqliteState::in_memory().unwrap();
    let repository = state.approval_repository();

    let denied = test_request_with_expiry(ApprovalId::new(), now + Duration::hours(1));
    repository
        .create_pending(&denied, &test_encrypted_checkpoint())
        .unwrap();
    repository
        .mark_denied(denied.approval_id, "local_user", now)
        .unwrap();

    let consumed = test_request_with_expiry(ApprovalId::new(), now + Duration::hours(1));
    repository
        .create_pending(&consumed, &test_encrypted_checkpoint())
        .unwrap();
    repository
        .begin_execution(consumed.approval_id, "local_user", now)
        .unwrap();
    repository.mark_consumed(consumed.approval_id, now).unwrap();

    let expired = test_request_with_expiry(ApprovalId::new(), now - Duration::seconds(1));
    repository
        .create_pending(&expired, &test_encrypted_checkpoint())
        .unwrap();
    assert_eq!(repository.expire_due(now).unwrap(), 1);

    for approval_id in [denied.approval_id, consumed.approval_id, expired.approval_id] {
        assert_eq!(
            repository.checkpoint_presence_for_test(approval_id).unwrap(),
            (false, false, false)
        );
    }
}
```

Add `checkpoint_presence_for_test` under `#[cfg(test)]`; it returns booleans for ciphertext, nonce, and key ID using one direct query.

Use these exact test helpers in `hc-state/src/approval.rs`:

```rust
fn test_request_with_expiry(
    approval_id: ApprovalId,
    expires_at: DateTime<Utc>,
) -> ApprovalRequest {
    let call = ToolCall::workspace_write_create("call-1", "notes.txt", "sentinel pending content");
    ApprovalRequest {
        approval_id,
        trace_id: TraceId::new(),
        mission_id: MissionId::new(),
        capability_id: call.capability_id.clone(),
        action_digest: ActionDigest::for_call(&call).unwrap().as_str().to_owned(),
        reason: "assist profile requires approval for consequential actions".into(),
        summary: json!({
            "path": "notes.txt",
            "mode": "create_new",
            "bytes": 24,
            "sha256": "f".repeat(64)
        }),
        status: ApprovalStatus::Pending,
        requested_at: expires_at - Duration::hours(1),
        expires_at,
        decided_at: None,
        decision_actor: None,
        failure_code: None,
    }
}

fn test_request() -> ApprovalRequest {
    test_request_with_expiry(ApprovalId::new(), Utc::now() + Duration::hours(1))
}

fn test_encrypted_checkpoint() -> EncryptedCheckpoint {
    EncryptedCheckpoint {
        ciphertext: vec![1, 2, 3, 4],
        nonce: [5_u8; 24],
        key_id: "test-key".into(),
    }
}

fn encrypted_checkpoint_containing_no_plaintext(
    request: &ApprovalRequest,
    plaintext: &[u8],
) -> EncryptedCheckpoint {
    let provider = InMemoryApprovalKeyProvider::new("test-key", [7_u8; 32]);
    let cipher = CheckpointCipher::new(Arc::new(provider));
    let context = CheckpointContext {
        approval_id: request.approval_id,
        trace_id: request.trace_id,
        mission_id: request.mission_id,
        action_digest: ActionDigest::parse(&request.action_digest).unwrap(),
        schema_version: 1,
    };
    cipher.seal(&context, plaintext).unwrap()
}
```

In the reopen/plaintext test, replace `test_encrypted_checkpoint()` with `encrypted_checkpoint_containing_no_plaintext(&request, plaintext)` and persist that exact value.

- [ ] **Step 2: Run state tests and observe RED**

```bash
cargo test -p hc-state approval
```

Expected: missing state/repository APIs.

- [ ] **Step 3: Define the policy-owned repository contract**

`hc-policy/src/approval.rs`:

```rust
pub struct StoredApproval {
    pub request: ApprovalRequest,
    pub encrypted_checkpoint: EncryptedCheckpoint,
}

pub trait ApprovalRepository: Send + Sync {
    fn create_pending(
        &self,
        request: &ApprovalRequest,
        checkpoint: &EncryptedCheckpoint,
    ) -> Result<(), ApprovalRepositoryError>;
    fn list_pending(&self, now: DateTime<Utc>)
        -> Result<Vec<ApprovalRequest>, ApprovalRepositoryError>;
    fn load_public(
        &self,
        approval_id: ApprovalId,
        now: DateTime<Utc>,
    ) -> Result<ApprovalRequest, ApprovalRepositoryError>;
    fn begin_execution(
        &self,
        approval_id: ApprovalId,
        actor: &str,
        now: DateTime<Utc>,
    ) -> Result<StoredApproval, ApprovalRepositoryError>;
    fn mark_consumed(
        &self,
        approval_id: ApprovalId,
        now: DateTime<Utc>,
    ) -> Result<(), ApprovalRepositoryError>;
    fn mark_denied(
        &self,
        approval_id: ApprovalId,
        actor: &str,
        now: DateTime<Utc>,
    ) -> Result<ApprovalRequest, ApprovalRepositoryError>;
    fn mark_failed(
        &self,
        approval_id: ApprovalId,
        failure_code: &str,
        now: DateTime<Utc>,
        erase_checkpoint: bool,
    ) -> Result<(), ApprovalRepositoryError>;
    fn expire_due(&self, now: DateTime<Utc>)
        -> Result<usize, ApprovalRepositoryError>;
    fn load_executing_for_recovery(
        &self,
    ) -> Result<Vec<StoredApproval>, ApprovalRepositoryError>;
}
```

Use this exact error surface:

```rust
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ApprovalRepositoryError {
    #[error("approval not found")]
    NotFound,
    #[error("approval expired")]
    Expired,
    #[error("approval already decided")]
    AlreadyDecided,
    #[error("approval already exists")]
    Duplicate,
    #[error("approval repository backend error: {0}")]
    Backend(String),
}
```

- [ ] **Step 4: Refactor state into one shared connection**

```rust
#[derive(Clone)]
pub struct SqliteState {
    connection: Arc<Mutex<Connection>>,
}

impl SqliteState {
    pub fn in_memory() -> Result<Self, StateError>;
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StateError>;
    pub fn evidence_store(&self) -> EvidenceStore;
    pub fn approval_repository(&self) -> SqliteApprovalRepository;
}

#[derive(Clone)]
pub struct EvidenceStore {
    connection: Arc<Mutex<Connection>>,
}
```

Keep `EvidenceStore::in_memory/open` as compatibility constructors by delegating through `SqliteState`. Existing callers remain green while application entry points later switch to `SqliteState`.

- [ ] **Step 5: Add the approval schema**

Set `connection.busy_timeout(std::time::Duration::from_secs(5))?` before migrations so concurrent conditional decisions wait instead of producing spurious lock errors. Initialize in the same `from_connection` migration:

```sql
CREATE TABLE IF NOT EXISTS approvals (
    approval_id TEXT PRIMARY KEY,
    trace_id TEXT NOT NULL,
    mission_id TEXT NOT NULL,
    capability_id TEXT NOT NULL,
    action_digest TEXT NOT NULL,
    reason TEXT NOT NULL,
    summary TEXT NOT NULL,
    status TEXT NOT NULL,
    requested_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    decided_at TEXT,
    decision_actor TEXT,
    failure_code TEXT,
    ciphertext BLOB,
    nonce BLOB,
    key_id TEXT
);
CREATE INDEX IF NOT EXISTS idx_approvals_status_expiry
    ON approvals(status, expires_at);
```

Serialize IDs as plain display strings, status as stable snake_case, summary JSON text, timestamps RFC3339, nonce exactly 24 bytes.

- [ ] **Step 6: Implement conditional transitions**

`begin_execution` uses `TransactionBehavior::Immediate`:

1. read row;
2. if pending and expired, set `expired`, null checkpoint columns, commit, return `Expired`;
3. conditional update `WHERE approval_id = ? AND status = 'pending'` to `executing`, actor, decided_at;
4. require one changed row;
5. return request + checkpoint.

`mark_denied`, `mark_consumed`, terminal `mark_failed`, and `expire_due` null `ciphertext`, `nonce`, and `key_id` in the same transaction.

- [ ] **Step 7: Implement `ApprovalService`**

```rust
#[derive(Clone)]
pub struct ApprovalService {
    repository: Arc<dyn ApprovalRepository>,
    cipher: CheckpointCipher,
}

pub struct NewApproval<'a> {
    pub trace_id: TraceId,
    pub mission_id: MissionId,
    pub call: &'a ToolCall,
    pub reason: &'a str,
    pub summary: serde_json::Value,
    pub checkpoint: &'a [u8],
    pub requested_at: DateTime<Utc>,
    pub lifetime: chrono::Duration,
}

pub struct ApprovedCheckpoint {
    pub request: ApprovalRequest,
    pub verified_approval: VerifiedApproval,
    pub plaintext: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum ApprovalError {
    #[error(transparent)]
    Repository(#[from] ApprovalRepositoryError),
    #[error(transparent)]
    Digest(#[from] DigestError),
    #[error(transparent)]
    Crypto(#[from] ApprovalCryptoError),
    #[error(transparent)]
    Key(#[from] ApprovalKeyError),
    #[error("approval checkpoint action digest mismatch")]
    DigestMismatch,
    #[error("approval checkpoint serialization failed: {0}")]
    Serialization(String),
}

impl ApprovalService {
    pub fn new(
        repository: Arc<dyn ApprovalRepository>,
        keys: Arc<dyn ApprovalKeyProvider>,
    ) -> Self;

    pub fn create_pending(
        &self,
        new_approval: NewApproval<'_>,
    ) -> Result<ApprovalPrompt, ApprovalError>;

    pub fn list_pending(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<ApprovalRequest>, ApprovalError>;

    pub fn load_public(
        &self,
        approval_id: ApprovalId,
        now: DateTime<Utc>,
    ) -> Result<ApprovalRequest, ApprovalError>;

    pub fn begin_execution(
        &self,
        approval_id: ApprovalId,
        actor: &str,
        now: DateTime<Utc>,
    ) -> Result<ApprovedCheckpoint, ApprovalError>;

    pub fn deny(
        &self,
        approval_id: ApprovalId,
        actor: &str,
        now: DateTime<Utc>,
    ) -> Result<ApprovalRequest, ApprovalError>;

    pub fn mark_consumed(
        &self,
        approval_id: ApprovalId,
        now: DateTime<Utc>,
    ) -> Result<(), ApprovalError>;

    pub fn mark_failed(
        &self,
        approval_id: ApprovalId,
        failure_code: &str,
        now: DateTime<Utc>,
        erase_checkpoint: bool,
    ) -> Result<(), ApprovalError>;

    pub fn executing_for_recovery(
        &self,
    ) -> Result<Vec<ApprovedCheckpoint>, ApprovalError>;
}
```

`create_pending` computes the digest, creates an `ApprovalId`, seals the checkpoint with row-bound AAD, persists it, and returns `ApprovalPrompt`. `begin_execution` loads/decrypts and constructs the opaque token. `executing_for_recovery` decrypts existing executing rows without changing status and constructs the same verified tokens. Default callers pass `Duration::hours(24)`.

- [ ] **Step 8: Run state/policy tests and full regression**

```bash
cargo test -p hc-state
cargo test -p hc-policy
cargo test --workspace --exclude hermesclaw-desktop --locked
cargo clippy -p hc-state -p hc-policy --all-targets --locked -- -D warnings
```

Expected: existing evidence tests remain green; approval persistence/concurrency/erasure tests pass.

- [ ] **Step 9: Commit**

```bash
git add crates/hc-policy/src crates/hc-state/src crates/hc-state/Cargo.toml Cargo.lock
git commit -m "feat(state): persist durable single-use approvals"
```

---

### Task 5: Add create-target validation and atomic `workspace.write`

**Files:**
- Create: `crates/hc-tools/src/workspace_write.rs`
- Modify: `crates/hc-tools/src/lib.rs`
- Modify: `crates/hc-tools/Cargo.toml`
- Modify: root `Cargo.toml`
- Modify: `Cargo.lock`
- Test: `crates/hc-tools/src/workspace_write.rs`

**Interfaces:**
- Consumes: shared `WorkspaceBoundary`, `ToolCall::workspace_write_create`.
- Produces: `CreateTarget`, `WorkspaceWriteCapability`, approval summary, atomic create result, recovery inspection.

- [ ] **Step 1: Extend the capability contract with approval preview and recovery**

Add tests first:

```rust
#[tokio::test]
async fn write_approval_summary_has_no_content() {
    let workspace = tempdir().unwrap();
    let mut registry = CapabilityRegistry::new();
    registry.register(WorkspaceWriteCapability::new(workspace.path()).unwrap());
    let call = ToolCall::workspace_write_create("call-1", "notes.txt", "sentinel secret");

    let summary = registry.approval_summary(&call).unwrap();

    assert_eq!(summary["path"], "notes.txt");
    assert_eq!(summary["mode"], "create_new");
    assert_eq!(summary["bytes"], 15);
    assert_eq!(summary["sha256"].as_str().unwrap().len(), 64);
    assert!(summary.get("content").is_none());
    assert!(!summary.to_string().contains("sentinel secret"));
}
```

Add to trait:

```rust
fn approval_summary(&self, _call: &ToolCall) -> Result<serde_json::Value, CapabilityError> {
    Err(CapabilityError::ApprovalSummaryUnavailable)
}

async fn recover(
    &self,
    _call: &ToolCall,
) -> Result<CapabilityRecovery, CapabilityError> {
    Err(CapabilityError::RecoveryUnsupported)
}
```

Registry delegates both by capability ID.

```rust
pub enum CapabilityRecovery {
    NotApplied,
    Applied(CapabilityExecution),
}
```

Conflict is a `CapabilityError::TargetConflict` rather than a third success-like state.

- [ ] **Step 2: Write failing boundary and write tests**

Add explicit tests for:

```rust
#[test]
fn create_target_rejects_absolute_parent_and_dot_components() {
    let workspace = tempdir().unwrap();
    let boundary = WorkspaceBoundary::new(workspace.path()).unwrap();
    let absolute = std::env::temp_dir().join("outside.txt");

    for requested in [
        absolute.to_string_lossy().into_owned(),
        "../outside.txt".into(),
        "./notes.txt".into(),
        "nested/../notes.txt".into(),
    ] {
        assert!(matches!(
            boundary.resolve_create_target(&requested),
            Err(CapabilityError::InvalidCreatePath)
                | Err(CapabilityError::PathEscapesWorkspace)
        ));
    }
}

#[tokio::test]
async fn workspace_write_creates_new_file_atomically_and_sanitizes_evidence() {
    let workspace = tempdir().unwrap();
    let capability = WorkspaceWriteCapability::new(workspace.path()).unwrap();
    let call = ToolCall::workspace_write_create("call-1", "notes.txt", "hello");

    let execution = capability.execute(&call).await.unwrap();

    assert_eq!(fs::read_to_string(workspace.path().join("notes.txt")).unwrap(), "hello");
    assert_eq!(execution.result.output["path"], "notes.txt");
    assert_eq!(execution.result.output["mode"], "create_new");
    assert_eq!(execution.result.output["bytes"], 5);
    assert_eq!(execution.result.output["created"], true);
    assert_eq!(execution.result.output["atomic"], true);
    assert_eq!(execution.result.output["sha256"].as_str().unwrap().len(), 64);
    assert!(execution.evidence_payload.get("content").is_none());
    assert!(!execution.evidence_payload.to_string().contains("hello"));
    assert!(fs::read_dir(workspace.path())
        .unwrap()
        .all(|entry| !entry.unwrap().file_name().to_string_lossy().contains("hermesclaw-tmp")));
}

#[tokio::test]
async fn workspace_write_rejects_existing_targets() {
    let workspace = tempdir().unwrap();
    fs::write(workspace.path().join("file.txt"), "existing").unwrap();
    fs::create_dir(workspace.path().join("directory")).unwrap();
    let capability = WorkspaceWriteCapability::new(workspace.path()).unwrap();

    for path in ["file.txt", "directory"] {
        let error = capability
            .execute(&ToolCall::workspace_write_create("call", path, "new"))
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), "target already exists");
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(workspace.path().join("file.txt"), workspace.path().join("link.txt")).unwrap();
        let error = capability
            .execute(&ToolCall::workspace_write_create("call", "link.txt", "new"))
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), "target already exists");
    }
}

#[tokio::test]
async fn workspace_write_rejects_missing_parent_escape_oversize_and_nul() {
    let workspace = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let capability = WorkspaceWriteCapability::new(workspace.path()).unwrap();

    let missing = capability
        .execute(&ToolCall::workspace_write_create("call", "missing/file.txt", "x"))
        .await
        .unwrap_err();
    assert_eq!(missing.to_string(), "parent directory does not exist");

    let escape = capability
        .execute(&ToolCall::workspace_write_create(
            "call",
            outside.path().join("file.txt").to_string_lossy(),
            "x",
        ))
        .await
        .unwrap_err();
    assert!(matches!(
        escape,
        CapabilityError::InvalidCreatePath | CapabilityError::PathEscapesWorkspace
    ));

    let oversize = capability
        .execute(&ToolCall::workspace_write_create(
            "call",
            "large.txt",
            "a".repeat(MAX_WORKSPACE_WRITE_BYTES + 1),
        ))
        .await
        .unwrap_err();
    assert_eq!(
        oversize.to_string(),
        "content exceeds 65536 byte limit: 65537 bytes"
    );

    let nul = capability
        .execute(&ToolCall::workspace_write_create("call", "nul.txt", "a\0b"))
        .await
        .unwrap_err();
    assert_eq!(nul.to_string(), "content contains a NUL byte");
}

#[tokio::test]
async fn persist_noclobber_never_overwrites_racing_target() {
    let workspace = tempdir().unwrap();
    let target = workspace.path().join("notes.txt");
    let mut temporary = tempfile::Builder::new()
        .prefix(".hermesclaw-tmp-")
        .tempfile_in(workspace.path())
        .unwrap();
    temporary.write_all(b"approved").unwrap();
    temporary.as_file().sync_all().unwrap();
    fs::write(&target, "racing content").unwrap();

    let error = persist_noclobber(temporary, &target).unwrap_err();

    assert_eq!(error.to_string(), "target already exists");
    assert_eq!(fs::read_to_string(target).unwrap(), "racing content");
}

#[tokio::test]
async fn recovery_returns_applied_only_for_matching_hash() {
    let absent_workspace = tempdir().unwrap();
    let absent = WorkspaceWriteCapability::new(absent_workspace.path()).unwrap();
    let call = ToolCall::workspace_write_create("call", "notes.txt", "hello");
    assert!(matches!(absent.recover(&call).await.unwrap(), CapabilityRecovery::NotApplied));

    let matching_workspace = tempdir().unwrap();
    fs::write(matching_workspace.path().join("notes.txt"), "hello").unwrap();
    let matching = WorkspaceWriteCapability::new(matching_workspace.path()).unwrap();
    assert!(matches!(
        matching.recover(&call).await.unwrap(),
        CapabilityRecovery::Applied(_)
    ));

    let conflict_workspace = tempdir().unwrap();
    fs::write(conflict_workspace.path().join("notes.txt"), "different").unwrap();
    let conflict = WorkspaceWriteCapability::new(conflict_workspace.path()).unwrap();
    assert_eq!(
        conflict.recover(&call).await.unwrap_err().to_string(),
        "target exists with content different from the approved action"
    );
}
```

- [ ] **Step 3: Run focused tests and observe RED**

```bash
cargo test -p hc-tools workspace_write
cargo test -p hc-tools create_target
cargo test -p hc-tools persist_noclobber
```

Expected: missing APIs and capability.

- [ ] **Step 4: Add production dependency**

Root already has `tempfile = "3"`; add `tempfile.workspace = true` to production dependencies of `hc-tools` instead of only dev dependencies.

- [ ] **Step 5: Implement create-target resolution**

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateTarget {
    pub canonical_parent: PathBuf,
    pub relative_path: String,
    pub target_path: PathBuf,
}
```

`resolve_create_target` rejects any component other than `Normal`, canonicalizes the parent, verifies containment and directory type, forms the target, and calls `symlink_metadata`. `NotFound` means available; every other success means `TargetAlreadyExists`; every other I/O error propagates.

A private `resolve_create_candidate` performs the same component/parent validation without requiring absence and is used only by recovery inspection.

- [ ] **Step 6: Implement one parser used by summary/execute/recovery**

```rust
struct PreparedWrite {
    target: CreateTarget,
    content: Vec<u8>,
    bytes: usize,
    sha256: String,
}

fn prepare_write(
    boundary: &WorkspaceBoundary,
    call: &ToolCall,
    require_absent: bool,
) -> Result<PreparedWrite, CapabilityError>;
```

Validate capability ID, exact mode, path/content strings, NUL, and byte limit before any filesystem mutation. When `require_absent` is false, use the recovery candidate resolver.

- [ ] **Step 7: Implement atomic create-new publication**

Use `tempfile::Builder` with prefix `.hermesclaw-tmp-` in the canonical parent. Call `write_all`, `flush`, `sync_all`, then `persist_noclobber`. Map `AlreadyExists` to `TargetAlreadyExists`, `Unsupported` to `AtomicPublishUnsupported`, and other errors to I/O.

After publication:

- synchronize the parent on Unix with `File::open(parent)?.sync_all()`;
- on non-Unix, return `Ok(())` from the parent-sync helper and rely on the no-clobber primitive;
- read the final file, require exact length/hash, then return result/evidence metadata.

`NamedTempFile` ownership must remain in scope until publication so all pre-publish failures remove it automatically.

- [ ] **Step 8: Implement recovery inspection**

- target absent -> `CapabilityRecovery::NotApplied`;
- regular file with exact bytes/hash -> `Applied` with the same sanitized result/evidence;
- any existing non-file, symlink, or different hash -> `TargetConflict`.

Recovery never writes by itself; the Agent Engine decides whether to call normal execution after `NotApplied`.

- [ ] **Step 9: Run tools tests and strict Clippy**

```bash
cargo test -p hc-tools
cargo clippy -p hc-tools --all-targets --locked -- -D warnings
```

Expected: old list/read tests and all create/recovery tests pass.

- [ ] **Step 10: Commit**

```bash
git add Cargo.toml Cargo.lock crates/hc-tools/Cargo.toml crates/hc-tools/src
git commit -m "feat(tools): create workspace files without overwrite"
```

---

### Task 6: Add deterministic and OpenAI-compatible write codecs

**Files:**
- Modify: `crates/hc-models/src/lib.rs`
- Test: `crates/hc-models/src/lib.rs`

**Interfaces:**
- Consumes: canonical write constructor/result.
- Produces: `DeterministicProvider::workspace_write_create` and OpenAI tool schema/parser.

- [ ] **Step 1: Write failing deterministic scenario test**

```rust
#[tokio::test]
async fn deterministic_write_scenario_calls_create_then_finishes() {
    let provider = DeterministicProvider::workspace_write_create("notes.txt", "hello");
    let first = provider
        .next_turn(ModelRequest::user("Create notes.txt"))
        .await
        .unwrap();
    let ModelOutput::ToolCalls(calls) = first else { panic!("expected tool call") };
    assert_eq!(calls, vec![ToolCall::workspace_write_create(
        "deterministic-call-1", "notes.txt", "hello"
    )]);

    let second = provider
        .next_turn(ModelRequest::with_tool_result(
            "Create notes.txt",
            ToolResult {
                call_id: calls[0].id.clone(),
                capability_id: "workspace.write".into(),
                output: json!({
                    "path": "notes.txt",
                    "mode": "create_new",
                    "bytes": 5,
                    "sha256": "digest",
                    "created": true,
                    "atomic": true
                }),
            },
        ))
        .await
        .unwrap();
    assert_eq!(
        second,
        ModelOutput::FinalText("Created notes.txt (5 bytes).".into())
    );
}
```

- [ ] **Step 2: Write failing OpenAI schema/parser tests**

Require tool names:

```rust
assert_eq!(
    names,
    vec!["workspace.list", "workspace.read", "workspace.write"]
);
```

Require exact schema:

```rust
assert_eq!(write["function"]["parameters"]["required"], json!(["path", "content", "mode"]));
assert_eq!(write["function"]["parameters"]["properties"]["mode"]["enum"], json!(["create_new"]));
```

Parser test must map path/content/mode to `ToolCall::workspace_write_create`; unsupported mode returns `InvalidResponse("workspace.write mode must be create_new")`.

- [ ] **Step 3: Run model tests and observe RED**

```bash
cargo test -p hc-models workspace_write
cargo test -p hc-models openai_request_declares
```

- [ ] **Step 4: Implement the scenario and codec**

Add `WorkspaceWriteCreate { path, content }` to `DeterministicScenario`, its constructor, first-turn call, and final text parser. Add the strict OpenAI schema and parser branch. Do not log content in errors.

- [ ] **Step 5: Run full model suite and Clippy**

```bash
cargo test -p hc-models
cargo clippy -p hc-models --all-targets --locked -- -D warnings
```

- [ ] **Step 6: Commit**

```bash
git add crates/hc-models/src/lib.rs
git commit -m "feat(models): support approved workspace writes"
```

---

### Task 7: Pause, resume, deny, and recover the same agent turn

**Files:**
- Create: `crates/hc-agent/src/approval.rs`
- Modify: `crates/hc-agent/src/lib.rs`
- Modify: `crates/hc-agent/Cargo.toml`
- Test: `crates/hc-agent/src/lib.rs`
- Test: `crates/hc-agent/src/approval.rs`

**Interfaces:**
- Consumes: `ApprovalService`, `VerifiedApproval`, approval summaries, write execution/recovery, serializable model messages.
- Produces: optional approval in `ChatOutcome`, waiting outcome, `approve`, `deny`, `list_pending`, startup `recover_executing`.

- [ ] **Step 1: Define the encrypted checkpoint and failing round-trip test**

```rust
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

Test serde round-trip and assert the pending call/content remain present in plaintext **before encryption**, because the cipher—not omission—provides durable confidentiality.

- [ ] **Step 2: Write failing waiting-approval test**

Build one `SqliteState`, one deterministic key provider, `ApprovalService`, write registry, and write deterministic provider. Assert:

```rust
let outcome = coordinator
    .run(ChatInput::new("Create notes.txt", AutonomyProfile::Assist))
    .await
    .unwrap();

assert_eq!(outcome.mission_state, MissionState::WaitingApproval);
assert_eq!(outcome.response, "Approval required");
let prompt = outcome.approval.unwrap();
assert_eq!(prompt.summary["path"], "notes.txt");
assert!(!workspace.path().join("notes.txt").exists());
assert_eq!(coordinator.list_pending_approvals(Utc::now()).unwrap().len(), 1);
assert!(outcome.evidence.iter().any(|row| row.kind == "approval_requested"));
assert!(!outcome.evidence.iter().any(|row| row.payload.to_string().contains("sentinel secret")));
```

- [ ] **Step 3: Write failing approve/deny/single-use tests**

Approval:

```rust
let completed = coordinator
    .approve(prompt.approval_id, "local_user", Utc::now())
    .await
    .unwrap();
assert_eq!(completed.mission_state, MissionState::Completed);
assert_eq!(fs::read_to_string(workspace.path().join("notes.txt")).unwrap(), "sentinel secret");
assert!(completed.response.contains("Created notes.txt"));
assert!(matches!(
    coordinator.approve(prompt.approval_id, "local_user", Utc::now()).await,
    Err(AgentError::ApprovalAlreadyDecided)
));
```

Denial must return `Cancelled`, erase pending content, and create no file.

Define this reusable test fixture immediately before the approval tests:

```rust
struct AgentApprovalFixture {
    workspace: tempfile::TempDir,
    repository: SqliteApprovalRepository,
    coordinator: TurnCoordinator,
}

impl AgentApprovalFixture {
    fn new(path: &str, content: &str) -> Self {
        let workspace = tempdir().unwrap();
        let state = SqliteState::in_memory().unwrap();
        let repository = state.approval_repository();
        let approvals = ApprovalService::new(
            Arc::new(repository.clone()),
            Arc::new(InMemoryApprovalKeyProvider::new("test-key", [7_u8; 32])),
        );
        let mut registry = CapabilityRegistry::new();
        registry.register(WorkspaceWriteCapability::new(workspace.path()).unwrap());
        let coordinator = TurnCoordinator::with_approvals(
            DeterministicProvider::workspace_write_create(path, content),
            registry,
            state.evidence_store(),
            approvals,
        );
        Self { workspace, repository, coordinator }
    }

    async fn request(&self) -> ApprovalPrompt {
        self.coordinator
            .run(ChatInput::new("Create the file", AutonomyProfile::Assist))
            .await
            .unwrap()
            .approval
            .unwrap()
    }
}

fn approval_fixture(path: &str, content: &str) -> AgentApprovalFixture {
    AgentApprovalFixture::new(path, content)
}
```

Add this changed-ciphertext/digest test using explicit test-only repository helpers:

```rust
#[tokio::test]
async fn tampered_checkpoint_fails_before_capability_execution() {
    let fixture = approval_fixture("notes.txt", "sentinel secret");
    let waiting = fixture.request().await;
    fixture
        .repository
        .replace_ciphertext_for_test(waiting.approval_id, vec![0_u8; 64])
        .unwrap();

    let error = fixture
        .coordinator
        .approve(waiting.approval_id, "local_user", Utc::now())
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        AgentError::Approval(ApprovalError::Crypto(
            ApprovalCryptoError::AuthenticationFailed
        ))
    ));
    assert!(!fixture.workspace.path().join("notes.txt").exists());
}
```

- [ ] **Step 4: Write failing restart/recovery tests**

1. Create pending with coordinator A and file-backed DB.
2. Drop A/state.
3. Reopen DB and construct coordinator B with same deterministic key.
4. Approve and assert completion.

Executing recovery fixtures:

- force row to `executing` with absent target -> `recover_executing` executes once and completes;
- force row to `executing` with matching final file -> recovery skips write, continues model, consumes approval;
- force row with conflicting final file -> failed, conflicting file unchanged;
- encrypt under wrong key -> failed closed, no file mutation.

- [ ] **Step 5: Run focused agent tests and observe RED**

```bash
cargo test -p hc-agent approval
cargo test -p hc-agent recovery
```

Expected: missing coordinator/service APIs.

- [ ] **Step 6: Refactor the coordinator into resumable `drive`**

Keep `TurnCoordinator::new` read-only compatible. Add:

```rust
pub fn with_approvals<P>(
    provider: P,
    registry: CapabilityRegistry,
    evidence: EvidenceStore,
    approvals: ApprovalService,
) -> Self;

pub fn list_pending_approvals(
    &self,
    now: DateTime<Utc>,
) -> Result<Vec<ApprovalRequest>, AgentError>;

pub async fn approve(
    &self,
    approval_id: ApprovalId,
    actor: &str,
    now: DateTime<Utc>,
) -> Result<ChatOutcome, AgentError>;

pub fn deny(
    &self,
    approval_id: ApprovalId,
    actor: &str,
    now: DateTime<Utc>,
) -> Result<ChatOutcome, AgentError>;

pub async fn recover_executing(&self) -> Vec<ApprovalRecoveryOutcome>;
```

Use this outcome type:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApprovalRecoveryOutcome {
    pub approval_id: ApprovalId,
    pub status: ApprovalStatus,
    pub failure_code: Option<String>,
}
```

Add `approval: Option<ApprovalPrompt>` to `ChatOutcome`; existing completed outcomes set `None`. Add `AgentError::Approval(#[from] ApprovalError)`, `AgentError::ApprovalUnavailable`, `AgentError::ApprovalBatchUnsupported`, and `AgentError::PendingCheckpoint(serde_json::Error)`. Tests match nested repository cases, for example:

```rust
assert!(matches!(
    error,
    AgentError::Approval(ApprovalError::Repository(
        ApprovalRepositoryError::AlreadyDecided
    ))
));
```

`drive(mission, trace_id, messages, remaining, verified_approval)` owns the provider loop. Decrement remaining before every provider call so checkpoint restoration preserves the exact future budget.

- [ ] **Step 7: Implement fail-closed batch handling**

Evaluate all calls before executing any. If any decision is `RequiresApproval`:

- require `calls.len() == 1`;
- otherwise transition to Failed and return `ApprovalBatchUnsupported`;
- compute summary through `CapabilityRegistry::approval_summary`;
- serialize/encrypt/persist checkpoint;
- append `approval_requested` evidence;
- return waiting outcome.

Do not execute allowed siblings before a pending consequential call.

- [ ] **Step 8: Implement approval and denial**

Approval loads `ApprovedCheckpoint`, deserializes checkpoint, recomputes digest, restores mission, appends `approval_decision`, reevaluates with `PolicyContext::with_approval`, executes, appends sanitized evidence, and continues `drive`.

Mark consumed only after final completion. If the capability executes but the model continuation fails, keep state `executing` so restart recovery can continue without a new approval.

Denial restores identity, transitions waiting -> cancelled, appends decision evidence, and returns without invoking provider/capability.

- [ ] **Step 9: Implement executing recovery**

For each stored executing approval:

1. decrypt/deserialize/recompute digest;
2. call `registry.recover(&pending_call)`;
3. `Applied` -> append recovered execution evidence and continue model;
4. `NotApplied` -> reevaluate using opaque token, execute once, then continue;
5. `TargetConflict`/authentication/digest error -> mark failed and append sanitized failure evidence;
6. consume only after completed continuation.

Return per-approval outcomes; one corrupt row must not prevent recovery of other rows.

- [ ] **Step 10: Run agent and workspace regression tests**

```bash
cargo test -p hc-agent
cargo test -p hc-mission
cargo test --workspace --exclude hermesclaw-desktop --locked
cargo clippy -p hc-agent --all-targets --locked -- -D warnings
```

- [ ] **Step 11: Commit**

```bash
git add crates/hc-agent/Cargo.toml crates/hc-agent/src
git commit -m "feat(agent): pause and resume approved mutations"
```

---

### Task 8: Add Axum approval endpoints and headless key configuration

**Files:**
- Modify: `crates/hc-api/src/lib.rs`
- Modify: `crates/hc-api/src/main.rs`
- Create: `crates/hc-api/tests/approval_slice.rs`
- Modify: `crates/hc-api/Cargo.toml`
- Test: `crates/hc-api/tests/approval_slice.rs`

**Interfaces:**
- Consumes: approval-enabled coordinator and public domain types.
- Produces: explicit autonomy chat, pending list/get, approve/deny endpoints, read-only startup without key, mutation-enabled startup with key.

- [ ] **Step 1: Write failing HTTP slice tests**

Test flow:

```rust
let response = post_json(
    &app,
    "/api/chat",
    json!({"message": "Create notes.txt", "autonomy": "assist"}),
).await;
assert_eq!(response.status(), StatusCode::OK);
let waiting: Value = body_json(response).await;
assert_eq!(waiting["mission_state"], "waiting_approval");
assert_eq!(waiting["approval"]["summary"]["path"], "notes.txt");
assert!(!workspace.path().join("notes.txt").exists());

let approval_id = waiting["approval"]["approval_id"].as_str().unwrap();
let completed = post_json(
    &app,
    &format!("/api/approvals/{approval_id}/decision"),
    json!({"decision": "approve"}),
).await;
assert_eq!(completed.status(), StatusCode::OK);
assert_eq!(body_json(completed).await["mission_state"], "completed");
```

Use these helpers in `crates/hc-api/tests/approval_slice.rs`:

```rust
async fn post_json(app: &Router, path: &str, value: Value) -> Response {
    app.clone()
        .oneshot(
            Request::post(path)
                .header("content-type", "application/json")
                .body(Body::from(value.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn body_json(response: Response) -> Value {
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}
```

Additional tests:

- missing/invalid autonomy -> `400 invalid_autonomy`;
- Observe write -> `403 policy_denied`, no approval row;
- `GET /api/approvals/pending` restores after reopening DB;
- get unknown -> `404 approval_not_found`;
- duplicate decision -> `409 approval_already_decided`;
- expired -> `409 approval_expired`;
- deny -> cancelled/no file;
- no approval key/service -> `503 approval_key_unavailable` for mutation flow while `/health` and Observe list/read remain available;
- response/evidence never contains sentinel content outside the final model response after approved execution.

- [ ] **Step 2: Run focused tests and observe RED**

```bash
cargo test -p hc-api --test approval_slice
```

- [ ] **Step 3: Make autonomy explicit**

```rust
#[derive(Debug, Deserialize)]
struct ChatRequest {
    message: String,
    autonomy: AutonomyProfile,
}
```

Pass the supplied profile to `ChatInput`. Existing web/Tauri clients are updated in later tasks; update current API list/read tests now to send `"autonomy": "observe"`.

`ChatResponse` includes:

```rust
approval: Option<ApprovalPrompt>,
```

- [ ] **Step 4: Add approval routes**

```rust
.route("/api/approvals/pending", get(list_pending_approvals))
.route("/api/approvals/{approval_id}", get(get_approval))
.route(
    "/api/approvals/{approval_id}/decision",
    post(decide_approval),
)
```

Decision request contains only `decision`; actor is the trusted local adapter constant `local_user`. Parse IDs strictly. Use this exact public mapping:

```text
invalid JSON/autonomy/approval ID              -> 400 invalid_request|invalid_autonomy|invalid_approval_id
PolicyDenied                                   -> 403 policy_denied
ApprovalRepositoryError::NotFound              -> 404 approval_not_found
ApprovalRepositoryError::Expired               -> 409 approval_expired
ApprovalRepositoryError::AlreadyDecided        -> 409 approval_already_decided
ApprovalError::DigestMismatch                  -> 409 action_digest_mismatch
ApprovalCryptoError::AuthenticationFailed      -> 409 approval_checkpoint_invalid
ApprovalKeyError::*                            -> 503 approval_key_unavailable
ApprovalUnavailable                            -> 503 approval_key_unavailable
all other repository/model/state/capability    -> 500 agent_failed
```

Public error bodies contain only `{"error": "<stable_code>"}`.

- [ ] **Step 5: Configure one SQLite state and optional headless approvals**

`main.rs`:

1. open one `SqliteState`;
2. always register list/read;
3. attempt `EnvApprovalKeyProvider::from_env()`;
4. if valid, register write and construct `ApprovalService` from `state.approval_repository()` plus key provider;
5. if absent, build read-only coordinator and keep approval routes returning 503;
6. if malformed, fail startup instead of silently disabling configured mutation;
7. run `recover_executing().await` before accepting requests when approvals are enabled.

- [ ] **Step 6: Run API and strict Clippy**

```bash
cargo test -p hc-api
cargo clippy -p hc-api --all-targets --locked -- -D warnings
```

- [ ] **Step 7: Commit**

```bash
git add crates/hc-api
git commit -m "feat(api): expose durable approval decisions"
```

---

### Task 9: Add desktop credential storage and Tauri approval commands

**Files:**
- Create: `apps/desktop/src-tauri/src/approval_key.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify: `apps/desktop/src-tauri/Cargo.toml`
- Modify: root `Cargo.toml`
- Modify: `Cargo.lock`
- Test: compile plus unit tests for base64/key conversion helpers.

**Interfaces:**
- Consumes: `ApprovalKeyProvider`, shared SQLite state, approval-enabled coordinator.
- Produces: OS-credential-backed key and Tauri chat/list/decision commands.

- [ ] **Step 1: Add the keyring dependency**

Root:

```toml
keyring = "3"
```

Desktop manifest:

```toml
base64.workspace = true
chacha20poly1305.workspace = true
keyring.workspace = true
hc-policy = { path = "../../../crates/hc-policy" }
```

Use Cargo feature resolution on all CI platforms; do not disable secure storage to make compilation easier.

- [ ] **Step 2: Write failing key-provider helper tests**

Separate pure encode/decode helpers from the real keyring call:

```rust
#[test]
fn stored_key_round_trips_as_base64_32_bytes() {
    let encoded = encode_key([11_u8; 32]);
    assert_eq!(decode_key(&encoded).unwrap(), [11_u8; 32]);
}

#[test]
fn stored_key_rejects_wrong_length() {
    let encoded = STANDARD.encode([1_u8; 31]);
    assert!(decode_key(&encoded).is_err());
}
```

- [ ] **Step 3: Implement `DesktopApprovalKeyProvider`**

Use service `HermesClaw` and account `approval-master-key-v1`.

- `get_password` success -> decode exactly 32 bytes;
- `NoEntry` -> generate 32 random bytes with `XChaCha20Poly1305::generate_key(&mut OsRng)`, store base64, return key ID `desktop-keyring-v1`;
- every other keyring error -> `ApprovalKeyError::Backend`;
- `key_by_id` accepts only `desktop-keyring-v1`.

The key string is never logged.

- [ ] **Step 4: Write failing Tauri command compile tests through `cargo check`**

Update command signatures:

```rust
#[tauri::command]
async fn chat(
    state: State<'_, DesktopState>,
    message: String,
    autonomy: AutonomyProfile,
) -> Result<ChatOutcome, String>;

#[tauri::command]
fn list_pending_approvals(
    state: State<'_, DesktopState>,
) -> Result<Vec<ApprovalRequest>, String>;

#[tauri::command]
async fn decide_approval(
    state: State<'_, DesktopState>,
    approval_id: String,
    decision: ApprovalDecision,
) -> Result<ChatOutcome, String>;
```

- [ ] **Step 5: Build one shared state and recover on startup**

In setup:

1. open `SqliteState` in app data;
2. build evidence and approval repository facades from it;
3. initialize desktop key provider;
4. register list/read/write only if key provider succeeds;
5. construct approval-enabled coordinator;
6. call `tauri::async_runtime::block_on(coordinator.recover_executing())`;
7. manage state and register commands.

If keyring initialization fails, start in read-only mode and return `approval_key_unavailable` from approval commands. Do not create a plaintext key file fallback.

- [ ] **Step 6: Compile desktop on supported CI environment**

```bash
cargo check -p hermesclaw-desktop --locked
```

Expected: success with Tauri system packages present.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock apps/desktop/src-tauri
git commit -m "feat(desktop): secure durable approval keys"
```

---

### Task 10: Add the Assist and durable approval cockpit

**Files:**
- Modify: `apps/web/src/api.ts`
- Modify: `apps/web/src/App.tsx`
- Modify: `apps/web/src/App.test.tsx`
- Modify: `apps/web/src/app.css`
- Test: `apps/web/src/App.test.tsx`

**Interfaces:**
- Consumes: explicit autonomy chat, pending approval APIs, decision result.
- Produces: Observe/Assist selector, reload restoration, secure approval card, approve/deny UX.

- [ ] **Step 1: Write failing API typing tests through TypeScript/Vitest**

Add types:

```typescript
export type AutonomyProfile = 'observe' | 'assist';
export type ApprovalDecision = 'approve' | 'deny';
export interface ApprovalPrompt {
  approval_id: string;
  capability_id: string;
  reason: string;
  summary: {
    path: string;
    mode: 'create_new';
    bytes: number;
    sha256: string;
  };
  expires_at: string;
}
```

Update `ChatResponse` with optional approval. Mock tests must fail until `sendChat(message, autonomy)`, `listPendingApprovals`, `getApproval`, and `decideApproval` exist.

- [ ] **Step 2: Write failing UI behavior tests**

Required tests:

1. Observe is default and sent explicitly.
2. Selecting Assist sends `assist`.
3. Waiting response renders path/mode/bytes/hash/expiry/reason and **not content**.
4. Pending approvals returned on initial load render after remount.
5. Approve disables both buttons while in flight, returns completed mission, then loads evidence.
6. Deny renders Cancelled and no success message.
7. Duplicate/expired errors render stable messages.
8. No element in the approval card contains sentinel plaintext.

Representative security assertion:

```typescript
const approvalCard = await screen.findByRole('region', { name: 'Approval required' });
expect(within(approvalCard).getByText('notes/new-file.txt')).toBeInTheDocument();
expect(within(approvalCard).queryByText(/sentinel secret/)).not.toBeInTheDocument();
```

Use this exact public-only fixture in the UI tests:

```typescript
const pendingApproval: ApprovalPrompt = {
  approval_id: 'approval-1',
  capability_id: 'workspace.write',
  reason: 'assist profile requires approval for consequential actions',
  summary: {
    path: 'notes/new-file.txt',
    mode: 'create_new',
    bytes: 15,
    sha256: 'a'.repeat(64),
  },
  expires_at: '2026-08-15T12:00:00Z',
};
```

Reload test:

```typescript
listPendingApprovals.mockResolvedValue([pendingApproval]);
const { unmount } = render(<App />);
expect(await screen.findByRole('region', { name: 'Approval required' })).toBeInTheDocument();
unmount();
render(<App />);
expect(await screen.findByText('notes/new-file.txt')).toBeInTheDocument();
```

- [ ] **Step 3: Run tests and observe RED**

```bash
npm --prefix apps/web ci
npm run test:web
```

- [ ] **Step 4: Implement API functions**

All functions check `response.ok`, parse `{error}` safely, and throw an `ApiError` carrying status/code.

```typescript
export async function sendChat(
  message: string,
  autonomy: AutonomyProfile,
): Promise<ChatResponse>;

export async function listPendingApprovals(): Promise<ApprovalPrompt[]>;

export async function decideApproval(
  approvalId: string,
  decision: ApprovalDecision,
): Promise<ChatResponse>;
```

- [ ] **Step 5: Implement the profile selector and approval state**

- default state: Observe;
- explicit segmented control or select with accessible label `Autonomy profile`;
- pending approvals fetched once on mount;
- approval cards derive solely from public summary;
- Approve/Deny call the decision endpoint; no original action/content is stored in React;
- pending state removed only after successful terminal response or stable terminal error;
- completed state shown only after the backend returns completed.

Use `<time dateTime={expires_at}>` and show the full digest in a `<code>` element with abbreviated visual styling.

- [ ] **Step 6: Add focused CSS without redesigning unrelated surfaces**

Add styles for `.profile-selector`, `.approval-card`, `.approval-grid`, `.approval-actions`, `.digest`, and disabled decision buttons. Preserve responsive behavior and existing visual hierarchy.

- [ ] **Step 7: Run Web gates**

```bash
npm run test:web
npm run lint:web
npm run build:web
```

Expected: all pass.

- [ ] **Step 8: Commit**

```bash
git add apps/web/src
git commit -m "feat(web): review and decide durable approvals"
```

---

### Task 11: Run recovery/startup integration and full end-to-end proof

**Files:**
- Modify: `crates/hc-api/tests/approval_slice.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Test: all workspace tests.

**Interfaces:**
- Consumes: complete backend/UI implementation.
- Produces: restart and recovery evidence across public transports.

- [ ] **Step 1: Add file-backed restart E2E test**

The test must:

1. create a file-backed DB and coordinator A;
2. request write and capture approval ID;
3. drop A and reopen coordinator B with the same key;
4. list pending and find the same ID;
5. approve through Axum;
6. assert completed response, exact file bytes, consumed status, erased ciphertext, and sanitized evidence.

- [ ] **Step 2: Add crash-window recovery E2E tests**

Use this exact helper and three tests. `force_status_for_test` exists only under `#[cfg(test)]` and sets status, actor `recovery-test`, and `decided_at` without altering ciphertext.

```rust
fn open_recovery_coordinator(
    database: &Path,
    workspace: &Path,
) -> (TurnCoordinator, SqliteApprovalRepository) {
    let state = SqliteState::open(database).unwrap();
    let repository = state.approval_repository();
    let approvals = ApprovalService::new(
        Arc::new(repository.clone()),
        Arc::new(InMemoryApprovalKeyProvider::new("test-key", [7_u8; 32])),
    );
    let mut registry = CapabilityRegistry::new();
    registry.register(WorkspaceWriteCapability::new(workspace).unwrap());
    let coordinator = TurnCoordinator::with_approvals(
        DeterministicProvider::workspace_write_create("notes.txt", "approved"),
        registry,
        state.evidence_store(),
        approvals,
    );
    (coordinator, repository)
}

#[tokio::test]
async fn restart_recovery_executes_absent_target_once() {
    let directory = tempdir().unwrap();
    let database = directory.path().join("state.db");
    let workspace = directory.path().join("workspace");
    fs::create_dir(&workspace).unwrap();

    let (coordinator, repository) = open_recovery_coordinator(&database, &workspace);
    let waiting = coordinator
        .run(ChatInput::new("Create notes.txt", AutonomyProfile::Assist))
        .await
        .unwrap();
    let approval_id = waiting.approval.as_ref().unwrap().approval_id;
    repository
        .force_status_for_test(approval_id, ApprovalStatus::Executing)
        .unwrap();
    drop(coordinator);

    let (restarted, restarted_repository) =
        open_recovery_coordinator(&database, &workspace);
    let outcomes = restarted.recover_executing().await;

    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].status, ApprovalStatus::Consumed);
    assert_eq!(fs::read_to_string(workspace.join("notes.txt")).unwrap(), "approved");
    let rows = restarted
        .evidence_store()
        .list_by_trace(waiting.trace_id)
        .unwrap();
    assert_eq!(
        rows.iter()
            .filter(|row| row.kind == "capability_execution")
            .count(),
        1
    );
    assert_eq!(
        restarted_repository
            .load_public(approval_id, Utc::now())
            .unwrap()
            .status,
        ApprovalStatus::Consumed
    );
}

#[tokio::test]
async fn restart_recovery_recognizes_matching_published_target() {
    let directory = tempdir().unwrap();
    let database = directory.path().join("state.db");
    let workspace = directory.path().join("workspace");
    fs::create_dir(&workspace).unwrap();

    let (coordinator, repository) = open_recovery_coordinator(&database, &workspace);
    let waiting = coordinator
        .run(ChatInput::new("Create notes.txt", AutonomyProfile::Assist))
        .await
        .unwrap();
    let approval_id = waiting.approval.as_ref().unwrap().approval_id;
    repository
        .force_status_for_test(approval_id, ApprovalStatus::Executing)
        .unwrap();
    fs::write(workspace.join("notes.txt"), "approved").unwrap();
    drop(coordinator);

    let (restarted, _) = open_recovery_coordinator(&database, &workspace);
    let outcomes = restarted.recover_executing().await;

    assert_eq!(outcomes[0].status, ApprovalStatus::Consumed);
    assert_eq!(fs::read_to_string(workspace.join("notes.txt")).unwrap(), "approved");
    let rows = restarted
        .evidence_store()
        .list_by_trace(waiting.trace_id)
        .unwrap();
    assert_eq!(
        rows.iter().filter(|row| row.kind == "capability_recovered").count(),
        1
    );
}

#[tokio::test]
async fn restart_recovery_fails_closed_on_conflicting_target() {
    let directory = tempdir().unwrap();
    let database = directory.path().join("state.db");
    let workspace = directory.path().join("workspace");
    fs::create_dir(&workspace).unwrap();

    let (coordinator, repository) = open_recovery_coordinator(&database, &workspace);
    let waiting = coordinator
        .run(ChatInput::new("Create notes.txt", AutonomyProfile::Assist))
        .await
        .unwrap();
    let approval_id = waiting.approval.as_ref().unwrap().approval_id;
    repository
        .force_status_for_test(approval_id, ApprovalStatus::Executing)
        .unwrap();
    fs::write(workspace.join("notes.txt"), "conflict").unwrap();
    drop(coordinator);

    let (restarted, restarted_repository) =
        open_recovery_coordinator(&database, &workspace);
    let outcomes = restarted.recover_executing().await;

    assert_eq!(outcomes[0].status, ApprovalStatus::Failed);
    assert_eq!(outcomes[0].failure_code.as_deref(), Some("target_conflict"));
    assert_eq!(fs::read_to_string(workspace.join("notes.txt")).unwrap(), "conflict");
    let request = restarted_repository
        .load_public(approval_id, Utc::now())
        .unwrap();
    assert_eq!(request.status, ApprovalStatus::Failed);
    assert_eq!(request.failure_code.as_deref(), Some("target_conflict"));
}
```

Assert evidence ordering by filtering kinds:

```rust
assert_eq!(
    evidence.iter().map(|row| row.kind.as_str()).collect::<Vec<_>>(),
    vec![
        "policy_decision",
        "approval_requested",
        "approval_decision",
        "policy_decision",
        "capability_execution",
    ]
);
```

If recovery adds a distinct `capability_recovered` evidence kind, specify and assert that exact sequence instead; do not weaken to unordered containment.

- [ ] **Step 3: Run full Rust and desktop gates**

```bash
cargo fmt --all --check
cargo test --workspace --exclude hermesclaw-desktop --locked
cargo clippy --workspace --exclude hermesclaw-desktop --all-targets --locked -- -D warnings
cargo check -p hermesclaw-desktop --locked
```

- [ ] **Step 4: Run Web gates again**

```bash
npm --prefix apps/web ci
npm run test:web
npm run lint:web
npm run build:web
```

- [ ] **Step 5: Commit any integration-only adjustments**

```bash
git add crates/hc-api/tests/approval_slice.rs apps/desktop/src-tauri/src/lib.rs
git commit -m "test: prove durable write approval recovery"
```

Skip the commit only when the tree is unchanged.

---

### Task 12: Independent review, canonical write-back, PR, and merge verification

**Files:**
- Create: `docs/evidence/2026-08-14-workspace-write-approval-verification.md`
- Modify: `README.md`
- Modify: `source/01_CANONICAL_FACTS.md`
- Modify: `source/03_CURRENT_STATE.md`
- Modify: `source/09_ROADMAP.md`
- Modify: `source/11_HANDOFF.md`
- Modify: `source/SOURCE_MANIFEST.json`

**Interfaces:**
- Consumes: exact command output, review findings, branch/PR/main CI identifiers.
- Produces: evidence-backed canonical state and release checkpoint.

- [ ] **Step 1: Run the complete local verification matrix from a clean tree**

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
git status --short
```

Record exact test counts and zero failures from actual output.

- [ ] **Step 2: Run independent engineering verification**

Use Codex Engineering Guardrails against the full diff and explicit approved spec. Review:

- authorization bypass;
- model-minted approval risk;
- digest mismatch or noncanonical JSON;
- plaintext leakage to SQLite/logs/evidence/UI;
- race/no-clobber behavior;
- expiration/single-use concurrency;
- recovery idempotence;
- database sharing/no duplicate state;
- API actor/autonomy trust boundary;
- regressions to list/read.

Use Fallow for changed TypeScript/JavaScript when the CLI/plugin is connected and operational. Use SonarQube only when a real project/scanner is configured. Record unavailable tooling honestly.

- [ ] **Step 3: Create the evidence document from actual results**

The document must include:

- branch and commit tested;
- exact commands and counts;
- approval/digest/encryption/repository/concurrency/security/recovery test matrix;
- proof that the sentinel plaintext is absent from SQLite/evidence/approval UI;
- key-provider modes and failure behavior;
- known limitations from the spec;
- independent review verdict and tool availability;
- no unsupported claim about overwrite, remote identity, universal filesystem atomicity, or hostile same-user protection.

- [ ] **Step 4: Update README and canonical memory only after local green**

- README: describe v0.3 write approval without implying autonomous mutation.
- Canonical facts: add only verified behavior and exact merged identifiers later.
- Current state: branch status until merge; after merge update to canonical.
- Roadmap: mark write slice “verified pending merge,” then “verified complete” only post-merge.
- Handoff: exact next action, blockers, evidence, residual limitations.

- [ ] **Step 5: Recalculate source-manifest hashes**

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

- [ ] **Step 6: Commit the verified documentation**

```bash
git add README.md docs/evidence/2026-08-14-workspace-write-approval-verification.md \
  source/01_CANONICAL_FACTS.md source/03_CURRENT_STATE.md source/09_ROADMAP.md \
  source/11_HANDOFF.md source/SOURCE_MANIFEST.json
git commit -m "docs: record durable workspace write verification"
```

- [ ] **Step 7: Push and open a draft PR**

Push the implementation branch and open a draft PR against `main`. The PR body must state:

- create-only semantics;
- exact approval binding;
- encrypted restart persistence;
- single-use/expiration behavior;
- atomic no-clobber execution and recovery;
- plaintext non-retention evidence;
- explicit non-goals/limitations;
- local verification counts.

- [ ] **Step 8: Resolve review and verify branch/PR CI**

Do not mark ready until required Rust, Web, and Tauri jobs are green and every actionable review thread is resolved with code/tests/evidence. Update the evidence/handoff with actual run and job IDs, then rerun source validation.

- [ ] **Step 9: Merge and verify `main`**

Squash merge only while the PR head SHA matches the reviewed commit. Wait for post-merge `main` CI. If any job fails, reopen a focused fix branch; do not claim completion.

- [ ] **Step 10: Final canonicalization PR**

After post-merge success, use a documentation-only branch to replace pending-merge language with:

- final merge commit;
- PR number;
- post-merge run/job IDs;
- canonical capability status;
- next bounded checkpoint.

Merge that PR only after its own CI is green.

---

## Plan Self-Review

- **Spec coverage:** create-only contract, shared boundary, approval policy, exact JCS digest, opaque grant, encrypted durable continuation, OS/env/test key providers, one SQLite database, concurrency, expiration, pause/resume/deny, crash recovery, no-clobber execution, evidence, API, Tauri, React, security tests, verification, and retirement criterion are each assigned to a task.
- **Resolved ambiguity:** Autonomous Scoped never auto-writes, but an exact human-approved token may authorize the same action; this is the only interpretation that satisfies both “requires approval” and restart/resume semantics. Approval `reason` is persisted because the durable API must reconstruct it after restart.
- **Type consistency:** later tasks consistently use `ApprovalId`, `ApprovalRequest`, `ApprovalPrompt`, `ActionDigest`, `VerifiedApproval`, `ApprovalService`, `SqliteState`, `WorkspaceWriteCapability`, `PendingTurnCheckpoint`, and `ChatOutcome.approval` as defined in earlier tasks.
- **Scope check:** no overwrite, patch, append, delete, rename, directory creation, process execution, automatic scoped mutation, remote multi-user identity, key rotation UI, or general durable Mission repository is included.
- **Placeholder scan:** no `TBD`, `TODO`, omitted test body, or undefined “similar to” step remains. Dynamic SHAs/run IDs are copied only from actual Git/GitHub output and are never guessed.
