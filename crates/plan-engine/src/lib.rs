//! Deterministic training-plan generation.
//!
//! Pure: no I/O, no clock, no database. Given the same
//! [`PlanRequest`](runalytics_core::PlanRequest) and the same `today`,
//! [`generate`] returns the same plan — the same weeks, the same session ids,
//! the same prescribed paces. That property is load-bearing: calendar `UID`s,
//! COROS workout ids and plan-diff views all depend on being able to regenerate
//! a plan without churning its identity.
//!
//! The pipeline is three stages, each independently testable:
//!
//! 1. [`volume`] — periodisation: which phase each week belongs to, the weekly
//!    volume curve, and the safety rails (step ceiling, volume ceiling,
//!    projected acute:chronic band, deloads).
//! 2. [`pace`] — one pace table per plan, derived from VO2max where the provider
//!    has reported it and from experience level otherwise, so every prescribed
//!    pace in the plan is mutually consistent.
//! 3. [`sessions`] — placement: which weekdays train, which of those are quality
//!    days, and the structured blocks inside each session.
//!
//! [`builder`] ties the three together and collects the [`PlanNote`]s that
//! explain to the athlete where the engine had to compromise.

#![forbid(unsafe_code)]

pub mod builder;
pub mod pace;
pub mod sessions;
pub mod volume;

pub use builder::{
    GeneratedPlan, PlanNote, earliest_start, generate, phase_note, plan_name, planned_volume,
    shift_start,
};
pub use pace::PaceModel;
pub use sessions::{
    build_week, pick_weekdays, quality_kind, week_start_of, weekly_frequency, weekly_quality_count,
};
pub use volume::{
    ACWR_TARGET_HIGH, ACWR_TARGET_LOW, REBUILD_ALLOWANCE, WeekVolume, ceiling_volume, ema,
    phase_spans, projected_acwr, starting_volume, throttled_start, volume_curve,
};
