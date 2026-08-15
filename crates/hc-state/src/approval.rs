use crate::SharedConnection;
use chrono::{DateTime, Utc};
use hc_domain::{ApprovalId, ApprovalRequest, ApprovalStatus};
use hc_policy::{ApprovalRepository, ApprovalRepositoryError, EncryptedCheckpoint, StoredApproval};
use rusqlite::{
    params, Connection, Error as SqliteError, ErrorCode, OptionalExtension, Row,
    TransactionBehavior,
};
use std::sync::MutexGuard;

const APPROVAL_COLUMNS: &str = "approval_id, trace_id, mission_id, capability_id,
    action_digest, reason, summary, status, requested_at, expires_at,
    decided_at, decision_actor, failure_code, ciphertext, nonce, key_id";

#[derive(Clone)]
pub struct SqliteApprovalRepository {
    pub(crate) connection: SharedConnection,
}

impl SqliteApprovalRepository {
    pub(crate) fn new(connection: SharedConnection) -> Self {
        Self { connection }
    }

    fn lock(&self) -> Result<MutexGuard<'_, Connection>, ApprovalRepositoryError> {
        self.connection
            .lock()
            .map_err(|_| ApprovalRepositoryError::Backend("SQLite connection lock poisoned".into()))
    }

    #[cfg(test)]
    pub(crate) fn checkpoint_presence_for_test(
        &self,
        approval_id: ApprovalId,
    ) -> Result<(bool, bool, bool), ApprovalRepositoryError> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT ciphertext IS NOT NULL, nonce IS NOT NULL, key_id IS NOT NULL
                 FROM approvals WHERE approval_id = ?1",
                params![approval_id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(backend)?
            .ok_or(ApprovalRepositoryError::NotFound)
    }
}

impl ApprovalRepository for SqliteApprovalRepository {
    fn create_pending(
        &self,
        request: &ApprovalRequest,
        checkpoint: &EncryptedCheckpoint,
    ) -> Result<(), ApprovalRepositoryError> {
        if request.status != ApprovalStatus::Pending {
            return Err(ApprovalRepositoryError::Backend(
                "new approval must have pending status".into(),
            ));
        }

        let connection = self.lock()?;
        let result = connection.execute(
            "INSERT INTO approvals (
                approval_id, trace_id, mission_id, capability_id, action_digest,
                reason, summary, status, requested_at, expires_at, decided_at,
                decision_actor, failure_code, ciphertext, nonce, key_id
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                       NULL, NULL, NULL, ?11, ?12, ?13)",
            params![
                request.approval_id.to_string(),
                request.trace_id.to_string(),
                request.mission_id.to_string(),
                request.capability_id,
                request.action_digest,
                request.reason,
                serde_json::to_string(&request.summary).map_err(backend)?,
                status_name(request.status),
                request.requested_at.to_rfc3339(),
                request.expires_at.to_rfc3339(),
                checkpoint.ciphertext,
                checkpoint.nonce.as_slice(),
                checkpoint.key_id,
            ],
        );

        match result {
            Ok(_) => Ok(()),
            Err(SqliteError::SqliteFailure(error, _))
                if error.code == ErrorCode::ConstraintViolation =>
            {
                Err(ApprovalRepositoryError::Duplicate)
            }
            Err(error) => Err(backend(error)),
        }
    }

    fn list_pending(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<ApprovalRequest>, ApprovalRepositoryError> {
        self.expire_due(now)?;
        let connection = self.lock()?;
        let sql = format!(
            "SELECT {APPROVAL_COLUMNS} FROM approvals
             WHERE status = 'pending' ORDER BY requested_at ASC, approval_id ASC"
        );
        let mut statement = connection.prepare(&sql).map_err(backend)?;
        let rows = statement
            .query_map([], RawApprovalRow::from_row)
            .map_err(backend)?;
        rows.map(|row| decode_request(row.map_err(backend)?))
            .collect()
    }

    fn load_public(
        &self,
        approval_id: ApprovalId,
        now: DateTime<Utc>,
    ) -> Result<ApprovalRequest, ApprovalRepositoryError> {
        self.expire_due(now)?;
        let connection = self.lock()?;
        let raw = query_raw(&connection, approval_id)?;
        decode_request(raw)
    }

    fn begin_execution(
        &self,
        approval_id: ApprovalId,
        actor: &str,
        now: DateTime<Utc>,
    ) -> Result<StoredApproval, ApprovalRepositoryError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(backend)?;
        let raw = query_raw(&transaction, approval_id)?;
        let mut request = decode_request(raw.clone())?;

        match request.status {
            ApprovalStatus::Pending => {}
            ApprovalStatus::Expired => return Err(ApprovalRepositoryError::Expired),
            _ => return Err(ApprovalRepositoryError::AlreadyDecided),
        }

        if request.expires_at <= now {
            transaction
                .execute(
                    "UPDATE approvals
                     SET status = 'expired', decided_at = ?2,
                         ciphertext = NULL, nonce = NULL, key_id = NULL
                     WHERE approval_id = ?1 AND status = 'pending'",
                    params![approval_id.to_string(), now.to_rfc3339()],
                )
                .map_err(backend)?;
            transaction.commit().map_err(backend)?;
            return Err(ApprovalRepositoryError::Expired);
        }

        let changed = transaction
            .execute(
                "UPDATE approvals
                 SET status = 'executing', decided_at = ?2, decision_actor = ?3
                 WHERE approval_id = ?1 AND status = 'pending'",
                params![approval_id.to_string(), now.to_rfc3339(), actor],
            )
            .map_err(backend)?;
        if changed != 1 {
            return Err(classify_transition_failure(&transaction, approval_id)?);
        }

        let checkpoint = decode_checkpoint(&raw)?;
        request.status = ApprovalStatus::Executing;
        request.decided_at = Some(now);
        request.decision_actor = Some(actor.to_owned());
        transaction.commit().map_err(backend)?;

        Ok(StoredApproval {
            request,
            encrypted_checkpoint: checkpoint,
        })
    }

    fn mark_consumed(
        &self,
        approval_id: ApprovalId,
        now: DateTime<Utc>,
    ) -> Result<(), ApprovalRepositoryError> {
        let connection = self.lock()?;
        let changed = connection
            .execute(
                "UPDATE approvals
                 SET status = 'consumed', decided_at = COALESCE(decided_at, ?2),
                     ciphertext = NULL, nonce = NULL, key_id = NULL
                 WHERE approval_id = ?1 AND status = 'executing'",
                params![approval_id.to_string(), now.to_rfc3339()],
            )
            .map_err(backend)?;
        if changed == 1 {
            Ok(())
        } else {
            Err(classify_transition_failure(&connection, approval_id)?)
        }
    }

    fn mark_denied(
        &self,
        approval_id: ApprovalId,
        actor: &str,
        now: DateTime<Utc>,
    ) -> Result<ApprovalRequest, ApprovalRepositoryError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(backend)?;
        let raw = query_raw(&transaction, approval_id)?;
        let mut request = decode_request(raw)?;

        match request.status {
            ApprovalStatus::Pending => {}
            ApprovalStatus::Expired => return Err(ApprovalRepositoryError::Expired),
            _ => return Err(ApprovalRepositoryError::AlreadyDecided),
        }
        if request.expires_at <= now {
            transaction
                .execute(
                    "UPDATE approvals
                     SET status = 'expired', decided_at = ?2,
                         ciphertext = NULL, nonce = NULL, key_id = NULL
                     WHERE approval_id = ?1 AND status = 'pending'",
                    params![approval_id.to_string(), now.to_rfc3339()],
                )
                .map_err(backend)?;
            transaction.commit().map_err(backend)?;
            return Err(ApprovalRepositoryError::Expired);
        }

        let changed = transaction
            .execute(
                "UPDATE approvals
                 SET status = 'denied', decided_at = ?2, decision_actor = ?3,
                     ciphertext = NULL, nonce = NULL, key_id = NULL
                 WHERE approval_id = ?1 AND status = 'pending'",
                params![approval_id.to_string(), now.to_rfc3339(), actor],
            )
            .map_err(backend)?;
        if changed != 1 {
            return Err(classify_transition_failure(&transaction, approval_id)?);
        }

        request.status = ApprovalStatus::Denied;
        request.decided_at = Some(now);
        request.decision_actor = Some(actor.to_owned());
        transaction.commit().map_err(backend)?;
        Ok(request)
    }

    fn mark_failed(
        &self,
        approval_id: ApprovalId,
        failure_code: &str,
        now: DateTime<Utc>,
        erase_checkpoint: bool,
    ) -> Result<(), ApprovalRepositoryError> {
        let connection = self.lock()?;
        let sql = if erase_checkpoint {
            "UPDATE approvals
             SET status = 'failed', failure_code = ?2, decided_at = ?3,
                 ciphertext = NULL, nonce = NULL, key_id = NULL
             WHERE approval_id = ?1 AND status IN ('pending', 'executing')"
        } else {
            "UPDATE approvals
             SET status = 'failed', failure_code = ?2, decided_at = ?3
             WHERE approval_id = ?1 AND status IN ('pending', 'executing')"
        };
        let changed = connection
            .execute(
                sql,
                params![approval_id.to_string(), failure_code, now.to_rfc3339()],
            )
            .map_err(backend)?;
        if changed == 1 {
            Ok(())
        } else {
            Err(classify_transition_failure(&connection, approval_id)?)
        }
    }

    fn expire_due(&self, now: DateTime<Utc>) -> Result<usize, ApprovalRepositoryError> {
        let connection = self.lock()?;
        connection
            .execute(
                "UPDATE approvals
                 SET status = 'expired', decided_at = ?1,
                     ciphertext = NULL, nonce = NULL, key_id = NULL
                 WHERE status = 'pending' AND expires_at <= ?1",
                params![now.to_rfc3339()],
            )
            .map_err(backend)
    }

    fn load_executing_for_recovery(&self) -> Result<Vec<StoredApproval>, ApprovalRepositoryError> {
        let connection = self.lock()?;
        let sql = format!(
            "SELECT {APPROVAL_COLUMNS} FROM approvals
             WHERE status = 'executing' ORDER BY decided_at ASC, approval_id ASC"
        );
        let mut statement = connection.prepare(&sql).map_err(backend)?;
        let rows = statement
            .query_map([], RawApprovalRow::from_row)
            .map_err(backend)?;
        rows.map(|row| {
            let raw = row.map_err(backend)?;
            Ok(StoredApproval {
                request: decode_request(raw.clone())?,
                encrypted_checkpoint: decode_checkpoint(&raw)?,
            })
        })
        .collect()
    }
}

#[derive(Clone, Debug)]
struct RawApprovalRow {
    approval_id: String,
    trace_id: String,
    mission_id: String,
    capability_id: String,
    action_digest: String,
    reason: String,
    summary: String,
    status: String,
    requested_at: String,
    expires_at: String,
    decided_at: Option<String>,
    decision_actor: Option<String>,
    failure_code: Option<String>,
    ciphertext: Option<Vec<u8>>,
    nonce: Option<Vec<u8>>,
    key_id: Option<String>,
}

impl RawApprovalRow {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            approval_id: row.get(0)?,
            trace_id: row.get(1)?,
            mission_id: row.get(2)?,
            capability_id: row.get(3)?,
            action_digest: row.get(4)?,
            reason: row.get(5)?,
            summary: row.get(6)?,
            status: row.get(7)?,
            requested_at: row.get(8)?,
            expires_at: row.get(9)?,
            decided_at: row.get(10)?,
            decision_actor: row.get(11)?,
            failure_code: row.get(12)?,
            ciphertext: row.get(13)?,
            nonce: row.get(14)?,
            key_id: row.get(15)?,
        })
    }
}

fn query_raw(
    connection: &Connection,
    approval_id: ApprovalId,
) -> Result<RawApprovalRow, ApprovalRepositoryError> {
    let sql = format!("SELECT {APPROVAL_COLUMNS} FROM approvals WHERE approval_id = ?1");
    connection
        .query_row(
            &sql,
            params![approval_id.to_string()],
            RawApprovalRow::from_row,
        )
        .optional()
        .map_err(backend)?
        .ok_or(ApprovalRepositoryError::NotFound)
}

fn decode_request(raw: RawApprovalRow) -> Result<ApprovalRequest, ApprovalRepositoryError> {
    Ok(ApprovalRequest {
        approval_id: raw.approval_id.parse().map_err(backend)?,
        trace_id: raw.trace_id.parse().map_err(backend)?,
        mission_id: raw.mission_id.parse().map_err(backend)?,
        capability_id: raw.capability_id,
        action_digest: raw.action_digest,
        reason: raw.reason,
        summary: serde_json::from_str(&raw.summary).map_err(backend)?,
        status: parse_status(&raw.status)?,
        requested_at: parse_timestamp(&raw.requested_at)?,
        expires_at: parse_timestamp(&raw.expires_at)?,
        decided_at: raw.decided_at.as_deref().map(parse_timestamp).transpose()?,
        decision_actor: raw.decision_actor,
        failure_code: raw.failure_code,
    })
}

fn decode_checkpoint(raw: &RawApprovalRow) -> Result<EncryptedCheckpoint, ApprovalRepositoryError> {
    let ciphertext = raw.ciphertext.clone().ok_or_else(|| {
        ApprovalRepositoryError::Backend("approval checkpoint ciphertext is missing".into())
    })?;
    let nonce = raw.nonce.as_ref().ok_or_else(|| {
        ApprovalRepositoryError::Backend("approval checkpoint nonce is missing".into())
    })?;
    let key_id = raw.key_id.clone().ok_or_else(|| {
        ApprovalRepositoryError::Backend("approval checkpoint key identifier is missing".into())
    })?;
    if nonce.len() != 24 {
        return Err(ApprovalRepositoryError::Backend(format!(
            "approval checkpoint nonce must contain 24 bytes, got {}",
            nonce.len()
        )));
    }
    let mut nonce_bytes = [0_u8; 24];
    nonce_bytes.copy_from_slice(nonce);
    Ok(EncryptedCheckpoint {
        ciphertext,
        nonce: nonce_bytes,
        key_id,
    })
}

fn status_name(status: ApprovalStatus) -> &'static str {
    match status {
        ApprovalStatus::Pending => "pending",
        ApprovalStatus::Executing => "executing",
        ApprovalStatus::Consumed => "consumed",
        ApprovalStatus::Denied => "denied",
        ApprovalStatus::Expired => "expired",
        ApprovalStatus::Failed => "failed",
    }
}

fn parse_status(value: &str) -> Result<ApprovalStatus, ApprovalRepositoryError> {
    match value {
        "pending" => Ok(ApprovalStatus::Pending),
        "executing" => Ok(ApprovalStatus::Executing),
        "consumed" => Ok(ApprovalStatus::Consumed),
        "denied" => Ok(ApprovalStatus::Denied),
        "expired" => Ok(ApprovalStatus::Expired),
        "failed" => Ok(ApprovalStatus::Failed),
        other => Err(ApprovalRepositoryError::Backend(format!(
            "unknown approval status: {other}"
        ))),
    }
}

fn parse_timestamp(value: &str) -> Result<DateTime<Utc>, ApprovalRepositoryError> {
    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .map_err(backend)
}

fn classify_transition_failure(
    connection: &Connection,
    approval_id: ApprovalId,
) -> Result<ApprovalRepositoryError, ApprovalRepositoryError> {
    let status = connection
        .query_row(
            "SELECT status FROM approvals WHERE approval_id = ?1",
            params![approval_id.to_string()],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(backend)?;
    Ok(match status.as_deref() {
        None => ApprovalRepositoryError::NotFound,
        Some("expired") => ApprovalRepositoryError::Expired,
        Some(_) => ApprovalRepositoryError::AlreadyDecided,
    })
}

fn backend(error: impl std::fmt::Display) -> ApprovalRepositoryError {
    ApprovalRepositoryError::Backend(error.to_string())
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use hc_domain::{ApprovalId, ApprovalRequest, ApprovalStatus, MissionId, ToolCall, TraceId};
    use hc_policy::{
        ActionDigest, ApprovalRepository, ApprovalRepositoryError, CheckpointCipher,
        CheckpointContext, EncryptedCheckpoint, InMemoryApprovalKeyProvider,
    };
    use serde_json::json;
    use std::{
        fs,
        sync::{Arc, Barrier},
        thread,
    };
    use tempfile::tempdir;

    fn test_request_with_expiry(
        approval_id: ApprovalId,
        expires_at: chrono::DateTime<Utc>,
    ) -> ApprovalRequest {
        let call =
            ToolCall::workspace_write_create("call-1", "notes.txt", "sentinel pending content");
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

    #[test]
    fn pending_approval_survives_database_reopen_without_plaintext() {
        let directory = tempdir().unwrap();
        let database = directory.path().join("hermesclaw.db");
        let plaintext = b"sentinel pending content";
        let request = test_request();

        {
            let state = crate::SqliteState::open(&database).unwrap();
            let repository = state.approval_repository();
            let checkpoint = encrypted_checkpoint_containing_no_plaintext(&request, plaintext);
            repository.create_pending(&request, &checkpoint).unwrap();
        }

        let database_bytes = fs::read(&database).unwrap();
        assert!(!database_bytes
            .windows(plaintext.len())
            .any(|window| window == plaintext));

        let state = crate::SqliteState::open(&database).unwrap();
        let pending = state
            .approval_repository()
            .list_pending(Utc::now())
            .unwrap();
        assert_eq!(pending, vec![request]);
    }

    #[test]
    fn begin_execution_is_single_winner_across_connections() {
        let directory = tempdir().unwrap();
        let database = directory.path().join("state.db");
        let request = test_request();
        crate::SqliteState::open(&database)
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
                    let repository = crate::SqliteState::open(database)
                        .unwrap()
                        .approval_repository();
                    barrier.wait();
                    repository.begin_execution(approval_id, &format!("actor-{index}"), Utc::now())
                })
            })
            .collect::<Vec<_>>();
        barrier.wait();

        let outcomes = results
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| matches!(outcome, Err(ApprovalRepositoryError::AlreadyDecided)))
                .count(),
            1
        );
    }

    #[test]
    fn expired_denied_and_consumed_rows_erase_ciphertext() {
        let now = Utc::now();
        let state = crate::SqliteState::in_memory().unwrap();
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

        for approval_id in [
            denied.approval_id,
            consumed.approval_id,
            expired.approval_id,
        ] {
            assert_eq!(
                repository
                    .checkpoint_presence_for_test(approval_id)
                    .unwrap(),
                (false, false, false)
            );
        }
    }
}
