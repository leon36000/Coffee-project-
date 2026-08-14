use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use hc_agent::{AgentError, ChatInput, ChatOutcome, TurnCoordinator};
use hc_domain::{AutonomyProfile, EvidenceRecord, MissionId, MissionState, TraceId};
use hc_state::EvidenceStore;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{str::FromStr, sync::Arc};

#[derive(Clone)]
struct AppState {
    coordinator: TurnCoordinator,
    evidence: Arc<EvidenceStore>,
}

pub fn build_router(coordinator: TurnCoordinator) -> Router {
    let state = AppState {
        evidence: coordinator.evidence_store(),
        coordinator,
    };

    Router::new()
        .route("/health", get(health))
        .route("/api/chat", post(chat))
        .route("/api/evidence/{trace_id}", get(evidence))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({"status": "ok"}))
}

#[derive(Debug, Deserialize)]
struct ChatRequest {
    message: String,
}

#[derive(Debug, Serialize)]
struct ChatResponse {
    trace_id: TraceId,
    mission_id: MissionId,
    mission_state: MissionState,
    response: String,
}

impl From<ChatOutcome> for ChatResponse {
    fn from(outcome: ChatOutcome) -> Self {
        Self {
            trace_id: outcome.trace_id,
            mission_id: outcome.mission_id,
            mission_state: outcome.mission_state,
            response: outcome.response,
        }
    }
}

async fn chat(
    State(state): State<AppState>,
    Json(request): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, ApiError> {
    if request.message.trim().is_empty() {
        return Err(ApiError::bad_request("invalid_request"));
    }

    let outcome = state
        .coordinator
        .run(ChatInput::new(request.message, AutonomyProfile::Observe))
        .await
        .map_err(ApiError::from_agent)?;
    Ok(Json(outcome.into()))
}

async fn evidence(
    State(state): State<AppState>,
    Path(trace_id): Path<String>,
) -> Result<Json<Vec<EvidenceRecord>>, ApiError> {
    let trace_id =
        TraceId::from_str(&trace_id).map_err(|_| ApiError::bad_request("invalid_trace_id"))?;
    let rows = state
        .evidence
        .list_by_trace(trace_id)
        .map_err(|_| ApiError::internal("evidence_unavailable"))?;
    Ok(Json(rows))
}

struct ApiError {
    status: StatusCode,
    code: &'static str,
}

impl ApiError {
    fn bad_request(code: &'static str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code,
        }
    }

    fn internal(code: &'static str) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code,
        }
    }

    fn from_agent(error: AgentError) -> Self {
        match error {
            AgentError::PolicyDenied(_) => Self {
                status: StatusCode::FORBIDDEN,
                code: "policy_denied",
            },
            AgentError::ApprovalRequired(_) => Self {
                status: StatusCode::CONFLICT,
                code: "approval_required",
            },
            _ => Self::internal("agent_failed"),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({"error": self.code}))).into_response()
    }
}
