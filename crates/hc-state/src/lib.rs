use chrono::{DateTime, Utc};
use hc_domain::{EvidenceRecord, MissionId, PolicyDecision, TraceId};
use rusqlite::{params, Connection};
use std::{path::Path, sync::Mutex};
use thiserror::Error;

pub struct EvidenceStore {
    connection: Mutex<Connection>,
}

impl EvidenceStore {
    pub fn in_memory() -> Result<Self, StateError> {
        let connection = Connection::open_in_memory()?;
        Self::from_connection(connection)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, StateError> {
        let connection = Connection::open(path)?;
        Self::from_connection(connection)
    }

    fn from_connection(connection: Connection) -> Result<Self, StateError> {
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS evidence (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                trace_id TEXT NOT NULL,
                mission_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                capability_id TEXT,
                policy_decision TEXT,
                status TEXT NOT NULL,
                payload TEXT NOT NULL,
                recorded_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_evidence_trace_id
                ON evidence(trace_id, id);",
        )?;

        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn append(&self, record: &EvidenceRecord) -> Result<(), StateError> {
        let trace_id = serde_json::to_string(&record.trace_id)?;
        let mission_id = serde_json::to_string(&record.mission_id)?;
        let policy_decision = record
            .policy_decision
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let payload = serde_json::to_string(&record.payload)?;
        let recorded_at = record.recorded_at.to_rfc3339();

        let connection = self
            .connection
            .lock()
            .map_err(|_| StateError::PoisonedConnection)?;
        connection.execute(
            "INSERT INTO evidence (
                trace_id, mission_id, kind, capability_id,
                policy_decision, status, payload, recorded_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                trace_id,
                mission_id,
                record.kind,
                record.capability_id,
                policy_decision,
                record.status,
                payload,
                recorded_at,
            ],
        )?;
        Ok(())
    }

    pub fn list_by_trace(&self, trace_id: TraceId) -> Result<Vec<EvidenceRecord>, StateError> {
        let encoded_trace_id = serde_json::to_string(&trace_id)?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| StateError::PoisonedConnection)?;
        let mut statement = connection.prepare(
            "SELECT mission_id, kind, capability_id, policy_decision,
                    status, payload, recorded_at
             FROM evidence
             WHERE trace_id = ?1
             ORDER BY id ASC",
        )?;

        let mut rows = statement.query(params![encoded_trace_id])?;
        let mut records = Vec::new();
        while let Some(row) = rows.next()? {
            let mission_id_json: String = row.get(0)?;
            let policy_json: Option<String> = row.get(3)?;
            let payload_json: String = row.get(5)?;
            let recorded_at: String = row.get(6)?;

            records.push(EvidenceRecord {
                trace_id,
                mission_id: serde_json::from_str::<MissionId>(&mission_id_json)?,
                kind: row.get(1)?,
                capability_id: row.get(2)?,
                policy_decision: policy_json
                    .as_deref()
                    .map(serde_json::from_str::<PolicyDecision>)
                    .transpose()?,
                status: row.get(4)?,
                payload: serde_json::from_str(&payload_json)?,
                recorded_at: DateTime::parse_from_rfc3339(&recorded_at)?.with_timezone(&Utc),
            });
        }

        Ok(records)
    }
}

#[derive(Debug, Error)]
pub enum StateError {
    #[error("SQLite state error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("state serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("invalid evidence timestamp: {0}")]
    Timestamp(#[from] chrono::ParseError),
    #[error("evidence connection lock poisoned")]
    PoisonedConnection,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use hc_domain::{EvidenceRecord, MissionId, PolicyDecision, TraceId};
    use serde_json::json;

    #[test]
    fn evidence_round_trips_through_sqlite_by_trace() {
        let store = EvidenceStore::in_memory().expect("open in-memory store");
        let trace_id = TraceId::new();
        let mission_id = MissionId::new();
        let record = EvidenceRecord {
            trace_id,
            mission_id,
            kind: "capability_execution".into(),
            capability_id: Some("workspace.list".into()),
            policy_decision: Some(PolicyDecision::Allow),
            status: "succeeded".into(),
            payload: json!({"entries": ["alpha.txt"]}),
            recorded_at: Utc::now(),
        };

        store.append(&record).expect("append evidence");
        let rows = store.list_by_trace(trace_id).expect("read evidence");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0], record);
    }
}
