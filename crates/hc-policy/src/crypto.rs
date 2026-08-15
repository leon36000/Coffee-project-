use crate::{ActionDigest, ApprovalKeyError, ApprovalKeyProvider};
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng, Payload},
    Key, XChaCha20Poly1305, XNonce,
};
use hc_domain::{ApprovalId, MissionId, TraceId};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedCheckpoint {
    pub ciphertext: Vec<u8>,
    pub nonce: [u8; 24],
    pub key_id: String,
}

#[derive(Clone, Debug)]
pub struct CheckpointContext {
    pub approval_id: ApprovalId,
    pub trace_id: TraceId,
    pub mission_id: MissionId,
    pub action_digest: ActionDigest,
    pub schema_version: u8,
}

#[derive(Clone)]
pub struct CheckpointCipher {
    keys: Arc<dyn ApprovalKeyProvider>,
}

impl CheckpointCipher {
    pub fn new(keys: Arc<dyn ApprovalKeyProvider>) -> Self {
        Self { keys }
    }

    pub fn seal(
        &self,
        context: &CheckpointContext,
        plaintext: &[u8],
    ) -> Result<EncryptedCheckpoint, ApprovalCryptoError> {
        let key = self.keys.active_key()?;
        let cipher = XChaCha20Poly1305::new(Key::from_slice(key.bytes()));
        let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
        let associated_data = canonical_associated_data(context)?;
        let ciphertext = cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: plaintext,
                    aad: &associated_data,
                },
            )
            .map_err(|_| ApprovalCryptoError::EncryptionFailed)?;
        let mut nonce_bytes = [0_u8; 24];
        nonce_bytes.copy_from_slice(&nonce);

        Ok(EncryptedCheckpoint {
            ciphertext,
            nonce: nonce_bytes,
            key_id: key.id().to_owned(),
        })
    }

    pub fn open(
        &self,
        context: &CheckpointContext,
        sealed: &EncryptedCheckpoint,
    ) -> Result<Vec<u8>, ApprovalCryptoError> {
        let key = self.keys.key_by_id(&sealed.key_id)?;
        let cipher = XChaCha20Poly1305::new(Key::from_slice(key.bytes()));
        let associated_data = canonical_associated_data(context)?;
        cipher
            .decrypt(
                XNonce::from_slice(&sealed.nonce),
                Payload {
                    msg: &sealed.ciphertext,
                    aad: &associated_data,
                },
            )
            .map_err(|_| ApprovalCryptoError::AuthenticationFailed)
    }
}

#[derive(Serialize)]
struct AssociatedData<'a> {
    approval_id: String,
    trace_id: String,
    mission_id: String,
    action_digest: &'a str,
    schema_version: u8,
}

fn canonical_associated_data(context: &CheckpointContext) -> Result<Vec<u8>, ApprovalCryptoError> {
    let data = AssociatedData {
        approval_id: context.approval_id.to_string(),
        trace_id: context.trace_id.to_string(),
        mission_id: context.mission_id.to_string(),
        action_digest: context.action_digest.as_str(),
        schema_version: context.schema_version,
    };
    serde_jcs::to_vec(&data).map_err(|error| ApprovalCryptoError::AssociatedData(error.to_string()))
}

#[derive(Debug, Error)]
pub enum ApprovalCryptoError {
    #[error(transparent)]
    Key(#[from] ApprovalKeyError),
    #[error("approval checkpoint associated data serialization failed: {0}")]
    AssociatedData(String),
    #[error("approval checkpoint encryption failed")]
    EncryptionFailed,
    #[error("approval checkpoint authentication failed")]
    AuthenticationFailed,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ActionDigest, InMemoryApprovalKeyProvider};
    use hc_domain::{ApprovalId, MissionId, ToolCall, TraceId};
    use std::sync::Arc;

    fn test_context() -> CheckpointContext {
        let call = ToolCall::workspace_write_create("call-1", "notes.txt", "secret");
        CheckpointContext {
            approval_id: ApprovalId::new(),
            trace_id: TraceId::new(),
            mission_id: MissionId::new(),
            action_digest: ActionDigest::for_call(&call).unwrap(),
            schema_version: 1,
        }
    }

    #[test]
    fn encrypted_checkpoint_round_trips_and_hides_plaintext() {
        let provider = InMemoryApprovalKeyProvider::new("test-key", [7_u8; 32]);
        let cipher = CheckpointCipher::new(Arc::new(provider));
        let context = test_context();
        let plaintext = b"sentinel pending file content";

        let sealed = cipher.seal(&context, plaintext).unwrap();

        assert!(!sealed
            .ciphertext
            .windows(plaintext.len())
            .any(|window| window == plaintext));
        assert_eq!(cipher.open(&context, &sealed).unwrap(), plaintext);
    }

    #[test]
    fn wrong_associated_data_or_key_fails_closed() {
        let cipher = CheckpointCipher::new(Arc::new(InMemoryApprovalKeyProvider::new(
            "key-a", [1_u8; 32],
        )));
        let context = test_context();
        let sealed = cipher.seal(&context, b"secret").unwrap();

        let mut changed = context.clone();
        changed.action_digest = ActionDigest::for_call(&ToolCall::workspace_write_create(
            "changed",
            "notes.txt",
            "secret",
        ))
        .unwrap();
        assert!(matches!(
            cipher.open(&changed, &sealed),
            Err(ApprovalCryptoError::AuthenticationFailed)
        ));

        let wrong_key = CheckpointCipher::new(Arc::new(InMemoryApprovalKeyProvider::new(
            "key-a", [2_u8; 32],
        )));
        assert!(matches!(
            wrong_key.open(&context, &sealed),
            Err(ApprovalCryptoError::AuthenticationFailed)
        ));
    }

    #[test]
    fn ciphertext_cannot_be_swapped_between_approval_rows() {
        let cipher = CheckpointCipher::new(Arc::new(InMemoryApprovalKeyProvider::new(
            "key-a", [3_u8; 32],
        )));
        let first = test_context();
        let mut second = first.clone();
        second.approval_id = ApprovalId::new();
        let sealed = cipher.seal(&first, b"secret").unwrap();

        assert!(matches!(
            cipher.open(&second, &sealed),
            Err(ApprovalCryptoError::AuthenticationFailed)
        ));
    }
}
