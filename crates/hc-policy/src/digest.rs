use hc_domain::{Provenance, RiskClass, SideEffectClass, ToolCall};
use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ActionDigest(String);

#[derive(Serialize)]
struct ActionDigestMaterial<'a> {
    schema_version: u8,
    call_id: &'a str,
    capability_id: &'a str,
    arguments: &'a serde_json::Value,
    risk: RiskClass,
    side_effect: SideEffectClass,
    provenance: &'a Provenance,
}

impl ActionDigest {
    pub fn for_call(call: &ToolCall) -> Result<Self, DigestError> {
        let material = ActionDigestMaterial {
            schema_version: 1,
            call_id: &call.id,
            capability_id: &call.capability_id,
            arguments: &call.arguments,
            risk: call.risk,
            side_effect: call.side_effect,
            provenance: &call.provenance,
        };
        let canonical = serde_jcs::to_vec(&material)
            .map_err(|error| DigestError::CanonicalJson(error.to_string()))?;
        Ok(Self(hex::encode(Sha256::digest(canonical))))
    }

    pub fn parse(value: impl Into<String>) -> Result<Self, DigestError> {
        let value = value.into();
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(DigestError::InvalidEncoding);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Error)]
pub enum DigestError {
    #[error("action digest serialization failed: {0}")]
    CanonicalJson(String),
    #[error("action digest must be 64 lowercase hexadecimal characters")]
    InvalidEncoding,
}

#[cfg(test)]
mod tests {
    use super::*;
    use hc_domain::{Provenance, RiskClass, SideEffectClass, ToolCall, TrustLevel};
    use serde_json::json;

    #[test]
    fn digest_is_stable_across_json_key_order() {
        let left = ToolCall::new(
            "call-1",
            "workspace.write",
            json!({"path": "notes.txt", "content": "hello", "mode": "create_new"}),
            RiskClass::Medium,
            SideEffectClass::Mutation,
            Provenance::new("model", TrustLevel::ModelGenerated),
        );
        let right = ToolCall::new(
            "call-1",
            "workspace.write",
            json!({"mode": "create_new", "content": "hello", "path": "notes.txt"}),
            RiskClass::Medium,
            SideEffectClass::Mutation,
            Provenance::new("model", TrustLevel::ModelGenerated),
        );

        assert_eq!(
            ActionDigest::for_call(&left).unwrap(),
            ActionDigest::for_call(&right).unwrap()
        );
    }

    #[test]
    fn any_action_change_changes_digest() {
        let original = ToolCall::workspace_write_create("call-1", "notes.txt", "hello");
        let changed_content =
            ToolCall::workspace_write_create("call-1", "notes.txt", "Hello");
        let changed_path =
            ToolCall::workspace_write_create("call-1", "other.txt", "hello");
        let changed_call =
            ToolCall::workspace_write_create("call-2", "notes.txt", "hello");

        let digest = ActionDigest::for_call(&original).unwrap();
        assert_ne!(
            digest,
            ActionDigest::for_call(&changed_content).unwrap()
        );
        assert_ne!(digest, ActionDigest::for_call(&changed_path).unwrap());
        assert_ne!(digest, ActionDigest::for_call(&changed_call).unwrap());
        assert_eq!(digest.as_str().len(), 64);
        assert_eq!(ActionDigest::parse(digest.as_str()).unwrap(), digest);
        assert!(ActionDigest::parse("ABC").is_err());
    }
}
