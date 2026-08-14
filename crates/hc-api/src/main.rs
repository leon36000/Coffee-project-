use hc_agent::TurnCoordinator;
use hc_api::build_router;
use hc_models::DeterministicProvider;
use hc_state::EvidenceStore;
use hc_tools::{CapabilityRegistry, WorkspaceListCapability};
use std::{env, error::Error, path::PathBuf};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let workspace = env::var_os("HERMESCLAW_WORKSPACE")
        .map(PathBuf::from)
        .unwrap_or(env::current_dir()?);
    let database = env::var_os("HERMESCLAW_DB")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("hermesclaw.db"));
    let bind = env::var("HERMESCLAW_BIND").unwrap_or_else(|_| "127.0.0.1:7777".into());

    let mut registry = CapabilityRegistry::new();
    registry.register(WorkspaceListCapability::new(workspace)?);
    let coordinator = TurnCoordinator::new(
        DeterministicProvider,
        registry,
        EvidenceStore::open(database)?,
    );

    let listener = tokio::net::TcpListener::bind(&bind).await?;
    println!("HermesClaw vertical proof listening on http://{bind}");
    axum::serve(listener, build_router(coordinator)).await?;
    Ok(())
}
