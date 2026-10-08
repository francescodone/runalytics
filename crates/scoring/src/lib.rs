//! Load, readiness, injury-risk, performance and session-quality scoring.
//!
//! This crate is the analytical heart of Runalytics. It is **pure**: no
//! database, no clock, no I/O, no provider types beyond the domain model. Every
//! function is a deterministic mapping from data in to score out, which is what
//! makes the golden fixtures in `fixtures/` real regression tests rather than
//! smoke tests.
//!
//! ## Why the formulas are versioned
//!
//! [`SCORING_VERSION`] is written into every score row the app persists. It is
//! not metadata for its own sake — it is what makes the app honest over time.
//!
//! A score computed in March and displayed in September must be interpretable,
//! and formulas change. When a weight is retuned or a factor added, the stored
//! history is not wrong exactly, but it is no longer comparable to the present,
//! and a chart that silently splices two different formulas is a chart that
//! lies. Carrying the version per row lets the app recompute stale rows and lets
//! the UI mark a pre-retune segment as such.
//!
//! Bump it whenever a persisted number would change for unchanged input, in the
//! same commit as the change, and add a fixture that pins the new value.
//!
//! ## The five scores and how they relate
//!
//! | Score | Question it answers | Module |
//! |---|---|---|
//! | Load (TSS, CTL, ATL, ACWR) | How much work, and how fast did it grow? | [`load`] |
//! | Readiness | Should today's hard session happen today? | [`readiness`] |
//! | Injury risk | Is this load trajectory safe? | [`injury`] |
//! | Performance | Is the athlete getting faster? | [`performance`] |
//! | Session quality | Did that session do its job? | [`quality`] |
//!
//! They are deliberately not collapsed into one number. Fitness, freshness and
//! durability are different axes, and an athlete handed a single "score" cannot
//! tell whether to train harder or sleep more.
//!
//! ## Shared arithmetic
//!
//! The exponential moving average and the safe ACWR band come from
//! `runalytics-plan-engine` rather than being reimplemented here. The plan
//! *projects* an ACWR and this crate *measures* one; if the two used different
//! smoothing, the athlete would see the same week scored two ways and the
//! dashboard would contradict the plan that produced it.

#![forbid(unsafe_code)]

pub mod injury;
pub mod load;
pub mod performance;
pub mod quality;
pub mod readiness;

pub use injury::{
    InjuryBand, InjuryInputs, InjuryRisk, RiskDriver, TRAINING_DAY_TSS, consecutive_day_risk,
    consecutive_training_days, detraining_risk, injury_risk, injury_risk_for, squash,
};
pub use load::{
    ACUTE_WINDOW_DAYS, CHRONIC_WINDOW_DAYS, DailyLoad, MONOTONY_CAP, activity_load,
    acute_chronic_ratio, acute_load, chronic_load, daily_loads, intensity_factor,
    intensity_from_hr, intensity_from_pace, intensity_from_rpe, load_from_rpe, monotony,
    training_stress_balance, training_stress_score, weekly_spike, weekly_totals,
};
pub use performance::{
    Effort, PerformancePoint, PerformanceSignals, RaceResult, best_effort_pace, efforts_from,
    latest_race, percent_change, performance_index, performance_series, performance_trend,
    predicted_finish, predicted_race_pace, race_is_current, resolve_threshold,
    threshold_from_race_pace, threshold_from_vo2max, vo2max_from_threshold, weeks_since_race,
};
pub use quality::{
    MIN_INTENSITY_SHARE_FOR_PEAK, PACE_TOLERANCE, QualityComponents, STRUCTURE_TOLERANCE,
    SessionFeedback, SessionQuality, ZONE_SHARE_TOLERANCE, ZONE_TOLERANCE, decoupling,
    decoupling_penalty, effort_adherence, executed_quality, hard_time_share, intensity_lap_pace,
    pace_adherence, peak_intensity_factor, planned_tss, proximity, quality_score, score_session,
    structure_adherence, subjective_score, verdict, zone_adherence,
};
pub use readiness::{
    BASELINE_WINDOW_DAYS, Baseline, Readiness, ReadinessBand, ReadinessComponents, baseline_window,
    day_is_scoreable, drift_is_concerning, readiness_for, readiness_score, readiness_with_drift,
    resting_hr_drift,
};

/// The version of every formula in this crate.
///
/// Persisted alongside every score. See the module documentation for why.
///
/// History:
/// * `1` — initial: Coggan TSS from pace/HR/RPE, 42/7-day EMA load model,
///   z-score readiness with renormalised weights, logistic injury score with
///   attributable drivers, critical-speed performance fit, structural session
///   adherence with a cardiac-decoupling penalty.
pub const SCORING_VERSION: i64 = 1;

/// The safe acute:chronic band, re-exported so the UI reads it from one place
/// rather than reaching into the plan engine.
pub use runalytics_plan_engine::{ACWR_TARGET_HIGH, ACWR_TARGET_LOW};

/// Compile-time guard: the ACWR band must be ordered.
///
/// A reversed band would invert every "are you in the safe zone" answer in the
/// app and would do so silently, so this is checked by the compiler rather than
/// by a test that has to be run to fail.
const _: () = assert!(ACWR_TARGET_LOW < ACWR_TARGET_HIGH, "ACWR band is inverted");
const _: () = assert!(ACWR_TARGET_LOW > 0.0, "ACWR band floor must be positive");
const _: () = assert!(SCORING_VERSION > 0, "formula version must be positive");

/// A computed score carrying the version of the formula that produced it.
///
/// The store's rows all take `formula_version` explicitly; this exists so a
/// caller cannot write a row with a stale version by forgetting to stamp it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scored<T> {
    pub value: T,
    pub formula_version: i64,
}

impl<T> Scored<T> {
    /// Stamp a value with the current formula version.
    #[must_use]
    pub fn current(value: T) -> Self {
        Self {
            value,
            formula_version: SCORING_VERSION,
        }
    }

    /// Whether a stored version still matches the live formulas.
    #[must_use]
    pub fn is_stale(version: i64) -> bool {
        version != SCORING_VERSION
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runalytics_core::Pace;

    #[test]
    fn version_is_stable() {
        assert!(!Scored::<()>::is_stale(SCORING_VERSION));
        assert!(Scored::<()>::is_stale(SCORING_VERSION - 1));
        assert!(Scored::<()>::is_stale(SCORING_VERSION + 1));
    }

    #[test]
    fn current_stamps_the_live_version() {
        let scored = Scored::current(42.0);
        assert_eq!(scored.formula_version, SCORING_VERSION);
        assert_eq!(scored.value, 42.0);
    }

    /// Every module must be reachable from the root, and the load model must
    /// not depend on readiness — if it did, the dashboard would be reasoning in
    /// a circle.
    #[test]
    fn modules_are_reachable_from_the_root() {
        let series = vec![
            DailyLoad {
                date: chrono::NaiveDate::from_ymd_opt(2026, 10, 1).expect("d"),
                load: 50.0,
            },
            DailyLoad {
                date: chrono::NaiveDate::from_ymd_opt(2026, 10, 2).expect("d"),
                load: 60.0,
            },
        ];
        assert!(chronic_load(&series) > 0.0);
        assert_eq!(ReadinessBand::from_score(70.0), ReadinessBand::Ready);
        assert_eq!(InjuryBand::from_score(70.0), InjuryBand::Elevated);
        assert!(performance_index(Pace::new(300.0)) > 0.0);
        assert_eq!(proximity(10.0, 10.0, 0.1), 1.0);
    }
}
