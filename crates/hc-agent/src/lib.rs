use chrono::Utc;
use hc_domain::{
    AutonomyProfile, EvidenceRecord, MissionId, MissionState, PolicyDecision, TraceId,
};
use hc_mission::{Mission, MissionError};
use hc_models::{ModelError, ModelMessage, ModelOutput, ModelProvider, ModelRequest};
use hc_policy::PolicyKernel;
use hc_state::{EvidenceStore, StateError};
use hc_tools::{CapabilityError, CapabilityRegistry};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use thiserror::Error;

const DEFAULT_MAX_MODEL_ITERATIONS: usize = 4;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatInput {
    pub message: String,
    pub autonomy: AutonomyProfile,
}

impl ChatInput {
    pub fn new(message: impl Into<String>, autonomy: AutonomyProfile) -> Self {
        Self {
            message: message.into(),
            autonomy,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatOutcome {
    pub trace_id: TraceId,
    pub mission_id: MissionId,
    pub mission_state: MissionState,
    pub response: String,
    pub evidence: Vec<EvidenceRecord>,
}

#[derive(Clone)]
pub struct TurnCoordinator {
    provider: Arc<dyn ModelProvider>,
    registry: Arc<CapabilityRegistry>,
    evidence: Arc<EvidenceStore>,
    max_model_iterations: usize,
}

impl TurnCoordinator {
    pub fn new<P>(provider: P, registry: CapabilityRegistry, evidence: EvidenceStore) -> Self
    where
        P: ModelProvider + 'static,
    {
        Self {
            provider: Arc::new(provider),
            registry: Arc::new(registry),
            evidence: Arc::new(evidence),
            max_model_iterations: DEFAULT_MAX_MODEL_ITERATIONS,
        }
    }

    pub fn evidence_store(&self) -> Arc<EvidenceStore> {
        Arc::clone(&self.evidence)
    }

    pub async fn run(&self, input: ChatInput) -> Result<ChatOutcome, AgentError> {
        let trace_id = TraceId::new();
        let mut mission = Mission::new(&input.message);
        mission.transition(MissionState::Executing)?;

        let mut messages = vec![ModelMessage::User(input.message)];

        for _ in 0..self.max_model_iterations {
            match self
                .provider
                .next_turn(ModelRequest::new(messages.clone()))
                .await?
            {
                ModelOutput::ToolCalls(calls) => {
                    messages.push(ModelMessage::AssistantToolCalls(calls.clone()));
                    for call in calls {
                        let decision = PolicyKernel::evaluate(input.autonomy, &call);
                        self.record_policy(trace_id, mission.id(), &call.capability_id, &decision)?;

                        match &decision {
                            PolicyDecision::Allow => {}
                            PolicyDecision::RequiresApproval(reason) => {
                                mission.transition(MissionState::WaitingApproval)?;
                                return Err(AgentError::ApprovalRequired(reason.clone()));
                            }
                            PolicyDecision::Deny(reason) => {
                                mission.transition(MissionState::Failed)?;
                                return Err(AgentError::PolicyDenied(reason.clone()));
                            }
                        }

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
                    }
                }
                ModelOutput::FinalText(response) => {
                    mission.transition(MissionState::Verifying)?;
                    mission.transition(MissionState::Completed)?;
                    let evidence = self.evidence.list_by_trace(trace_id)?;
                    return Ok(ChatOutcome {
                        trace_id,
                        mission_id: mission.id(),
                        mission_state: mission.state(),
                        response,
                        evidence,
                    });
                }
            }
        }

        mission.transition(MissionState::Failed)?;
        Err(AgentError::IterationBudgetExceeded)
    }

    fn record_policy(
        &self,
        trace_id: TraceId,
        mission_id: MissionId,
        capability_id: &str,
        decision: &PolicyDecision,
    ) -> Result<(), StateError> {
        let status = match decision {
            PolicyDecision::Allow => "allowed",
            PolicyDecision::RequiresApproval(_) => "requires_approval",
            PolicyDecision::Deny(_) => "denied",
        };
        self.evidence.append(&EvidenceRecord {
            trace_id,
            mission_id,
            kind: "policy_decision".into(),
            capability_id: Some(capability_id.to_owned()),
            policy_decision: Some(decision.clone()),
            status: status.into(),
            payload: json!({}),
            recorded_at: Utc::now(),
        })
    }
}

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("model error: {0}")]
    Model(#[from] ModelError),
    #[error("mission error: {0}")]
    Mission(#[from] MissionError),
    #[error("state error: {0}")]
    State(#[from] StateError),
    #[error("capability error: {0}")]
    Capability(#[from] CapabilityError),
    #[error("policy denied action: {0}")]
    PolicyDenied(String),
    #[error("action requires approval: {0}")]
    ApprovalRequired(String),
    #[error("agent model iteration budget exceeded")]
    IterationBudgetExceeded,
}

#[cfg(test)]
mod tests {
    use super::*;
    use hc_domain::{AutonomyProfile, MissionState, PolicyDecision};
    use hc_models::DeterministicProvider;
    use hc_state::EvidenceStore;
    use hc_tools::{CapabilityRegistry, WorkspaceListCapability, WorkspaceReadCapability};
    use std::fs;
    use tempfile::tempdir;

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
        assert_eq!(
            outcome.response,
            "Contents of alpha.txt:\nalpha secret text"
        );
        assert_eq!(outcome.evidence.len(), 2);
        let execution = &outcome.evidence[1];
        assert_eq!(execution.capability_id.as_deref(), Some("workspace.read"));
        assert_eq!(execution.payload["path"], "alpha.txt");
        assert_eq!(execution.payload["bytes"], 17);
        assert_eq!(execution.payload["sha256"].as_str().unwrap().len(), 64);
        assert!(execution.payload.get("content").is_none());
        assert!(!execution.payload.to_string().contains("alpha secret text"));
    }

    #[tokio::test]
    async fn deterministic_turn_completes_with_policy_tool_and_evidence() {
        let workspace = tempdir().expect("workspace");
        fs::write(workspace.path().join("alpha.txt"), "alpha").unwrap();

        let mut registry = CapabilityRegistry::new();
        registry.register(WorkspaceListCapability::new(workspace.path()).unwrap());
        let coordinator = TurnCoordinator::new(
            DeterministicProvider::default(),
            registry,
            EvidenceStore::in_memory().unwrap(),
        );

        let outcome = coordinator
            .run(ChatInput::new(
                "List the workspace",
                AutonomyProfile::Observe,
            ))
            .await
            .expect("complete deterministic turn");

        assert_eq!(outcome.mission_state, MissionState::Completed);
        assert!(outcome.response.contains("alpha.txt"));
        assert_eq!(outcome.evidence.len(), 2);
        assert_eq!(outcome.evidence[0].kind, "policy_decision");
        assert_eq!(
            outcome.evidence[0].policy_decision,
            Some(PolicyDecision::Allow)
        );
        assert_eq!(outcome.evidence[1].kind, "capability_execution");
        assert_eq!(outcome.evidence[1].status, "succeeded");
    }
}
