use hc_agent::{ChatInput, ChatOutcome, TurnCoordinator};
use hc_domain::{AutonomyProfile, EvidenceRecord, TraceId};
use hc_models::DeterministicProvider;
use hc_state::EvidenceStore;
use hc_tools::{CapabilityRegistry, WorkspaceListCapability};
use std::{env, path::PathBuf, str::FromStr, sync::Arc};
use tauri::{Manager, State};

struct DesktopState {
    coordinator: TurnCoordinator,
    evidence: Arc<EvidenceStore>,
}

#[tauri::command]
async fn chat(state: State<'_, DesktopState>, message: String) -> Result<ChatOutcome, String> {
    if message.trim().is_empty() {
        return Err("invalid_request".into());
    }

    state
        .coordinator
        .run(ChatInput::new(message, AutonomyProfile::Observe))
        .await
        .map_err(|_| "agent_failed".to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn evidence(
    state: State<'_, DesktopState>,
    trace_id: String,
) -> Result<Vec<EvidenceRecord>, String> {
    let trace_id = TraceId::from_str(&trace_id).map_err(|_| "invalid_trace_id".to_string())?;
    state
        .evidence
        .list_by_trace(trace_id)
        .map_err(|_| "evidence_unavailable".to_string())
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let workspace = env::var_os("HERMESCLAW_WORKSPACE")
                .map(PathBuf::from)
                .unwrap_or(env::current_dir()?);
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let database = env::var_os("HERMESCLAW_DB")
                .map(PathBuf::from)
                .unwrap_or_else(|| data_dir.join("hermesclaw.db"));

            let mut registry = CapabilityRegistry::new();
            registry.register(WorkspaceListCapability::new(workspace)?);
            let coordinator = TurnCoordinator::new(
                DeterministicProvider,
                registry,
                EvidenceStore::open(database)?,
            );
            let evidence = coordinator.evidence_store();
            app.manage(DesktopState {
                coordinator,
                evidence,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![chat, evidence])
        .run(tauri::generate_context!())
        .expect("failed to run HermesClaw desktop");
}
