use async_trait::async_trait;
use hc_domain::{ToolCall, ToolResult};
use serde_json::json;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[async_trait]
pub trait Capability: Send + Sync {
    fn id(&self) -> &'static str;
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, CapabilityError>;
}

#[derive(Default)]
pub struct CapabilityRegistry {
    capabilities: HashMap<String, Box<dyn Capability>>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<C>(&mut self, capability: C)
    where
        C: Capability + 'static,
    {
        self.capabilities
            .insert(capability.id().to_owned(), Box::new(capability));
    }

    pub async fn execute(&self, call: &ToolCall) -> Result<ToolResult, CapabilityError> {
        let capability = self
            .capabilities
            .get(&call.capability_id)
            .ok_or_else(|| CapabilityError::UnknownCapability(call.capability_id.clone()))?;
        capability.execute(call).await
    }
}

pub struct WorkspaceListCapability {
    workspace_root: PathBuf,
}

impl WorkspaceListCapability {
    pub fn new(workspace_root: impl AsRef<Path>) -> Result<Self, CapabilityError> {
        let workspace_root = std::fs::canonicalize(workspace_root)?;
        if !workspace_root.is_dir() {
            return Err(CapabilityError::WorkspaceRootNotDirectory);
        }
        Ok(Self { workspace_root })
    }

    fn resolve_target(&self, requested: &str) -> Result<PathBuf, CapabilityError> {
        let candidate = self.workspace_root.join(requested);
        let canonical = std::fs::canonicalize(candidate)?;
        if !canonical.starts_with(&self.workspace_root) {
            return Err(CapabilityError::PathEscapesWorkspace);
        }
        Ok(canonical)
    }
}

#[async_trait]
impl Capability for WorkspaceListCapability {
    fn id(&self) -> &'static str {
        "workspace.list"
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, CapabilityError> {
        let requested = call
            .arguments
            .get("path")
            .and_then(|value| value.as_str())
            .ok_or(CapabilityError::InvalidArguments)?;
        let target = self.resolve_target(requested)?;
        if !target.is_dir() {
            return Err(CapabilityError::TargetNotDirectory);
        }

        let mut reader = tokio::fs::read_dir(&target).await?;
        let mut entries = Vec::new();
        while let Some(entry) = reader.next_entry().await? {
            let relative = entry
                .path()
                .strip_prefix(&self.workspace_root)
                .map_err(|_| CapabilityError::PathEscapesWorkspace)?
                .to_string_lossy()
                .replace('\\', "/");
            entries.push(relative);
        }
        entries.sort();

        Ok(ToolResult {
            call_id: call.id.clone(),
            capability_id: call.capability_id.clone(),
            output: json!({ "entries": entries }),
        })
    }
}

#[derive(Debug, Error)]
pub enum CapabilityError {
    #[error("unknown capability: {0}")]
    UnknownCapability(String),
    #[error("invalid capability arguments")]
    InvalidArguments,
    #[error("path escapes workspace root")]
    PathEscapesWorkspace,
    #[error("workspace root is not a directory")]
    WorkspaceRootNotDirectory,
    #[error("target path is not a directory")]
    TargetNotDirectory,
    #[error("capability I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use hc_domain::ToolCall;
    use serde_json::json;
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn workspace_list_returns_sorted_relative_entries() {
        let workspace = tempdir().expect("temp workspace");
        fs::write(workspace.path().join("zeta.txt"), "z").unwrap();
        fs::write(workspace.path().join("alpha.txt"), "a").unwrap();

        let mut registry = CapabilityRegistry::new();
        registry.register(WorkspaceListCapability::new(workspace.path()).unwrap());

        let result = registry
            .execute(&ToolCall::workspace_list("call-1", "."))
            .await
            .expect("list workspace");

        assert_eq!(result.output, json!({"entries": ["alpha.txt", "zeta.txt"]}));
    }

    #[tokio::test]
    async fn workspace_list_rejects_parent_escape() {
        let workspace = tempdir().expect("temp workspace");
        let mut registry = CapabilityRegistry::new();
        registry.register(WorkspaceListCapability::new(workspace.path()).unwrap());

        let error = registry
            .execute(&ToolCall::workspace_list("call-1", "../"))
            .await
            .expect_err("parent escape must be denied");

        assert_eq!(error.to_string(), "path escapes workspace root");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn workspace_list_rejects_symlink_escape() {
        use std::os::unix::fs::symlink;

        let workspace = tempdir().expect("temp workspace");
        let outside = tempdir().expect("outside directory");
        fs::write(outside.path().join("secret.txt"), "secret").unwrap();
        symlink(outside.path(), workspace.path().join("outside-link")).unwrap();

        let mut registry = CapabilityRegistry::new();
        registry.register(WorkspaceListCapability::new(workspace.path()).unwrap());

        let error = registry
            .execute(&ToolCall::workspace_list("call-1", "outside-link"))
            .await
            .expect_err("symlink escape must be denied");

        assert_eq!(error.to_string(), "path escapes workspace root");
    }
}
