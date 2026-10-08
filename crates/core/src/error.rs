//! Errors shared by the pure domain crates.

use thiserror::Error;

/// Domain-level failures that every layer can surface.
///
/// These are *validation* and *logic* errors, not I/O errors: transport and
/// database failures get their own error types in the crates that own them and
/// convert into these only when they represent a domain problem.
#[derive(Debug, Error)]
pub enum DomainError {
    /// The request cannot produce a plan at all.
    #[error("cannot build a plan: {0}")]
    UnbuildablePlan(String),

    /// A horizon outside the supported window (1-10 weeks).
    #[error("horizon of {weeks} weeks is outside the supported range ({min}={max})")]
    HorizonOutOfRange { weeks: u32, min: u32, max: u32 },

    /// The race date is in the past or too soon to train for.
    #[error("race date {0} cannot be trained for")]
    RaceDateUnusable(String),

    /// A plan is in a state that forbids the requested mutation.
    #[error("plan {plan} is {state} and cannot accept this change")]
    InvalidPlanState { plan: String, state: String },

    /// A session reference could not be resolved inside a plan.
    #[error("session {0} was not found in this plan")]
    UnknownSession(String),

    /// The athlete profile is missing a value the model needs.
    #[error("athlete profile is missing {0}")]
    IncompleteAthlete(&'static str),

    /// Two inputs are mutually exclusive.
    #[error("conflicting inputs: {0}")]
    Conflict(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_read_as_user_facing_copy() {
        let err = DomainError::HorizonOutOfRange {
            weeks: 14,
            min: 1,
            max: 10,
        };
        assert_eq!(
            err.to_string(),
            "horizon of 14 weeks is outside the supported range (1=10)"
        );
    }
}
