//! Structured workouts: what a session actually contains.
//!
//! The shape mirrors what a COROS `createScheduledWorkout` accepts — a note, a
//! type, and an ordered list of blocks with a target, duration and pace or
//! heart-rate band — so a [`PlannedSession`] can be pushed to the watch without
//! a lossy translation layer.

use serde::{Deserialize, Serialize};

use crate::units::{DurationSecs, HeartRate, Pace, VolumeKm};
use crate::{Date, TimeOfDay};

/// The purpose of a session in the weekly structure.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionKind {
    /// Full rest. Emitted into the calendar as an informational event only.
    Rest,
    /// Aerobic base work, conversational.
    #[default]
    Easy,
    /// Deliberately shorter and slower than Easy, used after quality days.
    Recovery,
    /// The week's longest effort.
    LongRun,
    /// Easy volume with short accelerations to maintain neuromuscular sharpness.
    Progression,
    /// Sustained "comfortably hard" effort, roughly 20-40 minutes at threshold.
    Tempo,
    /// Cruise intervals: repeated threshold-length reps with short recovery.
    CruiseIntervals,
    /// Short, fast reps at 5K-to-3K effort.
    Intervals,
    /// Longer reps at marathon-to-half-marathon pace.
    ExtensiveIntervals,
    /// Unstructured speedplay, usually on grass.
    Fartlek,
    /// The race itself, or a rehearsal of it.
    Race,
    /// Non-impact cardio that protects aerobic volume without loading tissue.
    CrossTraining,
}

impl SessionKind {
    /// Stable key for database columns and MCP arguments.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rest => "rest",
            Self::Easy => "easy",
            Self::Recovery => "recovery",
            Self::LongRun => "long_run",
            Self::Progression => "progression",
            Self::Tempo => "tempo",
            Self::CruiseIntervals => "cruise_intervals",
            Self::Intervals => "intervals",
            Self::ExtensiveIntervals => "extensive_intervals",
            Self::Fartlek => "fartlek",
            Self::Race => "race",
            Self::CrossTraining => "cross_training",
        }
    }

    /// Whether the session is intended to stress the athlete.
    ///
    /// Drives training-load accounting: only quality sessions may push the
    /// acute:chronic ratio out of the safe band.
    #[must_use]
    pub const fn is_quality(self) -> bool {
        matches!(
            self,
            Self::Tempo
                | Self::CruiseIntervals
                | Self::Intervals
                | Self::ExtensiveIntervals
                | Self::Fartlek
                | Self::Race
        )
    }

    /// Whether the session should appear in the calendar at all.
    #[must_use]
    pub const fn is_schedulable(self) -> bool {
        !matches!(self, Self::Rest)
    }

    /// Whether the session counts toward weekly running volume.
    #[must_use]
    pub const fn counts_toward_volume(self) -> bool {
        !matches!(self, Self::Rest | Self::CrossTraining)
    }

    /// Human label for UI and calendar titles.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Rest => "Rest",
            Self::Easy => "Easy run",
            Self::Recovery => "Recovery run",
            Self::LongRun => "Long run",
            Self::Progression => "Easy with strides",
            Self::Tempo => "Tempo",
            Self::CruiseIntervals => "Cruise intervals",
            Self::Intervals => "Intervals",
            Self::ExtensiveIntervals => "Marathon-pace intervals",
            Self::Fartlek => "Fartlek",
            Self::Race => "Race day",
            Self::CrossTraining => "Cross training",
        }
    }

    /// Typical weekday this kind lands on, used as a placement hint.
    #[must_use]
    pub const fn preferred_weekday(self) -> u32 {
        match self {
            Self::LongRun | Self::Race => 6,
            Self::Tempo | Self::CruiseIntervals => 2,
            Self::Intervals | Self::ExtensiveIntervals | Self::Fartlek => 4,
            _ => 0,
        }
    }
}

impl std::fmt::Display for SessionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

impl std::str::FromStr for SessionKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "rest" => Ok(Self::Rest),
            "easy" => Ok(Self::Easy),
            "recovery" => Ok(Self::Recovery),
            "long_run" => Ok(Self::LongRun),
            "progression" => Ok(Self::Progression),
            "tempo" => Ok(Self::Tempo),
            "cruise_intervals" => Ok(Self::CruiseIntervals),
            "intervals" => Ok(Self::Intervals),
            "extensive_intervals" => Ok(Self::ExtensiveIntervals),
            "fartlek" => Ok(Self::Fartlek),
            "race" => Ok(Self::Race),
            "cross_training" => Ok(Self::CrossTraining),
            other => Err(format!("unknown session kind '{other}'")),
        }
    }
}

/// The five-zone `%HRmax` model shared with COROS and Garmin.
///
/// Variants serialise as `Z1`..`Z5` verbatim, which is the token both providers
/// use on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum HrZone {
    Z1,
    Z2,
    Z3,
    Z4,
    Z5,
}

impl HrZone {
    /// Upper bound of the zone as a fraction of max heart rate.
    #[must_use]
    pub const fn ceiling_pct(self) -> f64 {
        match self {
            Self::Z1 => 0.60,
            Self::Z2 => 0.70,
            Self::Z3 => 0.80,
            Self::Z4 => 0.90,
            Self::Z5 => 1.00,
        }
    }

    /// Lower bound of the zone as a fraction of max heart rate.
    #[must_use]
    pub const fn floor_pct(self) -> f64 {
        match self {
            Self::Z1 => 0.50,
            Self::Z2 => 0.60,
            Self::Z3 => 0.70,
            Self::Z4 => 0.80,
            Self::Z5 => 0.90,
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Z1 => "Recovery",
            Self::Z2 => "Aerobic",
            Self::Z3 => "Tempo",
            Self::Z4 => "Threshold",
            Self::Z5 => "VO2max",
        }
    }

    /// Classify a heart rate into a zone given the athlete's max.
    #[must_use]
    pub fn classify(hr: HeartRate, max_hr: HeartRate) -> Option<Self> {
        let pct = hr.pct_of_max(max_hr)?;
        Some(if pct <= Self::Z1.ceiling_pct() {
            Self::Z1
        } else if pct <= Self::Z2.ceiling_pct() {
            Self::Z2
        } else if pct <= Self::Z3.ceiling_pct() {
            Self::Z3
        } else if pct <= Self::Z4.ceiling_pct() {
            Self::Z4
        } else {
            Self::Z5
        })
    }
}

/// What an athlete is asked to do inside one block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockTarget {
    Warmup,
    Easy,
    Recovery,
    /// Sustained threshold work.
    Steady,
    /// The hard portion of an interval session.
    Hard,
    /// All-out but short.
    Sprint,
    Strides,
    Cooldown,
    /// Off-feet effort, used on cross-training days.
    Other,
}

impl BlockTarget {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Warmup => "warmup",
            Self::Easy => "easy",
            Self::Recovery => "recovery",
            Self::Steady => "steady",
            Self::Hard => "hard",
            Self::Sprint => "sprint",
            Self::Strides => "strides",
            Self::Cooldown => "cooldown",
            Self::Other => "other",
        }
    }

    /// Whether this block counts as intensity for load purposes.
    #[must_use]
    pub const fn is_intensity(self) -> bool {
        matches!(self, Self::Steady | Self::Hard | Self::Sprint)
    }
}

/// One contiguous instruction inside a workout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkoutBlock {
    pub target: BlockTarget,
    /// How long the block lasts. Repeats are expressed by listing blocks in
    /// order rather than with a repeat counter, which is what the COROS API
    /// expects.
    pub duration: DurationSecs,
    /// Prescribed pace, when the block is pace-driven.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pace: Option<Pace>,
    /// Prescribed heart-rate ceiling, when the block is effort-driven. Easy and
    /// recovery days are heart-rate-capped on purpose: pace drifts in heat and
    /// at altitude, effort does not.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hr_ceiling: Option<HeartRate>,
    /// Zone label carried through to the watch face.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone: Option<HrZone>,
}

impl WorkoutBlock {
    #[must_use]
    pub fn new(target: BlockTarget, duration: DurationSecs) -> Self {
        Self {
            target,
            duration,
            pace: None,
            hr_ceiling: None,
            zone: None,
        }
    }

    #[must_use]
    pub fn with_pace(mut self, pace: Pace) -> Self {
        self.pace = Some(pace);
        self
    }

    #[must_use]
    pub fn with_hr_ceiling(mut self, hr: HeartRate, zone: HrZone) -> Self {
        self.hr_ceiling = Some(hr);
        self.zone = Some(zone);
        self
    }

    #[must_use]
    pub fn volume_at(&self, pace: Pace) -> VolumeKm {
        Pace::from_parts(VolumeKm(1.0), self.duration)
            .map(|_| VolumeKm(f64::from(self.duration.as_u32()) / pace.as_secs_per_km()))
            .unwrap_or(VolumeKm::ZERO)
    }
}

/// The ordered blocks that make up a session.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredWorkout {
    pub blocks: Vec<WorkoutBlock>,
}

impl StructuredWorkout {
    /// A single continuous effort at one target.
    #[must_use]
    pub fn continuous(target: BlockTarget, duration: DurationSecs) -> Self {
        Self {
            blocks: vec![WorkoutBlock::new(target, duration)],
        }
    }

    /// Total prescribed duration.
    #[must_use]
    pub fn total_duration(&self) -> DurationSecs {
        self.blocks
            .iter()
            .fold(DurationSecs::ZERO, |acc, b| acc + b.duration)
    }

    /// Duration spent in intensity-bearing blocks.
    #[must_use]
    pub fn intensity_duration(&self) -> DurationSecs {
        self.blocks
            .iter()
            .filter(|b| b.target.is_intensity())
            .fold(DurationSecs::ZERO, |acc, b| acc + b.duration)
    }

    /// Volume estimate at a reference pace, used when no pace is prescribed.
    #[must_use]
    pub fn estimate_volume(&self, reference_pace: Pace) -> VolumeKm {
        self.blocks
            .iter()
            .map(|b| b.volume_at(b.pace.unwrap_or(reference_pace)))
            .fold(VolumeKm::ZERO, |acc, v| acc + v)
    }

    /// Warm-up and cool-down padding around a hard middle, the standard shape
    /// for every quality day.
    #[must_use]
    pub fn with_warmup_cooldown(
        warmup: DurationSecs,
        middle: Vec<WorkoutBlock>,
        cooldown: DurationSecs,
    ) -> Self {
        let mut blocks = Vec::with_capacity(middle.len() + 2);
        blocks.push(WorkoutBlock::new(BlockTarget::Warmup, warmup));
        blocks.extend(middle);
        blocks.push(WorkoutBlock::new(BlockTarget::Cooldown, cooldown));
        Self { blocks }
    }
}

/// A single day's prescribed training.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedSession {
    /// Stable id so a session can be referenced by the MCP server, the calendar
    /// `UID`, and the COROS workout id recorded after a push.
    pub id: crate::ids::PlannedSessionId,
    pub date: Date,
    /// Local wall-clock start, in the athlete's time zone.
    pub start: TimeOfDay,
    pub kind: SessionKind,
    /// Short human title, e.g. "6 x 1km @ 4:10".
    pub title: String,
    /// Coach-facing rationale shown in the UI and in the calendar description.
    pub intent: String,
    pub workout: StructuredWorkout,
    /// Prescribed distance, the number the athlete is most likely to check.
    pub target_volume: VolumeKm,
    /// Prescribed duration, used when the day is time-based rather than
    /// distance-based.
    pub target_duration: DurationSecs,
    /// Prescribed pace for the dominant portion, when pace-driven.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_pace: Option<Pace>,
    /// Perceived-effort target on a 1-10 scale, used to reconcile planned
    /// against actual load.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rpe_target: Option<u8>,
    /// Whether this session is expected to stress the athlete.
    pub quality: bool,
    /// Provider-side workout id after a successful push, kept so re-planning
    /// updates rather than duplicates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
}

impl PlannedSession {
    /// Whether anything is scheduled for this day.
    #[must_use]
    pub fn is_rest(&self) -> bool {
        self.kind == SessionKind::Rest
    }

    /// Volume attributed to the week, zero for rest and cross-training.
    #[must_use]
    pub fn volume(&self) -> VolumeKm {
        if self.kind.counts_toward_volume() {
            self.target_volume
        } else {
            VolumeKm::ZERO
        }
    }

    /// A one-line summary suitable for a calendar description.
    #[must_use]
    pub fn summary_line(&self) -> String {
        match self.target_pace {
            Some(pace) if self.kind.is_quality() => {
                format!("{} · {} · {}", self.kind.label(), self.target_volume, pace)
            }
            _ => format!("{} · {}", self.kind.label(), self.target_volume),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rest_and_cross_training_are_outside_volume() {
        let mut session = PlannedSession {
            id: crate::ids::PlannedSessionId::new(),
            date: Date::from_ymd_opt(2026, 1, 1).expect("test date"),
            start: TimeOfDay::from_hms_opt(7, 0, 0).unwrap(),
            kind: SessionKind::Rest,
            title: "Rest".into(),
            intent: String::new(),
            workout: StructuredWorkout::default(),
            target_volume: VolumeKm(0.0),
            target_duration: DurationSecs::ZERO,
            target_pace: None,
            rpe_target: None,
            quality: false,
            external_id: None,
        };
        assert_eq!(session.volume(), VolumeKm::ZERO);
        assert!(!session.kind.is_schedulable());

        session.kind = SessionKind::CrossTraining;
        session.target_volume = VolumeKm(8.0);
        assert_eq!(
            session.volume(),
            VolumeKm::ZERO,
            "the bike does not count as run volume"
        );
    }

    #[test]
    fn warmup_cooldown_wraps_the_hard_middle() {
        let middle = vec![
            WorkoutBlock::new(BlockTarget::Hard, DurationSecs::from_minutes(4)),
            WorkoutBlock::new(BlockTarget::Recovery, DurationSecs::from_minutes(2)),
        ];
        let w = StructuredWorkout::with_warmup_cooldown(
            DurationSecs::from_minutes(15),
            middle,
            DurationSecs::from_minutes(10),
        );
        assert_eq!(w.blocks.len(), 4);
        assert_eq!(w.total_duration(), DurationSecs::from_minutes(31));
        assert_eq!(w.intensity_duration(), DurationSecs::from_minutes(4));
    }

    #[test]
    fn zone_classification_uses_pct_of_max() {
        let max = HeartRate::new(200);
        assert_eq!(HrZone::classify(HeartRate::new(110), max), Some(HrZone::Z1));
        assert_eq!(HrZone::classify(HeartRate::new(150), max), Some(HrZone::Z3));
        assert_eq!(HrZone::classify(HeartRate::new(185), max), Some(HrZone::Z5));
        assert_eq!(HrZone::classify(HeartRate::NONE, max), None);
    }

    #[test]
    fn quality_kinds_are_the_intensity_ones() {
        assert!(SessionKind::Tempo.is_quality());
        assert!(!SessionKind::LongRun.is_quality());
        assert!(SessionKind::LongRun.counts_toward_volume());
    }
}
