use hc_domain::{AutonomyProfile, PolicyDecision, RiskClass, SideEffectClass, ToolCall};

pub struct PolicyKernel;

impl PolicyKernel {
    pub fn evaluate(profile: AutonomyProfile, call: &ToolCall) -> PolicyDecision {
        match profile {
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
}

#[cfg(test)]
mod tests {
    use hc_domain::{
        AutonomyProfile, PolicyDecision, Provenance, RiskClass, SideEffectClass, ToolCall,
        TrustLevel,
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
