use hc_domain::{MissionId, MissionState};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mission {
    id: MissionId,
    objective: String,
    state: MissionState,
}

impl Mission {
    pub fn new(objective: impl Into<String>) -> Self {
        Self {
            id: MissionId::new(),
            objective: objective.into(),
            state: MissionState::Created,
        }
    }

    pub fn restore(
        id: MissionId,
        objective: impl Into<String>,
        state: MissionState,
    ) -> Result<Self, MissionError> {
        let objective = objective.into();
        if objective.trim().is_empty() {
            return Err(MissionError::EmptyObjective);
        }
        Ok(Self {
            id,
            objective,
            state,
        })
    }

    pub fn id(&self) -> MissionId {
        self.id
    }

    pub fn objective(&self) -> &str {
        &self.objective
    }

    pub fn state(&self) -> MissionState {
        self.state
    }

    pub fn transition(&mut self, next: MissionState) -> Result<(), MissionError> {
        if is_valid_transition(self.state, next) {
            self.state = next;
            Ok(())
        } else {
            Err(MissionError::InvalidTransition {
                from: state_name(self.state),
                to: state_name(next),
            })
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MissionError {
    #[error("mission objective must not be empty")]
    EmptyObjective,
    #[error("invalid mission transition: {from} -> {to}")]
    InvalidTransition {
        from: &'static str,
        to: &'static str,
    },
}

fn is_valid_transition(from: MissionState, to: MissionState) -> bool {
    use MissionState::*;

    matches!(
        (from, to),
        (Created, Planning)
            | (Created, Executing)
            | (Created, Cancelled)
            | (Planning, Executing)
            | (Planning, Failed)
            | (Planning, Cancelled)
            | (Executing, WaitingApproval)
            | (Executing, WaitingExternal)
            | (Executing, Verifying)
            | (Executing, Failed)
            | (Executing, Cancelled)
            | (WaitingApproval, Executing)
            | (WaitingApproval, Failed)
            | (WaitingApproval, Cancelled)
            | (WaitingExternal, Executing)
            | (WaitingExternal, Failed)
            | (WaitingExternal, Cancelled)
            | (Verifying, Completed)
            | (Verifying, Failed)
            | (Verifying, Cancelled)
    )
}

fn state_name(state: MissionState) -> &'static str {
    use MissionState::*;

    match state {
        Created => "created",
        Planning => "planning",
        Executing => "executing",
        WaitingApproval => "waiting_approval",
        WaitingExternal => "waiting_external",
        Verifying => "verifying",
        Completed => "completed",
        Failed => "failed",
        Cancelled => "cancelled",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mission_restores_waiting_approval_with_original_identity() {
        let id = MissionId::new();
        let mission = Mission::restore(id, "Create notes.txt", MissionState::WaitingApproval)
            .expect("restore mission");

        assert_eq!(mission.id(), id);
        assert_eq!(mission.objective(), "Create notes.txt");
        assert_eq!(mission.state(), MissionState::WaitingApproval);
    }

    #[test]
    fn mission_restore_rejects_empty_objective() {
        let error =
            Mission::restore(MissionId::new(), "", MissionState::WaitingApproval).unwrap_err();
        assert_eq!(error.to_string(), "mission objective must not be empty");
    }

    #[test]
    fn mission_accepts_happy_path_transitions() {
        let mut mission = Mission::new("List the workspace");
        mission
            .transition(MissionState::Executing)
            .expect("created -> executing");
        mission
            .transition(MissionState::Verifying)
            .expect("executing -> verifying");
        mission
            .transition(MissionState::Completed)
            .expect("verifying -> completed");
        assert_eq!(mission.state(), MissionState::Completed);
    }

    #[test]
    fn completed_mission_cannot_resume_execution() {
        let mut mission = Mission::new("List the workspace");
        mission.transition(MissionState::Executing).unwrap();
        mission.transition(MissionState::Verifying).unwrap();
        mission.transition(MissionState::Completed).unwrap();
        let error = mission
            .transition(MissionState::Executing)
            .expect_err("terminal state");
        assert_eq!(
            error.to_string(),
            "invalid mission transition: completed -> executing"
        );
    }
}
