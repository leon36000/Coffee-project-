mod approval;
pub use approval::*;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fmt, str::FromStr};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TraceId(Uuid);

impl TraceId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for TraceId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for TraceId {
    type Err = uuid::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(value)?))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MissionId(Uuid);

impl MissionId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for MissionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for MissionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustLevel {
    TrustedUser,
    ExternalUntrusted,
    ToolOutput,
    ModelGenerated,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub source: String,
    pub trust: TrustLevel,
}

impl Provenance {
    pub fn new(source: impl Into<String>, trust: TrustLevel) -> Self {
        Self {
            source: source.into(),
            trust,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissionState {
    Created,
    Planning,
    Executing,
    WaitingApproval,
    WaitingExternal,
    Verifying,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskClass {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SideEffectClass {
    None,
    Mutation,
    External,
    Destructive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyProfile {
    Observe,
    Assist,
    AutonomousScoped,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", content = "reason", rename_all = "snake_case")]
pub enum PolicyDecision {
    Allow,
    RequiresApproval(String),
    Deny(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub capability_id: String,
    pub arguments: Value,
    pub risk: RiskClass,
    pub side_effect: SideEffectClass,
    pub provenance: Provenance,
}

impl ToolCall {
    pub fn new(
        id: impl Into<String>,
        capability_id: impl Into<String>,
        arguments: Value,
        risk: RiskClass,
        side_effect: SideEffectClass,
        provenance: Provenance,
    ) -> Self {
        Self {
            id: id.into(),
            capability_id: capability_id.into(),
            arguments,
            risk,
            side_effect,
            provenance,
        }
    }

    pub fn workspace_list(id: impl Into<String>, path: impl Into<String>) -> Self {
        Self::new(
            id,
            "workspace.list",
            serde_json::json!({ "path": path.into() }),
            RiskClass::Low,
            SideEffectClass::None,
            Provenance::new("model", TrustLevel::ModelGenerated),
        )
    }

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
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolResult {
    pub call_id: String,
    pub capability_id: String,
    pub output: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub trace_id: TraceId,
    pub mission_id: MissionId,
    pub kind: String,
    pub capability_id: Option<String>,
    pub policy_decision: Option<PolicyDecision>,
    pub status: String,
    pub payload: Value,
    pub recorded_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_call_round_trips_through_json() {
        let call = ToolCall::workspace_list("call-1", ".");
        let encoded = serde_json::to_string(&call).expect("serialize tool call");
        let decoded: ToolCall = serde_json::from_str(&encoded).expect("deserialize tool call");
        assert_eq!(decoded, call);
        assert_eq!(decoded.capability_id, "workspace.list");
    }

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

    #[test]
    fn workspace_write_constructor_is_medium_risk_create_only_mutation() {
        let call = ToolCall::workspace_write_create("call-write", "notes/new-file.txt", "hello");

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

    #[test]
    fn mission_state_serializes_to_stable_snake_case() {
        let encoded =
            serde_json::to_string(&MissionState::WaitingApproval).expect("serialize state");
        assert_eq!(encoded, "\"waiting_approval\"");
    }
}
