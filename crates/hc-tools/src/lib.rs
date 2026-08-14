use async_trait::async_trait;
use hc_domain::{ToolCall, ToolResult};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq)]
pub struct CapabilityExecution {
    pub result: ToolResult,
    pub evidence_payload: serde_json::Value,
}

#[async_trait]
pub trait Capability: Send + Sync {
    fn id(&self) -> &'static str;
    async fn execute(&self, call: &ToolCall) -> Result<CapabilityExecution, CapabilityError>;
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

    pub async fn execute(&self, call: &ToolCall) -> Result<CapabilityExecution, CapabilityError> {
        let capability = self
            .capabilities
            .get(&call.capability_id)
            .ok_or_else(|| CapabilityError::UnknownCapability(call.capability_id.clone()))?;
        capability.execute(call).await
    }
}

#[derive(Clone, Debug)]
pub struct WorkspaceBoundary {
    workspace_root: PathBuf,
}

impl WorkspaceBoundary {
    pub fn new(workspace_root: impl AsRef<Path>) -> Result<Self, CapabilityError> {
        let workspace_root = std::fs::canonicalize(workspace_root)?;
        if !workspace_root.is_dir() {
            return Err(CapabilityError::WorkspaceRootNotDirectory);
        }
        Ok(Self { workspace_root })
    }

    pub fn resolve_existing(&self, requested: &str) -> Result<PathBuf, CapabilityError> {
        if requested.is_empty() {
            return Err(CapabilityError::InvalidArguments);
        }
        let canonical = std::fs::canonicalize(self.workspace_root.join(requested))?;
        if !canonical.starts_with(&self.workspace_root) {
            return Err(CapabilityError::PathEscapesWorkspace);
        }
        Ok(canonical)
    }

    pub fn relative_path(&self, path: &Path) -> Result<String, CapabilityError> {
        let relative = path
            .strip_prefix(&self.workspace_root)
            .map_err(|_| CapabilityError::PathEscapesWorkspace)?;
        Ok(relative.to_string_lossy().replace('\\', "/"))
    }
}

pub struct WorkspaceListCapability {
    boundary: WorkspaceBoundary,
}

impl WorkspaceListCapability {
    pub fn new(workspace_root: impl AsRef<Path>) -> Result<Self, CapabilityError> {
        Ok(Self {
            boundary: WorkspaceBoundary::new(workspace_root)?,
        })
    }
}

#[async_trait]
impl Capability for WorkspaceListCapability {
    fn id(&self) -> &'static str {
        "workspace.list"
    }

    async fn execute(&self, call: &ToolCall) -> Result<CapabilityExecution, CapabilityError> {
        let requested = call
            .arguments
            .get("path")
            .and_then(|value| value.as_str())
            .ok_or(CapabilityError::InvalidArguments)?;
        let target = self.boundary.resolve_existing(requested)?;
        if !target.is_dir() {
            return Err(CapabilityError::TargetNotDirectory);
        }

        let mut reader = tokio::fs::read_dir(&target).await?;
        let mut entries = Vec::new();
        while let Some(entry) = reader.next_entry().await? {
            let relative = self.boundary.relative_path(&entry.path())?;
            entries.push(relative);
        }
        entries.sort();

        let output = json!({ "entries": entries });
        Ok(CapabilityExecution {
            result: ToolResult {
                call_id: call.id.clone(),
                capability_id: call.capability_id.clone(),
                output: output.clone(),
            },
            evidence_payload: output,
        })
    }
}

pub const MAX_WORKSPACE_READ_BYTES: usize = 64 * 1024;

pub struct WorkspaceReadCapability {
    boundary: WorkspaceBoundary,
}

impl WorkspaceReadCapability {
    pub fn new(workspace_root: impl AsRef<Path>) -> Result<Self, CapabilityError> {
        Ok(Self {
            boundary: WorkspaceBoundary::new(workspace_root)?,
        })
    }
}

#[async_trait]
impl Capability for WorkspaceReadCapability {
    fn id(&self) -> &'static str {
        "workspace.read"
    }

    async fn execute(&self, call: &ToolCall) -> Result<CapabilityExecution, CapabilityError> {
        let requested = call
            .arguments
            .get("path")
            .and_then(|value| value.as_str())
            .ok_or(CapabilityError::InvalidArguments)?;
        let target = self.boundary.resolve_existing(requested)?;
        let metadata = tokio::fs::metadata(&target).await?;
        if !metadata.is_file() {
            return Err(CapabilityError::TargetNotFile);
        }

        let metadata_bytes = usize::try_from(metadata.len()).unwrap_or(usize::MAX);
        enforce_read_limit(metadata_bytes)?;

        let bytes = tokio::fs::read(&target).await?;
        enforce_read_limit(bytes.len())?;
        if bytes.contains(&0) {
            return Err(CapabilityError::FileContainsNul);
        }

        let sha256 = hex::encode(Sha256::digest(&bytes));
        let bytes_len = bytes.len();
        let content = String::from_utf8(bytes).map_err(|_| CapabilityError::InvalidUtf8)?;
        let relative_path = self.boundary.relative_path(&target)?;
        let output = json!({
            "path": relative_path,
            "content": content,
            "bytes": bytes_len,
        });
        let evidence_payload = json!({
            "path": relative_path,
            "bytes": bytes_len,
            "sha256": sha256,
        });

        Ok(CapabilityExecution {
            result: ToolResult {
                call_id: call.id.clone(),
                capability_id: call.capability_id.clone(),
                output,
            },
            evidence_payload,
        })
    }
}

fn enforce_read_limit(actual_bytes: usize) -> Result<(), CapabilityError> {
    if actual_bytes > MAX_WORKSPACE_READ_BYTES {
        Err(CapabilityError::FileTooLarge {
            max_bytes: MAX_WORKSPACE_READ_BYTES,
            actual_bytes,
        })
    } else {
        Ok(())
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
    #[error("target path is not a regular file")]
    TargetNotFile,
    #[error("file exceeds {max_bytes} byte limit: {actual_bytes} bytes")]
    FileTooLarge {
        max_bytes: usize,
        actual_bytes: usize,
    },
    #[error("file contains a NUL byte")]
    FileContainsNul,
    #[error("file is not valid UTF-8 text")]
    InvalidUtf8,
    #[error("capability I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use hc_domain::ToolCall;
    use serde_json::json;
    use std::{fs, path::Path};
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

        assert_eq!(
            result.result.output,
            json!({"entries": ["alpha.txt", "zeta.txt"]})
        );
        assert_eq!(result.evidence_payload, result.result.output);
    }

    #[test]
    fn capability_execution_can_hold_different_result_and_evidence() {
        let execution = CapabilityExecution {
            result: ToolResult {
                call_id: "call-1".into(),
                capability_id: "workspace.read".into(),
                output: json!({"content": "secret"}),
            },
            evidence_payload: json!({"sha256": "digest"}),
        };

        assert_ne!(execution.result.output, execution.evidence_payload);
    }

    fn read_call(path: &str) -> ToolCall {
        ToolCall::workspace_read("call-read", path)
    }

    fn registry_with_read(workspace: &Path) -> CapabilityRegistry {
        let mut registry = CapabilityRegistry::new();
        registry.register(WorkspaceReadCapability::new(workspace).unwrap());
        registry
    }

    #[tokio::test]
    async fn workspace_read_returns_text_and_sanitized_evidence() {
        let workspace = tempdir().expect("workspace");
        fs::create_dir(workspace.path().join("docs")).unwrap();
        fs::write(workspace.path().join("docs/notes.md"), "hello").unwrap();

        let registry = registry_with_read(workspace.path());
        let execution = registry.execute(&read_call("docs/notes.md")).await.unwrap();

        assert_eq!(
            execution.result.output,
            json!({"path": "docs/notes.md", "content": "hello", "bytes": 5})
        );
        assert_eq!(
            execution.evidence_payload,
            json!({
                "path": "docs/notes.md",
                "bytes": 5,
                "sha256": "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
            })
        );
        assert!(execution.evidence_payload.get("content").is_none());
    }

    #[tokio::test]
    async fn workspace_read_rejects_parent_escape() {
        let parent = tempdir().unwrap();
        let workspace = parent.path().join("workspace");
        fs::create_dir(&workspace).unwrap();
        fs::write(parent.path().join("secret.txt"), "secret").unwrap();
        let registry = registry_with_read(&workspace);

        let error = registry
            .execute(&read_call("../secret.txt"))
            .await
            .unwrap_err();

        assert_eq!(error.to_string(), "path escapes workspace root");
    }

    #[tokio::test]
    async fn workspace_read_rejects_absolute_escape() {
        let workspace = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let secret = outside.path().join("secret.txt");
        fs::write(&secret, "secret").unwrap();
        let registry = registry_with_read(workspace.path());

        let error = registry
            .execute(&read_call(secret.to_str().unwrap()))
            .await
            .unwrap_err();

        assert_eq!(error.to_string(), "path escapes workspace root");
    }

    #[tokio::test]
    async fn workspace_read_rejects_directory() {
        let workspace = tempdir().unwrap();
        fs::create_dir(workspace.path().join("docs")).unwrap();
        let registry = registry_with_read(workspace.path());

        let error = registry.execute(&read_call("docs")).await.unwrap_err();

        assert_eq!(error.to_string(), "target path is not a regular file");
    }

    #[tokio::test]
    async fn workspace_read_rejects_file_larger_than_limit() {
        let workspace = tempdir().unwrap();
        fs::write(
            workspace.path().join("large.txt"),
            vec![b'a'; MAX_WORKSPACE_READ_BYTES + 1],
        )
        .unwrap();
        let registry = registry_with_read(workspace.path());

        let error = registry.execute(&read_call("large.txt")).await.unwrap_err();

        assert_eq!(
            error.to_string(),
            "file exceeds 65536 byte limit: 65537 bytes"
        );
    }

    #[tokio::test]
    async fn workspace_read_rejects_invalid_utf8() {
        let workspace = tempdir().unwrap();
        fs::write(workspace.path().join("binary.dat"), [0xff, 0xfe]).unwrap();
        let registry = registry_with_read(workspace.path());

        let error = registry
            .execute(&read_call("binary.dat"))
            .await
            .unwrap_err();

        assert_eq!(error.to_string(), "file is not valid UTF-8 text");
    }

    #[tokio::test]
    async fn workspace_read_rejects_nul_content() {
        let workspace = tempdir().unwrap();
        fs::write(workspace.path().join("nul.txt"), b"a\0b").unwrap();
        let registry = registry_with_read(workspace.path());

        let error = registry.execute(&read_call("nul.txt")).await.unwrap_err();

        assert_eq!(error.to_string(), "file contains a NUL byte");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn workspace_read_rejects_symlink_escape() {
        use std::os::unix::fs::symlink;

        let workspace = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let secret = outside.path().join("secret.txt");
        fs::write(&secret, "secret").unwrap();
        symlink(&secret, workspace.path().join("outside-link")).unwrap();
        let registry = registry_with_read(workspace.path());

        let error = registry
            .execute(&read_call("outside-link"))
            .await
            .unwrap_err();

        assert_eq!(error.to_string(), "path escapes workspace root");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn workspace_read_accepts_internal_symlink_and_reports_canonical_path() {
        use std::os::unix::fs::symlink;

        let workspace = tempdir().unwrap();
        fs::write(workspace.path().join("real.txt"), "hello").unwrap();
        symlink(
            workspace.path().join("real.txt"),
            workspace.path().join("alias.txt"),
        )
        .unwrap();
        let registry = registry_with_read(workspace.path());

        let execution = registry.execute(&read_call("alias.txt")).await.unwrap();

        assert_eq!(execution.result.output["path"], "real.txt");
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
