//! Agent handoff validation.
//!
//! Handoffs between agents use typed protobuf messages.
//! This module validates them before delivery to prevent
//! the specification-gap failures identified in MAST research.

use thiserror::Error;

/// Errors that can occur during handoff validation.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum HandoffError {
    #[error("missing source agent")]
    MissingSourceAgent,

    #[error("missing target agent")]
    MissingTargetAgent,

    #[error("source and target agent are the same: {0}")]
    SelfHandoff(String),

    #[error("missing task description")]
    MissingTaskDescription,

    #[error("missing user intent")]
    MissingUserIntent,

    #[error("target agent '{0}' is not registered")]
    UnknownTargetAgent(String),

    #[error("completion status indicates failure but no distortion flags provided")]
    FailureWithoutFlags,
}

/// Validates agent handoff messages.
pub struct HandoffValidator {
    known_agents: Vec<String>,
}

impl HandoffValidator {
    /// Create a validator with a list of known agent names.
    #[must_use]
    pub fn new(known_agents: Vec<String>) -> Self {
        Self { known_agents }
    }

    /// Validate a protobuf `AgentHandoff` message.
    pub fn validate(&self, handoff: &crate::proto::AgentHandoff) -> Result<(), Vec<HandoffError>> {
        let mut errors = Vec::new();

        if handoff.source_agent.is_empty() {
            errors.push(HandoffError::MissingSourceAgent);
        }

        if handoff.target_agent.is_empty() {
            errors.push(HandoffError::MissingTargetAgent);
        }

        if !handoff.source_agent.is_empty()
            && !handoff.target_agent.is_empty()
            && handoff.source_agent == handoff.target_agent
        {
            errors.push(HandoffError::SelfHandoff(handoff.source_agent.clone()));
        }

        if handoff.task_description.is_empty() {
            errors.push(HandoffError::MissingTaskDescription);
        }

        if handoff.user_intent.is_empty() {
            errors.push(HandoffError::MissingUserIntent);
        }

        if !handoff.target_agent.is_empty()
            && !self.known_agents.contains(&handoff.target_agent)
        {
            errors.push(HandoffError::UnknownTargetAgent(
                handoff.target_agent.clone(),
            ));
        }

        // FAILED completion without distortion flags is suspicious
        if handoff.source_completion == (crate::proto::CompletionStatus::Failed as i32)
            && handoff.distortion_flags.is_empty()
        {
            errors.push(HandoffError::FailureWithoutFlags);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::{AgentHandoff, CompletionStatus};

    fn validator() -> HandoffValidator {
        HandoffValidator::new(vec![
            "general".into(),
            "music".into(),
            "investment".into(),
            "development".into(),
            "framework".into(),
        ])
    }

    fn valid_handoff() -> AgentHandoff {
        AgentHandoff {
            source_agent: "music".into(),
            target_agent: "development".into(),
            task_description: "Build a plugin for the DAW".into(),
            structured_data: std::collections::HashMap::new(),
            context_keys: vec![],
            user_intent: "I need a VST plugin that does X".into(),
            constraints: vec!["Keep the existing MIDI mapping".into()],
            source_completion: CompletionStatus::HandedOff as i32,
            distortion_flags: vec![],
        }
    }

    #[test]
    fn accepts_valid_handoff() {
        let v = validator();
        assert!(v.validate(&valid_handoff()).is_ok());
    }

    #[test]
    fn rejects_missing_source() {
        let v = validator();
        let mut h = valid_handoff();
        h.source_agent = String::new();
        let errs = v.validate(&h).unwrap_err();
        assert!(errs.contains(&HandoffError::MissingSourceAgent));
    }

    #[test]
    fn rejects_missing_target() {
        let v = validator();
        let mut h = valid_handoff();
        h.target_agent = String::new();
        let errs = v.validate(&h).unwrap_err();
        assert!(errs.contains(&HandoffError::MissingTargetAgent));
    }

    #[test]
    fn rejects_self_handoff() {
        let v = validator();
        let mut h = valid_handoff();
        h.target_agent = "music".into();
        h.source_agent = "music".into();
        let errs = v.validate(&h).unwrap_err();
        assert!(errs.iter().any(|e| matches!(e, HandoffError::SelfHandoff(_))));
    }

    #[test]
    fn rejects_unknown_target_agent() {
        let v = validator();
        let mut h = valid_handoff();
        h.target_agent = "nonexistent".into();
        let errs = v.validate(&h).unwrap_err();
        assert!(errs
            .iter()
            .any(|e| matches!(e, HandoffError::UnknownTargetAgent(_))));
    }

    #[test]
    fn rejects_missing_task_description() {
        let v = validator();
        let mut h = valid_handoff();
        h.task_description = String::new();
        let errs = v.validate(&h).unwrap_err();
        assert!(errs.contains(&HandoffError::MissingTaskDescription));
    }

    #[test]
    fn rejects_missing_user_intent() {
        let v = validator();
        let mut h = valid_handoff();
        h.user_intent = String::new();
        let errs = v.validate(&h).unwrap_err();
        assert!(errs.contains(&HandoffError::MissingUserIntent));
    }

    #[test]
    fn rejects_failure_without_distortion_flags() {
        let v = validator();
        let mut h = valid_handoff();
        h.source_completion = CompletionStatus::Failed as i32;
        h.distortion_flags = vec![];
        let errs = v.validate(&h).unwrap_err();
        assert!(errs.contains(&HandoffError::FailureWithoutFlags));
    }

    #[test]
    fn allows_failure_with_distortion_flags() {
        let v = validator();
        let mut h = valid_handoff();
        h.source_completion = CompletionStatus::Failed as i32;
        h.distortion_flags = vec!["stuck_loop".into()];
        assert!(v.validate(&h).is_ok());
    }

    #[test]
    fn collects_multiple_errors() {
        let v = validator();
        let h = AgentHandoff {
            source_agent: String::new(),
            target_agent: String::new(),
            task_description: String::new(),
            structured_data: std::collections::HashMap::new(),
            context_keys: vec![],
            user_intent: String::new(),
            constraints: vec![],
            source_completion: 0,
            distortion_flags: vec![],
        };
        let errs = v.validate(&h).unwrap_err();
        assert!(errs.len() >= 4);
    }
}
