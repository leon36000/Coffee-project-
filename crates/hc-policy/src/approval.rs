use crate::{
    ActionDigest, ApprovalCryptoError, ApprovalKeyProvider, CheckpointCipher, CheckpointContext,
    DigestError, EncryptedCheckpoint, VerifiedApproval,
};
use chrono::{DateTime, Duration, Utc};
use hc_domain::{ApprovalId, ApprovalPrompt, ApprovalRequest, ApprovalStatus, ToolCall};
use std::sync::Arc;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq)]
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

    fn list_pending(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<ApprovalRequest>, ApprovalRepositoryError>;

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

    fn expire_due(&self, now: DateTime<Utc>) -> Result<usize, ApprovalRepositoryError>;

    fn load_executing_for_recovery(&self) -> Result<Vec<StoredApproval>, ApprovalRepositoryError>;
}

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

#[derive(Clone)]
pub struct ApprovalService {
    repository: Arc<dyn ApprovalRepository>,
    cipher: CheckpointCipher,
}

pub struct NewApproval<'a> {
    pub trace_id: hc_domain::TraceId,
    pub mission_id: hc_domain::MissionId,
    pub call: &'a ToolCall,
    pub reason: &'a str,
    pub summary: serde_json::Value,
    pub checkpoint: &'a [u8],
    pub requested_at: DateTime<Utc>,
    pub lifetime: Duration,
}

pub struct ApprovedCheckpoint {
    pub request: ApprovalRequest,
    pub verified_approval: VerifiedApproval,
    pub plaintext: Vec<u8>,
}

impl ApprovalService {
    pub fn new(
        repository: Arc<dyn ApprovalRepository>,
        keys: Arc<dyn ApprovalKeyProvider>,
    ) -> Self {
        Self {
            repository,
            cipher: CheckpointCipher::new(keys),
        }
    }

    pub fn create_pending(
        &self,
        new_approval: NewApproval<'_>,
    ) -> Result<ApprovalPrompt, ApprovalError> {
        if new_approval.lifetime <= Duration::zero() {
            return Err(ApprovalError::InvalidLifetime);
        }

        let action_digest = ActionDigest::for_call(new_approval.call)?;
        let approval_id = ApprovalId::new();
        let expires_at = new_approval
            .requested_at
            .checked_add_signed(new_approval.lifetime)
            .ok_or(ApprovalError::InvalidLifetime)?;
        let request = ApprovalRequest {
            approval_id,
            trace_id: new_approval.trace_id,
            mission_id: new_approval.mission_id,
            capability_id: new_approval.call.capability_id.clone(),
            action_digest: action_digest.as_str().to_owned(),
            reason: new_approval.reason.to_owned(),
            summary: new_approval.summary,
            status: ApprovalStatus::Pending,
            requested_at: new_approval.requested_at,
            expires_at,
            decided_at: None,
            decision_actor: None,
            failure_code: None,
        };
        let context = CheckpointContext {
            approval_id,
            trace_id: request.trace_id,
            mission_id: request.mission_id,
            action_digest,
            schema_version: 1,
        };
        let checkpoint = self.cipher.seal(&context, new_approval.checkpoint)?;
        self.repository.create_pending(&request, &checkpoint)?;
        Ok(prompt_from_request(&request))
    }

    pub fn list_pending(&self, now: DateTime<Utc>) -> Result<Vec<ApprovalRequest>, ApprovalError> {
        Ok(self.repository.list_pending(now)?)
    }

    pub fn load_public(
        &self,
        approval_id: ApprovalId,
        now: DateTime<Utc>,
    ) -> Result<ApprovalRequest, ApprovalError> {
        Ok(self.repository.load_public(approval_id, now)?)
    }

    pub fn begin_execution(
        &self,
        approval_id: ApprovalId,
        actor: &str,
        now: DateTime<Utc>,
    ) -> Result<ApprovedCheckpoint, ApprovalError> {
        let stored = self.repository.begin_execution(approval_id, actor, now)?;
        self.decrypt_stored(stored)
    }

    pub fn deny(
        &self,
        approval_id: ApprovalId,
        actor: &str,
        now: DateTime<Utc>,
    ) -> Result<ApprovalRequest, ApprovalError> {
        Ok(self.repository.mark_denied(approval_id, actor, now)?)
    }

    pub fn mark_consumed(
        &self,
        approval_id: ApprovalId,
        now: DateTime<Utc>,
    ) -> Result<(), ApprovalError> {
        Ok(self.repository.mark_consumed(approval_id, now)?)
    }

    pub fn mark_failed(
        &self,
        approval_id: ApprovalId,
        failure_code: &str,
        now: DateTime<Utc>,
        erase_checkpoint: bool,
    ) -> Result<(), ApprovalError> {
        Ok(self
            .repository
            .mark_failed(approval_id, failure_code, now, erase_checkpoint)?)
    }

    pub fn executing_for_recovery(&self) -> Result<Vec<ApprovedCheckpoint>, ApprovalError> {
        self.repository
            .load_executing_for_recovery()?
            .into_iter()
            .map(|stored| self.decrypt_stored(stored))
            .collect()
    }

    fn decrypt_stored(&self, stored: StoredApproval) -> Result<ApprovedCheckpoint, ApprovalError> {
        let action_digest = ActionDigest::parse(&stored.request.action_digest)?;
        let context = CheckpointContext {
            approval_id: stored.request.approval_id,
            trace_id: stored.request.trace_id,
            mission_id: stored.request.mission_id,
            action_digest: action_digest.clone(),
            schema_version: 1,
        };
        let plaintext = self.cipher.open(&context, &stored.encrypted_checkpoint)?;
        Ok(ApprovedCheckpoint {
            request: stored.request,
            verified_approval: VerifiedApproval::new(
                approval_id_from_context(&context),
                action_digest,
            ),
            plaintext,
        })
    }
}

fn approval_id_from_context(context: &CheckpointContext) -> ApprovalId {
    context.approval_id
}

fn prompt_from_request(request: &ApprovalRequest) -> ApprovalPrompt {
    ApprovalPrompt {
        approval_id: request.approval_id,
        capability_id: request.capability_id.clone(),
        reason: request.reason.clone(),
        summary: request.summary.clone(),
        expires_at: request.expires_at,
    }
}

#[derive(Debug, Error)]
pub enum ApprovalError {
    #[error(transparent)]
    Repository(#[from] ApprovalRepositoryError),
    #[error(transparent)]
    Digest(#[from] DigestError),
    #[error(transparent)]
    Crypto(#[from] ApprovalCryptoError),
    #[error("approval lifetime must be positive and representable")]
    InvalidLifetime,
    #[error("approval checkpoint serialization failed: {0}")]
    Serialization(String),
    #[error("approval checkpoint action digest mismatch")]
    DigestMismatch,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InMemoryApprovalKeyProvider, PolicyContext, PolicyKernel};
    use hc_domain::{ApprovalStatus, AutonomyProfile, PolicyDecision, ToolCall};
    use serde_json::json;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemoryRepository {
        stored: Mutex<Option<StoredApproval>>,
    }

    impl ApprovalRepository for MemoryRepository {
        fn create_pending(
            &self,
            request: &ApprovalRequest,
            checkpoint: &EncryptedCheckpoint,
        ) -> Result<(), ApprovalRepositoryError> {
            let mut stored = self.stored.lock().unwrap();
            if stored.is_some() {
                return Err(ApprovalRepositoryError::Duplicate);
            }
            *stored = Some(StoredApproval {
                request: request.clone(),
                encrypted_checkpoint: checkpoint.clone(),
            });
            Ok(())
        }

        fn list_pending(
            &self,
            _now: DateTime<Utc>,
        ) -> Result<Vec<ApprovalRequest>, ApprovalRepositoryError> {
            Ok(self
                .stored
                .lock()
                .unwrap()
                .as_ref()
                .filter(|stored| stored.request.status == ApprovalStatus::Pending)
                .map(|stored| vec![stored.request.clone()])
                .unwrap_or_default())
        }

        fn load_public(
            &self,
            approval_id: ApprovalId,
            _now: DateTime<Utc>,
        ) -> Result<ApprovalRequest, ApprovalRepositoryError> {
            self.stored
                .lock()
                .unwrap()
                .as_ref()
                .filter(|stored| stored.request.approval_id == approval_id)
                .map(|stored| stored.request.clone())
                .ok_or(ApprovalRepositoryError::NotFound)
        }

        fn begin_execution(
            &self,
            approval_id: ApprovalId,
            actor: &str,
            now: DateTime<Utc>,
        ) -> Result<StoredApproval, ApprovalRepositoryError> {
            let mut guard = self.stored.lock().unwrap();
            let stored = guard
                .as_mut()
                .filter(|stored| stored.request.approval_id == approval_id)
                .ok_or(ApprovalRepositoryError::NotFound)?;
            if stored.request.status != ApprovalStatus::Pending {
                return Err(ApprovalRepositoryError::AlreadyDecided);
            }
            stored.request.status = ApprovalStatus::Executing;
            stored.request.decision_actor = Some(actor.to_owned());
            stored.request.decided_at = Some(now);
            Ok(stored.clone())
        }

        fn mark_consumed(
            &self,
            approval_id: ApprovalId,
            _now: DateTime<Utc>,
        ) -> Result<(), ApprovalRepositoryError> {
            let mut guard = self.stored.lock().unwrap();
            let stored = guard
                .as_mut()
                .filter(|stored| stored.request.approval_id == approval_id)
                .ok_or(ApprovalRepositoryError::NotFound)?;
            stored.request.status = ApprovalStatus::Consumed;
            Ok(())
        }

        fn mark_denied(
            &self,
            approval_id: ApprovalId,
            actor: &str,
            now: DateTime<Utc>,
        ) -> Result<ApprovalRequest, ApprovalRepositoryError> {
            let mut guard = self.stored.lock().unwrap();
            let stored = guard
                .as_mut()
                .filter(|stored| stored.request.approval_id == approval_id)
                .ok_or(ApprovalRepositoryError::NotFound)?;
            stored.request.status = ApprovalStatus::Denied;
            stored.request.decision_actor = Some(actor.to_owned());
            stored.request.decided_at = Some(now);
            Ok(stored.request.clone())
        }

        fn mark_failed(
            &self,
            approval_id: ApprovalId,
            failure_code: &str,
            _now: DateTime<Utc>,
            _erase_checkpoint: bool,
        ) -> Result<(), ApprovalRepositoryError> {
            let mut guard = self.stored.lock().unwrap();
            let stored = guard
                .as_mut()
                .filter(|stored| stored.request.approval_id == approval_id)
                .ok_or(ApprovalRepositoryError::NotFound)?;
            stored.request.status = ApprovalStatus::Failed;
            stored.request.failure_code = Some(failure_code.to_owned());
            Ok(())
        }

        fn expire_due(&self, _now: DateTime<Utc>) -> Result<usize, ApprovalRepositoryError> {
            Ok(0)
        }

        fn load_executing_for_recovery(
            &self,
        ) -> Result<Vec<StoredApproval>, ApprovalRepositoryError> {
            Ok(self
                .stored
                .lock()
                .unwrap()
                .as_ref()
                .filter(|stored| stored.request.status == ApprovalStatus::Executing)
                .cloned()
                .into_iter()
                .collect())
        }
    }

    #[test]
    fn service_persists_and_decrypts_exact_pending_checkpoint() {
        let repository = Arc::new(MemoryRepository::default());
        let service = ApprovalService::new(
            repository,
            Arc::new(InMemoryApprovalKeyProvider::new("test-key", [7_u8; 32])),
        );
        let trace_id = hc_domain::TraceId::new();
        let mission_id = hc_domain::MissionId::new();
        let call = ToolCall::workspace_write_create("call-1", "notes.txt", "secret");
        let requested_at = Utc::now();

        let prompt = service
            .create_pending(NewApproval {
                trace_id,
                mission_id,
                call: &call,
                reason: "approval required",
                summary: json!({
                    "path": "notes.txt",
                    "mode": "create_new",
                    "bytes": 6,
                    "sha256": "a".repeat(64)
                }),
                checkpoint: b"encrypted continuation",
                requested_at,
                lifetime: Duration::hours(24),
            })
            .unwrap();

        assert_eq!(prompt.capability_id, "workspace.write");
        assert_eq!(prompt.summary["path"], "notes.txt");
        assert!(prompt.summary.get("content").is_none());
        let approved = service
            .begin_execution(prompt.approval_id, "local_user", requested_at)
            .unwrap();
        assert_eq!(approved.plaintext, b"encrypted continuation");
        assert_eq!(approved.request.status, ApprovalStatus::Executing);
        assert_eq!(
            PolicyKernel::evaluate_with_context(
                PolicyContext::new(AutonomyProfile::Assist)
                    .with_approval(&approved.verified_approval),
                &call,
            ),
            PolicyDecision::Allow
        );
    }

    #[test]
    fn service_rejects_non_positive_lifetime() {
        let service = ApprovalService::new(
            Arc::new(MemoryRepository::default()),
            Arc::new(InMemoryApprovalKeyProvider::new("test-key", [7_u8; 32])),
        );
        let call = ToolCall::workspace_write_create("call-1", "notes.txt", "secret");
        let error = service
            .create_pending(NewApproval {
                trace_id: hc_domain::TraceId::new(),
                mission_id: hc_domain::MissionId::new(),
                call: &call,
                reason: "approval required",
                summary: json!({}),
                checkpoint: b"checkpoint",
                requested_at: Utc::now(),
                lifetime: Duration::zero(),
            })
            .unwrap_err();
        assert!(matches!(error, ApprovalError::InvalidLifetime));
    }
}
