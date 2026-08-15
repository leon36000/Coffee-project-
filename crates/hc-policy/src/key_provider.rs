use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::env;
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct ApprovalKey {
    id: String,
    bytes: [u8; 32],
}

impl ApprovalKey {
    pub fn new(id: impl Into<String>, bytes: [u8; 32]) -> Self {
        Self {
            id: id.into(),
            bytes,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn bytes(&self) -> &[u8; 32] {
        &self.bytes
    }
}

pub trait ApprovalKeyProvider: Send + Sync {
    fn active_key(&self) -> Result<ApprovalKey, ApprovalKeyError>;
    fn key_by_id(&self, key_id: &str) -> Result<ApprovalKey, ApprovalKeyError>;
}

#[derive(Clone)]
pub struct InMemoryApprovalKeyProvider {
    key: ApprovalKey,
}

impl InMemoryApprovalKeyProvider {
    pub fn new(key_id: impl Into<String>, bytes: [u8; 32]) -> Self {
        Self {
            key: ApprovalKey::new(key_id, bytes),
        }
    }
}

impl ApprovalKeyProvider for InMemoryApprovalKeyProvider {
    fn active_key(&self) -> Result<ApprovalKey, ApprovalKeyError> {
        Ok(self.key.clone())
    }

    fn key_by_id(&self, key_id: &str) -> Result<ApprovalKey, ApprovalKeyError> {
        if self.key.id() == key_id {
            Ok(self.key.clone())
        } else {
            Err(ApprovalKeyError::UnknownKey(key_id.to_owned()))
        }
    }
}

#[derive(Clone)]
pub struct EnvApprovalKeyProvider {
    key: ApprovalKey,
}

impl EnvApprovalKeyProvider {
    pub fn from_env() -> Result<Self, ApprovalKeyError> {
        let value = env::var("HERMESCLAW_APPROVAL_KEY").map_err(|error| match error {
            env::VarError::NotPresent => ApprovalKeyError::Missing,
            env::VarError::NotUnicode(_) => ApprovalKeyError::InvalidEncoding,
        })?;
        Self::from_encoded("env-v1", &value)
    }

    pub fn from_encoded(
        key_id: impl Into<String>,
        encoded: &str,
    ) -> Result<Self, ApprovalKeyError> {
        let decoded = STANDARD
            .decode(encoded)
            .map_err(|_| ApprovalKeyError::InvalidEncoding)?;
        if decoded.len() != 32 {
            return Err(ApprovalKeyError::InvalidLength {
                actual: decoded.len(),
            });
        }

        let mut bytes = [0_u8; 32];
        bytes.copy_from_slice(&decoded);
        Ok(Self {
            key: ApprovalKey::new(key_id, bytes),
        })
    }
}

impl ApprovalKeyProvider for EnvApprovalKeyProvider {
    fn active_key(&self) -> Result<ApprovalKey, ApprovalKeyError> {
        Ok(self.key.clone())
    }

    fn key_by_id(&self, key_id: &str) -> Result<ApprovalKey, ApprovalKeyError> {
        if self.key.id() == key_id {
            Ok(self.key.clone())
        } else {
            Err(ApprovalKeyError::UnknownKey(key_id.to_owned()))
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ApprovalKeyError {
    #[error("approval key is unavailable")]
    Missing,
    #[error("approval key is not valid base64")]
    InvalidEncoding,
    #[error("approval key must decode to 32 bytes, got {actual}")]
    InvalidLength { actual: usize },
    #[error("approval key identifier is unknown: {0}")]
    UnknownKey(String),
    #[error("approval key backend error: {0}")]
    Backend(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_KEY: &str = "CQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQk=";
    const SHORT_KEY: &str = "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQ==";

    #[test]
    fn env_provider_accepts_exactly_32_decoded_bytes() {
        let provider = EnvApprovalKeyProvider::from_encoded("env-v1", VALID_KEY).unwrap();
        assert_eq!(provider.active_key().unwrap().id(), "env-v1");
    }

    #[test]
    fn env_provider_rejects_wrong_length_and_invalid_base64() {
        assert!(matches!(
            EnvApprovalKeyProvider::from_encoded("env-v1", "%%%"),
            Err(ApprovalKeyError::InvalidEncoding)
        ));
        assert!(matches!(
            EnvApprovalKeyProvider::from_encoded("env-v1", SHORT_KEY),
            Err(ApprovalKeyError::InvalidLength { actual: 31 })
        ));
    }
}
