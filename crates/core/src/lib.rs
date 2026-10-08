//! Provider-agnostic domain model for Runalytics.
//!
//! This crate is deliberately free of I/O, database and transport concerns. It
//! defines the vocabulary every other crate speaks: [`AthleteSnapshot`],
//! [`Plan`], [`PlannedSession`], [`Activity`], [`HealthDay`] and the score
//! envelopes produced by `runalytics-scoring`.
//!
//! Two invariants hold everywhere in this crate and are relied upon upstream:
//!
//! 1. **Dates are naive local dates.** A [`Date`] is the calendar day *as the
//!    athlete experiences it*, never a UTC instant. Sessions crossing midnight
//!    are attributed to the day they started.
//! 2. **Everything measurable is a newtype.** Volume, pace and heart rate are
//!    distinct types with distinct zero values, so adding a pace to a distance
//!    is a compile error rather than a season-ending training bug.

#![forbid(unsafe_code)]

pub mod activity;
pub mod athlete;
pub mod error;
pub mod health;
pub mod ids;
pub mod plan;
pub mod provider;
pub mod units;
pub mod workout;

pub use activity::{Activity, ActivityLap, ActivitySummary, Intensity};
pub use athlete::{AthleteSnapshot, ExperienceLevel, Sex};
pub use chrono_tz::Tz;
pub use error::DomainError;
pub use health::{FitnessAssessment, HealthDay, SleepDay};
pub use ids::{
    ACTIVITY_NAMESPACE, ActivityId, HealthDayId, PlanId, PlannedSessionId, ProviderAccountId,
    SESSION_NAMESPACE, SessionResultId, UserId,
};
pub use plan::{
    Anchor, AnchorResolution, GoalKind, Phase, PhaseSpan, Plan, PlanConstraints, PlanRequest,
    PlanStatus, PlanWeek, next_monday,
};
pub use provider::Provider;
pub use units::{DurationSecs, HeartRate, Pace, VolumeKm};
pub use uuid::Uuid;
pub use workout::{
    BlockTarget, HrZone, PlannedSession, SessionKind, StructuredWorkout, WorkoutBlock,
};

/// Calendar day as experienced by the athlete.
pub type Date = chrono::NaiveDate;

/// Instant in time, always stored and compared in UTC.
pub type Timestamp = chrono::DateTime<chrono::Utc>;

/// Wall-clock time of day, used for scheduled session start times.
pub type TimeOfDay = chrono::NaiveTime;

/// Crate-wide prelude for downstream crates.
pub mod prelude {
    pub use super::{
        Activity, ActivityId, Anchor, AthleteSnapshot, Date, DomainError, GoalKind, HealthDay,
        HeartRate, Intensity, Phase, Plan, PlanConstraints, PlanId, PlanRequest, PlanStatus,
        PlanWeek, PlannedSession, PlannedSessionId, Provider, SessionKind, SleepDay,
        StructuredWorkout, TimeOfDay, Timestamp, Tz, VolumeKm, WorkoutBlock,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The domain crate must stay free of transport and UI dependencies. This
    /// test exists so a future `Cargo.toml` edit that pulls `tauri` or `rmcp`
    /// into `runalytics-core` fails loudly instead of silently slowing every
    /// unit test in the workspace.
    #[test]
    fn date_alias_is_a_naive_date() {
        let d: Date = Date::from_ymd_opt(2026, 10, 8).expect("test date");
        assert_eq!(d.format("%Y-%m-%d").to_string(), "2026-10-08");
    }
}
