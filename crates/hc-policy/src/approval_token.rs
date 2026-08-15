use crate::ActionDigest;
use hc_domain::{ApprovalId, AutonomyProfile};

#[derive(Clone, Debug)]
pub struct VerifiedApproval {
    approval_id: ApprovalId,
    action_digest: ActionDigest,
}

impl VerifiedApproval {
    pub fn approval_id(&self) -> ApprovalId {
        self.approval_id
    }

    pub(crate) fn new(approval_id: ApprovalId, action_digest: ActionDigest) -> Self {
        Self {
            approval_id,
            action_digest,
        }
    }

    pub(crate) fn matches(&self, digest: &ActionDigest) -> bool {
        &self.action_digest == digest
    }

    #[cfg(test)]
    pub(crate) fn for_test(approval_id: ApprovalId, action_digest: ActionDigest) -> Self {
        Self::new(approval_id, action_digest)
    }
}

pub struct PolicyContext<'a> {
    pub profile: AutonomyProfile,
    verified_approval: Option<&'a VerifiedApproval>,
}

impl<'a> PolicyContext<'a> {
    pub fn new(profile: AutonomyProfile) -> Self {
        Self {
            profile,
            verified_approval: None,
        }
    }

    pub fn with_approval(mut self, approval: &'a VerifiedApproval) -> Self {
        self.verified_approval = Some(approval);
        self
    }

    pub(crate) fn verified_approval(&self) -> Option<&'a VerifiedApproval> {
        self.verified_approval
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ActionDigest;
    use hc_domain::ToolCall;

    #[test]
    fn verified_approval_exposes_only_identity_to_callers() {
        let approval_id = ApprovalId::new();
        let call = ToolCall::workspace_write_create("call-1", "notes.txt", "hello");
        let digest = ActionDigest::for_call(&call).unwrap();
        let approval = VerifiedApproval::for_test(approval_id, digest.clone());

        assert_eq!(approval.approval_id(), approval_id);
        assert!(approval.matches(&digest));
    }
}
