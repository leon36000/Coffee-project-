use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use hc_agent::TurnCoordinator;
use hc_api::build_router;
use hc_models::DeterministicProvider;
use hc_state::EvidenceStore;
use hc_tools::{CapabilityRegistry, WorkspaceListCapability, WorkspaceReadCapability};
use serde_json::{json, Value};
use std::fs;
use tempfile::tempdir;
use tower::ServiceExt;

#[tokio::test]
async fn chat_endpoint_runs_vertical_slice_and_exposes_evidence() {
    let workspace = tempdir().expect("workspace");
    fs::write(workspace.path().join("alpha.txt"), "alpha").unwrap();

    let mut registry = CapabilityRegistry::new();
    registry.register(WorkspaceListCapability::new(workspace.path()).unwrap());
    let coordinator = TurnCoordinator::new(
        DeterministicProvider::default(),
        registry,
        EvidenceStore::in_memory().unwrap(),
    );
    let app = build_router(coordinator);

    let response = app
        .clone()
        .oneshot(
            Request::post("/api/chat")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"message": "List the workspace"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("chat response");
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let chat: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(chat["mission_state"], "completed");
    assert!(chat["response"].as_str().unwrap().contains("alpha.txt"));
    let trace_id = chat["trace_id"].as_str().unwrap();

    let response = app
        .oneshot(
            Request::get(format!("/api/evidence/{trace_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("evidence response");
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let evidence: Vec<Value> = serde_json::from_slice(&body).unwrap();
    assert!(evidence.iter().any(|row| {
        row["kind"] == "capability_execution"
            && row["capability_id"] == "workspace.list"
            && row["status"] == "succeeded"
    }));
}

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
    assert_eq!(
        chat["response"],
        "Contents of alpha.txt:\nalpha secret text"
    );
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
        .find(|row| {
            row["kind"] == "capability_execution" && row["capability_id"] == "workspace.read"
        })
        .unwrap();
    assert_eq!(execution["payload"]["path"], "alpha.txt");
    assert_eq!(execution["payload"]["bytes"], 17);
    assert_eq!(execution["payload"]["sha256"].as_str().unwrap().len(), 64);
    assert!(execution["payload"].get("content").is_none());
    assert!(!execution.to_string().contains("alpha secret text"));
}
