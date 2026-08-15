use hc_domain::{AutonomyProfile, PolicyDecision, RiskClass, SideEffectClass, ToolCall};

mod approval_token;
mod digest;

pub use approval_token::{PolicyContext, VerifiedApproval};
pub use digest::{ActionDigest, DigestError};

pub struct PolicyKernel;

impl PolicyKernel {
    pub fn evaluate(profile: AutonomyProfile, call: &ToolCall) -> PolicyDecision {
        Self::evaluate_with_context(PolicyContext::new(profile), call)
    }

    pub fn evaluate_with_context(context: PolicyContext<'_>, call: &ToolCall) -> PolicyDecision {
        if call.capability_id == "workspace.write" {
            return Self::evaluate_workspace_write(context, call);
        }

        match context.profile {
            AutonomyProfile::Observe => {
                if call.side_effect != SideEffectClass::None {
                    return PolicyDecision::Deny(
                        "observe profile forbids side effects".to_string(),
                    );
                }
                if call.risk != RiskClass::Low {
                    return PolicyDecision::Deny(
                        "observe profile permits only low-risk capabilities".to_string(),
                    );
                }
                PolicyDecision::Allow
            }
            AutonomyProfile::Assist => match (call.risk, call.side_effect) {
                (RiskClass::Low, SideEffectClass::None) => PolicyDecision::Allow,
                _ => PolicyDecision::RequiresApproval(
                    "assist profile requires approval for consequential actions".to_string(),
                ),
            },
            AutonomyProfile::AutonomousScoped => {
                if call.risk == RiskClass::Critical
                    || call.side_effect == SideEffectClass::Destructive
                {
                    PolicyDecision::RequiresApproval(
                        "critical or destructive actions require explicit approval".to_string(),
                    )
                } else {
                    PolicyDecision::Allow
                }
            }
        }
    }

    fn evaluate_workspace_write(context: PolicyContext<'_>, call: &ToolCall) -> PolicyDecision {
        if context.profile == AutonomyProfile::Observe {
            return PolicyDecision::Deny("observe profile forbids side effects".into());
        }

        let Ok(digest) = ActionDigest::for_call(call) else {
            return PolicyDecision::Deny("unable to bind approval to action".into());
        };

        if context
            .verified_approval()
            .is_some_and(|approval| approval.matches(&digest))
        {
            return PolicyDecision::Allow;
        }

        PolicyDecision::RequiresApproval(
            "assist profile requires approval for consequential actions".into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use hc_domain::{
        ApprovalId, AutonomyProfile, PolicyDecision, Provenance, RiskClass, SideEffectClass,
        ToolCall, TrustLevel,
    };

    use super::*;

    #[test]
    fn observe_allows_low_risk_read_only_tool() {
        let call = ToolCall::workspace_list("call-1", ".");
        assert_eq!(
            PolicyKernel::evaluate(AutonomyProfile::Observe, &call),
            PolicyDecision::Allow
        );
    }

    #[test]
    fn assist_requires_approval_for_workspace_write() {
        let call = ToolCall::workspace_write_create("call-1", "notes.txt", "hello");
        let decision = PolicyKernel::evaluate(AutonomyProfile::Assist, &call);
        assert_eq!(
            decision,
            PolicyDecision::RequiresApproval(
                "assist profile requires approval for consequential actions".into()
            )
        );
    }

    #[test]
    fn matching_verified_approval_allows_exact_write() {
        let call = ToolCall::workspace_write_create("call-1", "notes.txt", "hello");
        let digest = ActionDigest::for_call(&call).unwrap();
        let approval = VerifiedApproval::for_test(ApprovalId::new(), digest);
        let context = PolicyContext::new(AutonomyProfile::Assist).with_approval(&approval);

        assert_eq!(
            PolicyKernel::evaluate_with_context(context, &call),
            PolicyDecision::Allow
        );
    }

    #[test]
    fn verified_approval_does_not_authorize_changed_call() {
        let approved = ToolCall::workspace_write_create("call-1", "notes.txt", "hello");
        let changed = ToolCall::workspace_write_create("call-1", "notes.txt", "changed");
        let approval = VerifiedApproval::for_test(
            ApprovalId::new(),
            ActionDigest::for_call(&approved).unwrap(),
        );
        let context = PolicyContext::new(AutonomyProfile::Assist).with_approval(&approval);

        assert!(matches!(
            PolicyKernel::evaluate_with_context(context, &changed),
            PolicyDecision::RequiresApproval(_)
        ));
    }

    #[test]
    fn autonomous_scoped_write_never_auto_allows_but_accepts_exact_human_approval() {
        let call = ToolCall::workspace_write_create("call-1", "notes.txt", "hello");
        assert!(matches!(
            PolicyKernel::evaluate(AutonomyProfile::AutonomousScoped, &call),
            PolicyDecision::RequiresApproval(_)
        ));

        let approval =
            VerifiedApproval::for_test(ApprovalId::new(), ActionDigest::for_call(&call).unwrap());
        let context =
            PolicyContext::new(AutonomyProfile::AutonomousScoped).with_approval(&approval);
        assert_eq!(
            PolicyKernel::evaluate_with_context(context, &call),
            PolicyDecision::Allow
        );
    }

    #[test]
    fn observe_denies_mutating_tool() {
        let call = ToolCall::new(
            "call-2",
            "workspace.write",
            serde_json::json!({"path": "notes.txt", "content": "no"}),
            RiskClass::Medium,
            SideEffectClass::Mutation,
            Provenance::new("model", TrustLevel::ModelGenerated),
        );
        assert_eq!(
            PolicyKernel::evaluate(AutonomyProfile::Observe, &call),
            PolicyDecision::Deny("observe profile forbids side effects".into())
        );
    }
}
