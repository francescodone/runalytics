use serde::Serialize;

use super::engine::Phase;
use crate::types::{PlanKind, PlanWorkout, RaceDistance, TrainingPlan, WorkoutKind};
use chrono::{Datelike, NaiveDate};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum QualityPattern {
    Repeats(u32),
    Continuous,
    Cruise,
    Progressive,
    /// Over/unders: reps at 10K pace floated at marathon pace, never stopping.
    Rolling(u32),
    /// Descending 3:2:1 blocks from just slower than threshold to 10K pace.
    Ladder,
}

impl QualityPattern {
    pub(super) fn kind(self) -> WorkoutKind {
        if matches!(self, Self::Repeats(_)) {
            WorkoutKind::Intervals
        } else {
            WorkoutKind::Tempo
        }
    }

    pub(super) fn pace(self, threshold: f64) -> (i64, i64) {
        match self {
            Self::Repeats(metres) if metres <= 600 => super::engine::interval_s_km(threshold),
            Self::Repeats(metres) => {
                let race_m = if metres <= 1000 { 5000.0 } else { 10_000.0 };
                let pace = super::engine::race_pace_s_km(threshold, race_m);
                ((pace - 5.0).round() as i64, (pace + 5.0).round() as i64)
            }
            Self::Rolling(_) => {
                let pace = super::engine::race_pace_s_km(threshold, 10_000.0);
                ((pace - 5.0).round() as i64, (pace + 5.0).round() as i64)
            }
            _ => super::engine::tempo_s_km(threshold),
        }
    }

    /// The band for this pattern moved by an accepted pacing adjustment.
    ///
    /// `offset` is seconds per km added to the prescribed pace, so a *negative*
    /// offset sharpens the range (the runner kept running down) and a positive
    /// one softens it. Only quality paces ever move — see
    /// [`TrainingPlan::quality_offset_sec_km`].
    pub(super) fn pace_at(self, threshold: f64, offset: f64) -> (i64, i64) {
        if offset == 0.0 {
            return self.pace(threshold);
        }
        let (low, high) = self.pace(threshold);
        let shift = |pace: i64| (pace as f64 + offset).round().max(1.0) as i64;
        (shift(low), shift(high))
    }
}

/// The quality sessions a plan week can hold: one threshold-family and one
/// speed-family pattern. A week runs `single` when it has one quality day and
/// both (`emphasis` first) when it has two. Because each kind maps to exactly
/// one pattern per week, a session is reconstructed from its kind alone.
#[derive(Clone, Copy, Debug)]
pub(super) struct WeekQuality {
    pub threshold: QualityPattern,
    pub speed: QualityPattern,
    pub single: QualityPattern,
    pub emphasis: QualityPattern,
}

impl WeekQuality {
    pub(super) fn for_kind(self, kind: WorkoutKind) -> QualityPattern {
        if kind == WorkoutKind::Intervals {
            self.speed
        } else {
            self.threshold
        }
    }

    pub(super) fn other(self, pattern: QualityPattern) -> QualityPattern {
        if pattern.kind() == WorkoutKind::Intervals {
            self.threshold
        } else {
            self.speed
        }
    }
}

/// Periodised session choice (Pfitzinger's mesocycles; Casado et al. 2022):
/// - Base: aerobic foundation — progressive/steady threshold-lite running with
///   an occasional set of 400s to keep leg speed.
/// - Build: lactate threshold (cruise, ladders, over/unders) alongside VO2max
///   reps of 400-1000 m, alternating when there is one quality day.
/// - Peak: race-specific — long threshold for 21K/42K, 600-1600 m reps for
///   5K/10K — with the other family every third session.
/// - Taper: short sharpening reps.
pub(super) fn week_quality(
    kind: PlanKind,
    distance: RaceDistance,
    week: usize,
    weeks: usize,
) -> WeekQuality {
    use QualityPattern::*;
    let index = week - week / 4;
    // The closing weeks of a build narrow to short repeats: sharpening into a
    // race, and sharpening into an improvement block's time trial. A
    // maintenance block has neither, so it keeps rotating the full mix —
    // otherwise week 7 of a held block would look like a taper it isn't in.
    if (kind.has_race() || kind == PlanKind::Improvement) && weeks.saturating_sub(week) <= 2 {
        return WeekQuality {
            threshold: Cruise,
            speed: Repeats(400),
            single: Repeats(400),
            emphasis: Repeats(400),
        };
    }
    let long_race = matches!(distance, RaceDistance::K21 | RaceDistance::K42);
    // (threshold list, speed list, speed leads, minority every third slot)
    let (thresholds, speeds, speed_led, every_third): (&[QualityPattern], &[QualityPattern], _, _) =
        match super::engine::phase_at(kind, distance, weeks, week) {
            Phase::Base => (
                &[Progressive, Rolling(400), Continuous],
                &[Repeats(400)],
                false,
                true,
            ),
            Phase::Build => (
                &[Cruise, Ladder, Continuous, Rolling(1000)],
                &[Repeats(400), Repeats(800), Repeats(600), Repeats(1000)],
                !long_race,
                false,
            ),
            Phase::Peak | Phase::Taper if long_race => (
                &[Continuous, Cruise, Ladder, Progressive],
                &[Repeats(1000), Repeats(1600), Repeats(600)],
                false,
                true,
            ),
            Phase::Peak | Phase::Taper => (
                &[Cruise, Rolling(400), Ladder],
                &[Repeats(1000), Repeats(600), Repeats(800), Repeats(400)],
                true,
                true,
            ),
        };
    let minority = if every_third {
        index % 3 == 2
    } else {
        index % 2 == 1
    };
    // Ordinal of each family's sessions so far, so each list rotates in full.
    let minority_n = if every_third { index / 3 } else { index / 2 };
    let majority_n = index - minority_n;
    let (threshold_n, speed_n) = if speed_led {
        (minority_n, majority_n)
    } else {
        (majority_n, minority_n)
    };
    let threshold = thresholds[threshold_n % thresholds.len()];
    let speed = speeds[speed_n % speeds.len()];
    WeekQuality {
        threshold,
        speed,
        single: if speed_led != minority {
            speed
        } else {
            threshold
        },
        emphasis: if speed_led { speed } else { threshold },
    }
}

/// How a week's long run is run. Only 21K/42K long runs carry race-pace work
/// (Pfitzinger's marathon-pace long runs; both reference builds ran MP blocks
/// and fast finishes in the final two months).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LongRunPattern {
    Easy,
    /// Last ~20 % at race pace (cap 6 km).
    ShortFinish,
    /// Last ~25 % at race pace (cap 12 km).
    LongFinish,
    /// Two race-pace blocks split by an easy float.
    RaceBlocks,
    /// Half-marathon effort two weeks out, as both builds raced one.
    TuneUp,
}

impl LongRunPattern {
    /// A long race-pace segment makes the long run the week's second hard
    /// day; a short fast finish is a light touch that still leaves room.
    pub(super) fn is_hard(self) -> bool {
        !matches!(self, Self::Easy | Self::ShortFinish)
    }
}

pub(super) fn long_run_pattern(
    kind: PlanKind,
    distance: RaceDistance,
    week: usize,
    weeks: usize,
) -> LongRunPattern {
    use LongRunPattern::*;
    if !matches!(distance, RaceDistance::K21 | RaceDistance::K42) {
        return Easy;
    }
    let weeks_to_race = weeks.saturating_sub(week + 1);
    if kind.has_race() && distance == RaceDistance::K42 && weeks >= 10 && weeks_to_race == 2 {
        return TuneUp;
    }
    if (week + 1) % 4 == 0 {
        return Easy;
    }
    let index = week - week / 4;
    match super::engine::phase_at(kind, distance, weeks, week) {
        Phase::Base | Phase::Taper => Easy,
        Phase::Build => [Easy, ShortFinish][index % 2],
        Phase::Peak if distance == RaceDistance::K42 => [LongFinish, Easy, RaceBlocks][index % 3],
        Phase::Peak => [LongFinish, Easy, ShortFinish][index % 3],
    }
}

/// Steps of a long run: easy throughout, or with race-pace segments. Each
/// segment is rounded to 500 m and the easy running absorbs the remainder, so
/// the steps always sum to `distance_m`.
pub(super) fn long_run_steps(
    distance_m: f64,
    pattern: LongRunPattern,
    threshold: f64,
    race: RaceDistance,
) -> Vec<WorkoutStep> {
    let easy = super::engine::easy_pace_s_km(threshold);
    let easy: Band = (Some(easy.0), Some(easy.1));
    let around = |pace: f64| -> Band {
        (
            Some((pace - 6.0).round() as i64),
            Some((pace + 6.0).round() as i64),
        )
    };
    let half = around(super::engine::race_pace_s_km(
        threshold,
        RaceDistance::K21.distance_m(),
    ));
    let race_band = if race == RaceDistance::K42 {
        let mp = super::engine::mp_s_km(threshold);
        (Some(mp.0), Some(mp.1))
    } else {
        half
    };
    let round = |m: f64| (m / 500.0).floor() * 500.0;
    let easy_step = |label: &str, metres: f64| open(step(label, metres, easy));
    let plain = || vec![easy_step("Long run - easy", distance_m)];
    let finish = |share: f64, cap: f64| {
        let fast = round((share * distance_m).min(cap));
        if fast < 2000.0 {
            return plain();
        }
        vec![
            easy_step("Long run - easy", distance_m - fast),
            step("Long run - finish at race pace", fast, race_band),
        ]
    };
    match pattern {
        LongRunPattern::Easy => plain(),
        LongRunPattern::ShortFinish => finish(0.2, 6000.0),
        LongRunPattern::LongFinish => finish(0.25, 12_000.0),
        LongRunPattern::RaceBlocks => {
            let block = round((0.18 * distance_m).min(7000.0));
            let float = 3000.0;
            let rest = distance_m - 2.0 * block - float;
            let warm = round(rest / 2.0);
            if block < 3000.0 || warm < 3000.0 {
                return finish(0.25, 12_000.0);
            }
            vec![
                easy_step("Long run - easy", warm),
                step("Long run - race-pace block", block, race_band),
                easy_step("Long run - easy float", float),
                step("Long run - race-pace block", block, race_band),
                easy_step("Long run - easy", rest - warm),
            ]
        }
        LongRunPattern::TuneUp => {
            let warm = 3000.0;
            let block = round((distance_m - 5000.0).min(14_000.0));
            if block < 8000.0 {
                return plain();
            }
            vec![
                easy_step("Warm-up - easy jog", warm),
                step("Tune-up - half-marathon effort", block, half),
                easy_step("Cool-down - easy jog", distance_m - warm - block),
            ]
        }
    }
}

/// Display title for a long run built from `steps`.
pub(super) fn long_run_title(km: f64, steps: &[WorkoutStep]) -> String {
    let fast: Vec<&WorkoutStep> = steps.iter().filter(|s| !s.label.contains("easy")).collect();
    match fast.as_slice() {
        [] => format!("Long run {km:.0} km"),
        [one] if one.label.starts_with("Tune-up") => {
            format!(
                "Tune-up {km:.0} km — {:.1} km at half-marathon effort",
                one.distance_m / 1000.0
            )
        }
        [one] => format!(
            "Long run {km:.0} km — last {:.1} km at race pace",
            one.distance_m / 1000.0
        ),
        [first, ..] => format!(
            "Long run {km:.0} km — {} x {:.1} km at race pace",
            fast.len(),
            first.distance_m / 1000.0
        ),
    }
}

/// Separates a quality session's name from its structure: `Tempo Tapas · 4 x 1.5 km tempo`.
pub const TITLE_SEP: &str = " · ";

const QUARTER_NAMES: &[&str] = &["Quarter Pounders", "Hamster Wheel", "Oval Office", "Round and Round"];
const HALF_NAMES: &[&str] = &["Pace Gremlins", "Two-Lap Tantrum", "Half-Mile Mood Swings", "Eight Hundred Ways to Wheeze"];
const KILO_NAMES: &[&str] = &["Thousand-Metre Stare", "The Full Kilo", "Kilo Kickoff"];
const MILE_NAMES: &[&str] = &["Big Rep Energy", "Mile Marker Madness", "The Long Reps Are Coming"];
const ROLLING_NAMES: &[&str] = &["Seesaw Shuffle", "Yo-Yo Tempo", "Gear Grinder", "Wave Rider", "Pace Roulette"];
const CRUISE_NAMES: &[&str] = &["Tempo Tapas", "Block Party", "Cruise Control", "Threshold Buffet"];
const LADDER_NAMES: &[&str] = &["Stairway to Tempo", "Countdown Crew", "Staircase Shuffle", "Down the Rabbit Hole"];
const PROGRESSIVE_NAMES: &[&str] = &["Slow Burn", "Dimmer Switch", "Negative Nelly", "Turn It Up"];
const CONTINUOUS_NAMES: &[&str] = &["Hold My Tempo", "Metronome Mode", "Straight Talk", "The Grind Stone"];

/// Display title for a quality session of `pattern` built into `details`: a
/// name picked by `seed` (the plan week, so repeats of a pattern rotate)
/// followed by the structure.
pub(super) fn quality_title(
    pattern: QualityPattern,
    details: &WorkoutDetails,
    seed: usize,
) -> String {
    if details.steps.len() < 3 {
        return "Easy run - reduced session".into();
    }
    let main = &details.steps[1..details.steps.len() - 1];
    let first = &main[0];
    let work_km: f64 = main
        .iter()
        .map(|s| s.distance_m * s.repeats as f64)
        .sum::<f64>()
        / 1000.0;
    let (names, structure) = match pattern {
        QualityPattern::Repeats(_) => (
            match first.distance_m {
                m if m <= 500.0 => QUARTER_NAMES,
                m if m <= 900.0 => HALF_NAMES,
                m if m <= 1200.0 => KILO_NAMES,
                _ => MILE_NAMES,
            },
            format!("{} x {:.0} m", first.repeats, first.distance_m),
        ),
        QualityPattern::Rolling(_) if first.repeats > 1 => (
            ROLLING_NAMES,
            format!("{} x {:.0} m over/unders", first.repeats, first.distance_m),
        ),
        QualityPattern::Cruise if first.repeats > 1 => (
            CRUISE_NAMES,
            format!("{} x {:.1} km tempo", first.repeats, first.distance_m / 1000.0),
        ),
        QualityPattern::Ladder if main.len() == 3 => (
            LADDER_NAMES,
            format!(
                "{:.1}/{:.1}/{:.1} km ladder",
                main[0].distance_m / 1000.0,
                main[1].distance_m / 1000.0,
                main[2].distance_m / 1000.0
            ),
        ),
        QualityPattern::Progressive if main.len() > 1 => (
            PROGRESSIVE_NAMES,
            format!("{work_km:.1} km progressive tempo"),
        ),
        _ => (CONTINUOUS_NAMES, format!("{work_km:.1} km tempo")),
    };
    format!("{}{TITLE_SEP}{structure}", names[seed % names.len()])
}

/// The new-style title for a quality session still carrying the old plain
/// one, or `None` when there is nothing to rename.
pub fn title_for_plan(workout: &PlanWorkout, plan: &TrainingPlan) -> Option<String> {
    if !matches!(workout.kind, WorkoutKind::Intervals | WorkoutKind::Tempo)
        || workout.title.contains(TITLE_SEP)
    {
        return None;
    }
    let details = details_for_plan(workout, plan);
    if details.steps.len() < 3 {
        return None;
    }
    let (weeks, week) = plan_week(plan, workout);
    let pattern = week_quality(plan.kind, plan.distance, week, weeks).for_kind(workout.kind);
    Some(quality_title(pattern, &details, week))
}

/// `(weeks, week)` — the plan's total week count and the zero-based plan week
/// `workout` sits in, derived the same way the engine built the schedule.
/// Shared by `details_for_plan` and the tests so a session's fitness ramp is
/// reconstructed exactly as it was sized.
fn plan_week(plan: &TrainingPlan, workout: &PlanWorkout) -> (usize, usize) {
    let weeks = ((plan.target_date - plan.created_at.date_naive()).num_days() as f64 / 7.0)
        .ceil()
        .max(1.0) as usize;
    let start = plan.target_date - chrono::Duration::days(7 * weeks as i64 - 1);
    let monday = start - chrono::Duration::days(start.weekday().num_days_from_monday() as i64);
    let week = ((workout.date - monday).num_days() / 7).max(0) as usize;
    (weeks, week)
}

/// The quality-session load a plan prescribes for `workout`'s week: the plan's
/// end-of-plan load scaled by how far through the build that week sits.
pub(super) fn workout_load(plan: &TrainingPlan, workout: &PlanWorkout) -> Option<f64> {
    let (weeks, week) = plan_week(plan, workout);
    plan.baseline_weekly_km.map(|baseline| {
        super::engine::quality_load_at(
            plan.distance,
            baseline,
            super::engine::fitness_progress(plan.kind, plan.distance, weeks, week),
        )
    })
}

/// The quality-pace offset in force for a session on `date`.
///
/// Zero when no adjustment was ever accepted, and zero for sessions dated
/// before it took effect — which is what keeps a session the runner already ran
/// being graded against the range it was actually given, rather than one the
/// plan only acquired afterwards.
pub fn quality_offset_for(plan: &TrainingPlan, date: NaiveDate) -> f64 {
    match plan.quality_offset_from {
        Some(from) if date >= from => plan.quality_offset_sec_km,
        _ => 0.0,
    }
}

/// The pace band `workout` actually asked for, offset included — the range the
/// runner's reps are graded against, and the one the UI draws.
pub fn quality_band(plan: &TrainingPlan, workout: &PlanWorkout) -> Option<(i64, i64)> {
    let (low, high) = (workout.pace_lo_s_km?, workout.pace_hi_s_km?);
    if !matches!(workout.kind, WorkoutKind::Intervals | WorkoutKind::Tempo) {
        return Some((low, high));
    }
    let offset = quality_offset_for(plan, workout.date);
    if offset == 0.0 {
        return Some((low, high));
    }
    let shift = |pace: i64| (pace as f64 + offset).round().max(1.0) as i64;
    Some((shift(low), shift(high)))
}

pub fn details_for_plan(workout: &PlanWorkout, plan: &TrainingPlan) -> WorkoutDetails {
    let (weeks, week) = plan_week(plan, workout);
    // The plan's threshold sharpens week by week (engine::threshold_at); the
    // reconstruction must use the week's value, not the one the plan was
    // created with, or the sized main set no longer fits the workout and every
    // session falls back to the distance-filling legacy path. A maintenance
    // block holds its threshold, which `threshold_at` reports as flat.
    let thr =
        super::engine::threshold_at(plan.kind, plan.distance, plan.threshold_sec_km, weeks, week);
    match workout.kind {
        WorkoutKind::Intervals | WorkoutKind::Tempo => {
            let pattern =
                week_quality(plan.kind, plan.distance, week, weeks).for_kind(workout.kind);
            build_details(
                workout,
                Some(thr),
                pattern,
                workout_load(plan, workout),
                quality_offset_for(plan, workout.date),
            )
        }
        WorkoutKind::LongRun => {
            let pattern = long_run_pattern(plan.kind, plan.distance, week, weeks);
            let steps = long_run_steps(
                workout.distance_m.unwrap_or(0.0).max(0.0),
                pattern,
                thr,
                plan.distance,
            );
            let purpose = if pattern.is_hard() {
                LONG_RUN_RACE_PURPOSE
            } else {
                LONG_RUN_PURPOSE
            };
            let steps = if workout.distance_m.unwrap_or(0.0) > 0.0 {
                steps
            } else {
                Vec::new()
            };
            WorkoutDetails {
                purpose: purpose.into(),
                estimated_min: estimate_min(&steps, workout.duration_min),
                steps,
            }
        }
        _ => details(workout, Some(thr)),
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkoutStep {
    pub label: String,
    pub repeats: u32,
    pub distance_m: f64,
    pub pace_lo_s_km: Option<i64>,
    pub pace_hi_s_km: Option<i64>,
    pub recovery_m: f64,
    pub recovery_pace_lo_s_km: Option<i64>,
    pub recovery_pace_hi_s_km: Option<i64>,
    /// Easy running: `pace_lo_s_km` is a floor and anything slower is fine.
    /// `pace_hi_s_km` stays set only for the time and load maths.
    pub open_ended: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkoutDetails {
    pub purpose: String,
    pub steps: Vec<WorkoutStep>,
    pub estimated_min: Option<f64>,
}

impl WorkoutDetails {
    pub fn distance_m(&self) -> f64 {
        steps_m(&self.steps)
    }
}

fn steps_m(steps: &[WorkoutStep]) -> f64 {
    steps
        .iter()
        .map(|step| {
            step.distance_m * step.repeats as f64
                + step.recovery_m * step.repeats.saturating_sub(1) as f64
        })
        .sum()
}

const WARMUP_M: f64 = 1500.0;
const COOLDOWN_M: f64 = 1000.0;

type Band = (Option<i64>, Option<i64>);

/// Stress of a step in threshold-minutes: time at pace weighted by intensity², intensity = threshold / pace.
pub(super) fn step_load(step: &WorkoutStep, threshold: f64) -> f64 {
    let (Some(low), Some(high)) = (step.pace_lo_s_km, step.pace_hi_s_km) else {
        return 0.0;
    };
    let pace = (low + high) as f64 / 2.0;
    let minutes = step.distance_m * step.repeats as f64 / 1000.0 * pace / 60.0;
    minutes * (threshold / pace).powi(2)
}

/// Load of a main set. Recoveries count only when they are floats run faster
/// than easy pace (over/unders); jog recoveries are rest, not work.
pub(super) fn set_load(steps: &[WorkoutStep], threshold: f64) -> f64 {
    steps
        .iter()
        .map(|s| {
            let float = match (s.recovery_pace_lo_s_km, s.recovery_pace_hi_s_km) {
                (Some(lo), Some(hi))
                    if s.recovery_m > 0.0 && ((lo + hi) as f64 / 2.0) < threshold + 40.0 =>
                {
                    let metres = s.recovery_m * s.repeats.saturating_sub(1) as f64;
                    step_load(&step("", metres, (Some(lo), Some(hi))), threshold)
                }
                _ => 0.0,
            };
            step_load(s, threshold) + float
        })
        .sum()
}

fn widen(low: i64, high: i64) -> Band {
    let midpoint = (low + high) / 2;
    (
        Some(low.min(midpoint - 5).max(1)),
        Some(high.max(midpoint + 5)),
    )
}

/// Move a prescribed band by the plan's pacing offset, keeping the endpoints
/// positive (a band that reaches 0 s/km would divide by zero in the load math).
fn shift_band(band: Band, offset: f64) -> Band {
    if offset == 0.0 {
        return band;
    }
    let shift = |pace: Option<i64>| pace.map(|p| (p as f64 + offset).round().max(1.0) as i64);
    (shift(band.0), shift(band.1))
}

fn step(label: &str, distance_m: f64, pace: Band) -> WorkoutStep {
    WorkoutStep {
        label: label.into(),
        repeats: 1,
        distance_m,
        pace_lo_s_km: pace.0,
        pace_hi_s_km: pace.1,
        recovery_m: 0.0,
        recovery_pace_lo_s_km: None,
        recovery_pace_hi_s_km: None,
        open_ended: false,
    }
}

fn open(mut step: WorkoutStep) -> WorkoutStep {
    step.open_ended = true;
    step
}

fn with_recovery(mut work: WorkoutStep, repeats: u32, recovery_m: f64, easy: Band) -> WorkoutStep {
    work.repeats = repeats;
    work.recovery_m = recovery_m;
    work.recovery_pace_lo_s_km = easy.0;
    work.recovery_pace_hi_s_km = easy.1;
    work
}

/// Total distance of a planned quality session whose main set delivers `load` threshold-minutes.
pub(super) fn quality_session_m(pattern: QualityPattern, threshold: f64, load: f64) -> f64 {
    let pace = pattern.pace(threshold);
    let easy = super::engine::easy_pace_s_km(threshold);
    let main = main_set(
        pattern,
        Some(threshold),
        widen(pace.0, pace.1),
        (Some(easy.0), Some(easy.1)),
        load,
        f64::INFINITY,
        0.0,
    );
    WARMUP_M + steps_m(&main) + COOLDOWN_M
}

/// Main set sized to `target` load (threshold-minutes) within the 25 min interval / 30 min tempo
/// caps. An infinite target fills `available` instead (legacy plans, trimmed sessions).
///
/// `offset` is the plan's accepted pacing adjustment in seconds per km: every
/// prescribed pace is shifted by it, so the session is built for the range the
/// runner is actually being asked to hold.
fn main_set(
    pattern: QualityPattern,
    threshold: Option<f64>,
    band: Band,
    easy: Band,
    target: f64,
    available: f64,
    offset: f64,
) -> Vec<WorkoutStep> {
    let thr = threshold.unwrap_or(300.0);
    let load_of = |steps: &[WorkoutStep]| set_load(steps, thr);
    if let QualityPattern::Repeats(requested) = pattern {
        let rep_band = |metres: u32| {
            threshold
                .map(|t| QualityPattern::Repeats(metres).pace_at(t, offset))
                .map(|pace| (Some(pace.0), Some(pace.1)))
                .unwrap_or(band)
        };
        let metres = [1600_u32, 1000, 800, 600, 400]
            .into_iter()
            .find(|&metres| {
                let recovery = if metres >= 800 { 400.0 } else { 200.0 };
                let pace = rep_band(metres).1.unwrap_or(300) as f64;
                metres <= requested
                    && 2.0 * metres as f64 + recovery <= available
                    && (metres == 400
                        || metres as f64 / 1000.0 * pace
                            <= if metres == 1600 { 480.0 } else { 360.0 })
            })
            .unwrap_or(400);
        let fast_band = rep_band(metres);
        let recovery = if metres >= 800 { 400.0 } else { 200.0 };
        let max_work_m = 25.0 * 60.0 / fast_band.1.unwrap_or(300) as f64 * 1000.0;
        let per_m = step_load(&step("", 1.0, fast_band), thr);
        let mut rep_m = metres as f64;
        let fit = ((available + recovery) / (rep_m + recovery)).floor();
        let upper = (max_work_m / rep_m).floor().min(20.0).min(fit).max(2.0);
        let repeats = (target / (per_m * rep_m)).round().clamp(2.0, upper);
        if metres >= 600 && target.is_finite() {
            // Whole reps alone miss the target by up to half a rep; trim the rep length to close it.
            // Only when the whole-rep set is actually off target — a 600 m rep
            // that already lands within 5 % stays a round 600 m.
            let miss = ((repeats * rep_m * per_m) - target).abs() > 0.05 * target;
            if miss {
                let tuned = (target / (per_m * repeats) / 100.0).round() * 100.0;
                let tuned = tuned.clamp(
                    (rep_m * 0.8 / 100.0).ceil() * 100.0,
                    (rep_m * 1.25 / 100.0).floor() * 100.0,
                );
                if repeats * tuned <= max_work_m {
                    rep_m = tuned;
                }
            }
        }
        let work = step("Fast repetitions - jog between reps only", rep_m, fast_band);
        return vec![with_recovery(work, repeats as u32, recovery, easy)];
    }
    let tempo_hi = band.1.unwrap_or(300) as f64 + offset;
    let band = shift_band(band, offset);
    let cap_m = |slowdown: f64| available.min(30.0 * 60.0 / (tempo_hi + slowdown) * 1000.0);
    let per_100 = step_load(&step("", 100.0, band), thr);
    let continuous = || {
        let cap = (cap_m(0.0) / 100.0).floor() * 100.0;
        let work = ((target / per_100).round() * 100.0).clamp(1000.0_f64.min(cap), cap);
        vec![step("Tempo - continuous", work, band)]
    };
    match pattern {
        QualityPattern::Cruise => {
            let mut best: Option<(f64, Vec<WorkoutStep>)> = None;
            let cap = cap_m(0.0);
            for repeats in 2..=20_u32 {
                let n = repeats as f64;
                // 1-2 km blocks in 100 m steps, sized to the target within the time cap and session.
                let room = (cap / n).min((available + 500.0) / n - 500.0);
                let block = ((target / (n * per_100)).round() * 100.0)
                    .min(2000.0)
                    .min((room / 100.0).floor() * 100.0);
                if block < 1000.0 {
                    break;
                }
                let set = vec![with_recovery(
                    step("Tempo - cruise intervals", block, band),
                    repeats,
                    500.0,
                    easy,
                )];
                let error = if target.is_finite() {
                    (load_of(&set) - target).abs()
                } else {
                    -load_of(&set)
                };
                if best.as_ref().map_or(true, |(e, _)| error < *e) {
                    best = Some((error, set));
                }
            }
            best.map(|(_, set)| set).unwrap_or_else(continuous)
        }
        QualityPattern::Progressive => {
            let shifted = |offset: i64| (band.0.map(|p| p + offset), band.1.map(|p| p + offset));
            let cap = cap_m(25.0);
            let trio: f64 = [25, 10, 0]
                .iter()
                .map(|&offset| step_load(&step("", 500.0, shifted(offset)), thr))
                .sum();
            let block = ((target / trio).floor() * 500.0).min((cap / 3.0 / 500.0).floor() * 500.0);
            if block < 1000.0 {
                return continuous();
            }
            let mut set = vec![
                step("Tempo - progressive block", block, shifted(25)),
                step("Tempo - progressive block", block, shifted(10)),
            ];
            let last_cap = ((cap - 2.0 * block) / 100.0).floor() * 100.0;
            let last = ((target - load_of(&set)) / per_100).round() * 100.0;
            set.push(step(
                "Tempo - progressive block",
                last.clamp(500.0, last_cap),
                band,
            ));
            set
        }
        QualityPattern::Ladder => {
            let shifted = |offset: i64| (band.0.map(|p| p + offset), band.1.map(|p| p + offset));
            let paces = [shifted(10), band, shifted(-8)];
            // Load of a ladder whose shortest block is 100 m (300/200/100 m).
            let unit: f64 = paces
                .iter()
                .zip([3.0, 2.0, 1.0])
                .map(|(pace, k)| step_load(&step("", 100.0 * k, *pace), thr))
                .sum();
            let max_block = (cap_m(10.0) / 6.0 / 50.0).floor() * 50.0;
            let block = ((target / unit * 2.0).round() * 50.0).min(max_block);
            if block < 300.0 {
                return continuous();
            }
            vec![
                step("Tempo - ladder", 3.0 * block, paces[0]),
                step("Tempo - ladder", 2.0 * block, paces[1]),
                step("Tempo - ladder", block, paces[2]),
            ]
        }
        QualityPattern::Rolling(metres) => {
            let rep = metres as f64;
            let float_band = match threshold {
                Some(t) => {
                    let mp = super::engine::mp_s_km(t);
                    shift_band((Some(mp.0), Some(mp.1)), offset)
                }
                None => (band.0.map(|p| p + 20), band.1.map(|p| p + 20)),
            };
            let over = step_load(&step("", rep, band), thr);
            let under = step_load(&step("", rep, float_band), thr);
            let secs = |pace: Band| rep / 1000.0 * pace.1.unwrap_or(300) as f64;
            let max_n = (2..=20_u32)
                .take_while(|&n| {
                    let n = n as f64;
                    n * secs(band) + (n - 1.0) * secs(float_band) <= 30.0 * 60.0
                        && (2.0 * n - 1.0) * rep <= available
                })
                .last();
            let Some(max_n) = max_n else {
                return continuous();
            };
            let n = ((target + under) / (over + under))
                .round()
                .clamp(2.0, max_n as f64);
            // Whole pairs miss the target by up to half a pair; trim the rep
            // length (50 m steps, ±25 %) to close it while staying in the caps.
            let per_m = (n * over + (n - 1.0) * under) / rep;
            let tuned = ((target / per_m) / 50.0).round() * 50.0;
            let tuned = tuned.clamp(
                (rep * 0.75 / 50.0).ceil() * 50.0,
                (rep * 1.25 / 50.0).floor() * 50.0,
            );
            let fits = |m: f64| {
                let scale = m / rep;
                scale * (n * secs(band) + (n - 1.0) * secs(float_band)) <= 30.0 * 60.0
                    && (2.0 * n - 1.0) * m <= available
            };
            let rep = if target.is_finite() && fits(tuned) {
                tuned
            } else {
                rep
            };
            let work = step(
                "Over/unders - fast, float between at marathon pace",
                rep,
                band,
            );
            vec![with_recovery(work, n as u32, rep, float_band)]
        }
        _ => continuous(),
    }
}

pub fn details(workout: &PlanWorkout, threshold: Option<f64>) -> WorkoutDetails {
    let pattern = if workout.kind == WorkoutKind::Intervals {
        QualityPattern::Repeats(400)
    } else {
        QualityPattern::Continuous
    };
    build_details(workout, threshold, pattern, None, 0.0)
}

const LONG_RUN_PURPOSE: &str = "Long, sustained aerobic running raises glycogen storage, capillary density and fatigue resistance, and conditions tendons and bones. Keep it conversational; ease off if the effort stops feeling sustainable.";
const LONG_RUN_RACE_PURPOSE: &str = "Race-pace segments on pre-fatigued legs rehearse race-specific economy and pacing, while the easy running around them keeps the session aerobic and recovery manageable. Keep the easy parts truly easy so the fast parts are controlled and even.";

fn quality_purpose(pattern: QualityPattern) -> &'static str {
    match pattern {
        QualityPattern::Repeats(metres) if metres <= 600 => "Short fast reps near VO2max pace recruit fast-twitch fibres and sharpen running economy and neuromuscular speed; brief jog recoveries let you repeat quality without lactate build-up. Keep the last rep as fast as the first.",
        QualityPattern::Repeats(_) => "Reps at roughly 5-10K pace accumulate time close to VO2max, the strongest stimulus for aerobic power, while jog recoveries clear lactate so each rep stays quality. Hold the last rep as controlled as the first.",
        QualityPattern::Cruise => "Cruise intervals (Daniels) accumulate more time at lactate-threshold pace than a single run could, with short jog recoveries to limit fatigue. This raises the pace you can sustain before blood lactate climbs steeply.",
        QualityPattern::Progressive => "Progressive blocks climb from just below to threshold pace, training lactate clearance as fatigue accumulates and rehearsing negative-split pacing. Each block should feel controlled, not raced.",
        QualityPattern::Ladder => "Descending blocks step from just slower than threshold toward 10K pace as they shorten, training threshold endurance and the ability to change gear on tired legs without extra load.",
        QualityPattern::Rolling(_) => "Over/unders alternate efforts just above threshold with floats at marathon pace and no stopping, so you produce lactate and clear it while still running. This trains lactate clearance and the ability to recover mid-race.",
        QualityPattern::Continuous => "A sustained run at lactate-threshold pace, about the effort you could hold for an hour, improves lactate clearance and the fraction of VO2max you can sustain. Stay even and finish without sprinting.",
    }
}

fn build_details(
    workout: &PlanWorkout,
    threshold: Option<f64>,
    pattern: QualityPattern,
    load: Option<f64>,
    offset: f64,
) -> WorkoutDetails {
    let purpose = match workout.kind {
        WorkoutKind::Intervals | WorkoutKind::Tempo => quality_purpose(pattern),
        WorkoutKind::Easy => "Easy aerobic running, below the first lactate threshold, builds mitochondrial density, capillary supply and fat oxidation with little mechanical and hormonal stress, so the hard days can stay hard. If you can't talk in full sentences, slow down.",
        WorkoutKind::LongRun => LONG_RUN_PURPOSE,
        WorkoutKind::Race => "Race effort: starting at the slower end of the target range conserves glycogen and delays lactate accumulation, and even pacing is the most economical way to cover the distance. Settle into rhythm before pushing.",
        WorkoutKind::Rest => "Rest day. No running prescribed.",
    }.to_string();
    let band = match (workout.pace_lo_s_km, workout.pace_hi_s_km) {
        (Some(low), Some(high)) => widen(low, high),
        _ => (None, None),
    };
    let easy = threshold.map(super::engine::easy_pace_s_km);
    let easy_band = (easy.map(|pace| pace.0), easy.map(|pace| pace.1));
    let distance = workout.distance_m.unwrap_or(0.0).max(0.0);
    let mut steps = Vec::new();
    if distance > 0.0 && workout.kind != WorkoutKind::Rest {
        if matches!(workout.kind, WorkoutKind::Intervals | WorkoutKind::Tempo) && distance >= 3500.0
        {
            let available = distance - WARMUP_M - COOLDOWN_M;
            let main = threshold
                .and(load)
                .map(|load| {
                    main_set(
                        pattern,
                        threshold,
                        band,
                        easy_band,
                        load,
                        f64::INFINITY,
                        offset,
                    )
                })
                .filter(|main| steps_m(main) <= available + 0.01)
                .unwrap_or_else(|| {
                    main_set(
                        pattern,
                        threshold,
                        band,
                        easy_band,
                        f64::INFINITY,
                        available,
                        offset,
                    )
                });
            // Mileage beyond the main set lands in the warm-up; the cool-down stays fixed.
            let warmup = distance - COOLDOWN_M - steps_m(&main);
            steps.push(open(step("Warm-up - easy jog", warmup, easy_band)));
            steps.extend(main);
            steps.push(open(step("Cool-down - easy jog", COOLDOWN_M, easy_band)));
        } else {
            let short_quality = matches!(workout.kind, WorkoutKind::Intervals | WorkoutKind::Tempo);
            let plain = step(
                if short_quality {
                    "Easy run - reduced session"
                } else {
                    "Continuous run"
                },
                distance,
                if short_quality { easy_band } else { band },
            );
            let easy_run = short_quality
                || matches!(workout.kind, WorkoutKind::Easy | WorkoutKind::LongRun);
            steps.push(if easy_run { open(plain) } else { plain });
        }
    }
    WorkoutDetails {
        purpose,
        estimated_min: estimate_min(&steps, workout.duration_min),
        steps,
    }
}

fn estimate_min(steps: &[WorkoutStep], fallback: Option<f64>) -> Option<f64> {
    if steps.is_empty() {
        return fallback;
    }
    steps
        .iter()
        .map(|step| {
            let pace = (step.pace_lo_s_km? + step.pace_hi_s_km?) as f64 / 2.0;
            let running = step.distance_m * step.repeats as f64 / 1000.0 * pace;
            let recovery = if step.recovery_m > 0.0 {
                let recovery_pace =
                    (step.recovery_pace_lo_s_km? + step.recovery_pace_hi_s_km?) as f64 / 2.0;
                step.recovery_m * step.repeats.saturating_sub(1) as f64 / 1000.0 * recovery_pace
            } else {
                0.0
            };
            Some((running + recovery) / 60.0)
        })
        .sum::<Option<f64>>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    fn request(distance: RaceDistance, weeks: i64, threshold: f64, baseline: f64) -> PlanRequest {
        PlanRequest {
            distance,
            kind: PlanKind::Race,
            target_date: chrono::Utc::now().date_naive() + chrono::Duration::days(weeks * 7),
            threshold_sec_km: threshold,
            weekly_frequency: 4,
            longrun_day: 7,
            increase: IncreaseMethod::Moderate,
            current_weekly_km: Some(baseline),
        }
    }

    #[test]
    fn marathon_rotates_reps_and_tempo_formats() {
        let (plan, workouts) =
            super::super::engine::generate(&request(RaceDistance::K42, 24, 228.0, 50.0), 50.0)
                .unwrap();
        let mut reps = std::collections::BTreeSet::new();
        let mut tempos = std::collections::BTreeSet::new();
        for workout in workouts {
            let breakdown = details_for_plan(&workout, &plan);
            if workout.kind == WorkoutKind::Intervals {
                reps.insert(breakdown.steps[1].distance_m as u32);
            } else if workout.kind == WorkoutKind::Tempo {
                tempos.insert(breakdown.steps[1].label.clone());
            }
        }
        // Rep lengths of 800 m and up are trimmed to hit the load, so check families, not exact metres.
        assert!(
            reps.len() >= 3,
            "expected the rotation's rep lengths: {reps:?}"
        );
        assert_eq!(reps.first(), Some(&400));
        assert!(reps.iter().any(|m| (700..=1200).contains(m)));
        assert!(reps.iter().any(|m| *m >= 1300), "no long reps: {reps:?}");
        assert!(
            tempos.len() >= 4,
            "expected continuous, cruise, progressive, ladder and over/under tempos: {tempos:?}"
        );
        let late: Vec<_> = (12..22)
            .filter(|week| (week + 1) % 4 != 0)
            .map(|week| {
                week_quality(PlanKind::Race, RaceDistance::K42, week, 24)
                    .single
                    .kind()
            })
            .collect();
        assert!(
            late.iter()
                .filter(|kind| **kind == WorkoutKind::Tempo)
                .count()
                > late.len() / 2
        );
    }

    #[test]
    fn quality_sessions_stay_bounded_and_stable_across_runner_profiles() {
        for distance in [
            RaceDistance::K5,
            RaceDistance::K10,
            RaceDistance::K21,
            RaceDistance::K42,
        ] {
            for weeks in [3, 8, 16, 24] {
                for threshold in [228.0, 285.0, 390.0] {
                    for (baseline, frequency) in
                        [(10.0, 4), (30.0, 4), (65.0, 4), (45.0, 5), (70.0, 6)]
                    {
                        let mut req = request(distance, weeks, threshold, baseline);
                        req.weekly_frequency = frequency;
                        let (plan, workouts) =
                            super::super::engine::generate(&req, baseline).unwrap();
                        for workout in workouts {
                            let breakdown = details_for_plan(&workout, &plan);
                            assert!((breakdown.distance_m() - workout.distance_m.unwrap()).abs() < 0.01,
                                "{:?}, {weeks} weeks, threshold {threshold}, {baseline} km: {} totals {} != {:?}", distance, workout.title, breakdown.distance_m(), workout.distance_m);
                            assert_eq!(breakdown.estimated_min, workout.duration_min);
                            if !matches!(workout.kind, WorkoutKind::Intervals | WorkoutKind::Tempo)
                            {
                                continue;
                            }
                            assert!(breakdown.steps.first().unwrap().distance_m >= 1500.0);
                            assert_eq!(breakdown.steps.last().unwrap().distance_m, 1000.0);
                            // The plan ramps fitness, so each week's session is
                            // sized to that week's threshold and load, not the
                            // end-of-plan values. Measure and compare at the
                            // week's own threshold.
                            let (n_weeks, week) = plan_week(&plan, &workout);
                            let thr_w = super::super::engine::threshold_at(
                                PlanKind::Race,
                                distance,
                                threshold,
                                n_weeks,
                                week,
                            );
                            let target = workout_load(&plan, &workout).unwrap();
                            let load =
                                set_load(&breakdown.steps[1..breakdown.steps.len() - 1], thr_w);
                            assert!(
                                (load - target).abs() <= 0.1 * target,
                                "{distance:?}, {weeks} weeks, threshold {threshold}, {baseline} km: {} load {load:.1} vs {target:.1}",
                                workout.title
                            );
                            let work_seconds: f64 = breakdown.steps[1..breakdown.steps.len() - 1]
                                .iter()
                                .map(|step| {
                                    step.distance_m * step.repeats as f64 / 1000.0
                                        * step.pace_hi_s_km.unwrap() as f64
                                })
                                .sum();
                            assert!(
                                work_seconds
                                    <= if workout.kind == WorkoutKind::Intervals {
                                        1500.0
                                    } else {
                                        1800.0
                                    }
                            );
                            assert!(breakdown
                                .steps
                                .iter()
                                .all(|step| step.distance_m > 0.0 && step.repeats > 0));
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn recovery_weeks_are_easy_and_quality_is_spaced_from_long_runs() {
        for frequency in 3..=7 {
            for longrun_day in 1..=7 {
                let mut request = request(RaceDistance::K42, 16, 285.0, 40.0);
                request.weekly_frequency = frequency;
                request.longrun_day = longrun_day;
                let (_, workouts) = super::super::engine::generate(&request, 40.0).unwrap();
                let start = request.target_date - chrono::Duration::days(16 * 7 - 1);
                let monday =
                    start - chrono::Duration::days(start.weekday().num_days_from_monday() as i64);
                for workout in workouts.iter().filter(|workout| {
                    matches!(workout.kind, WorkoutKind::Intervals | WorkoutKind::Tempo)
                }) {
                    let week = (workout.date - monday).num_days() / 7;
                    assert_ne!((week + 1) % 4, 0);
                    // Hard days never touch: no quality session the day
                    // before or after another one.
                    for other in workouts.iter().filter(|other| {
                        other.id != workout.id
                            && matches!(other.kind, WorkoutKind::Intervals | WorkoutKind::Tempo)
                    }) {
                        assert!((workout.date - other.date).num_days().abs() >= 2,
                            "frequency {frequency}, long-run day {longrun_day}: back-to-back quality {} and {}", workout.date, other.date);
                    }
                    for long in workouts
                        .iter()
                        .filter(|other| other.kind == WorkoutKind::LongRun)
                    {
                        assert!((workout.date - long.date).num_days().abs() >= 2,
                            "frequency {frequency}, long-run day {longrun_day}: quality {} next to long run {}", workout.date, long.date);
                    }
                }
            }
        }
    }

    #[test]
    fn longer_repetitions_use_slower_paces() {
        let short = QualityPattern::Repeats(400).pace(228.0);
        let medium = QualityPattern::Repeats(1000).pace(228.0);
        let long = QualityPattern::Repeats(1600).pace(228.0);
        assert!(short.0 < medium.0 && medium.0 < long.0);
        assert!((199..=215).contains(&short.0));
        assert!((211..=225).contains(&long.0));
    }

    #[test]
    fn warmup_absorbs_spare_distance_and_cooldown_is_fixed() {
        let workout = PlanWorkout {
            id: "rounding-test".into(),
            plan_id: "test".into(),
            date: chrono::NaiveDate::from_ymd_opt(2027, 1, 5).unwrap(),
            kind: WorkoutKind::Intervals,
            title: "Intervals".into(),
            distance_m: Some(6870.0),
            duration_min: None,
            pace_lo_s_km: Some(245),
            pace_hi_s_km: Some(260),
            done: false,
        };
        for kind in [WorkoutKind::Intervals, WorkoutKind::Tempo] {
            let workout = PlanWorkout {
                kind,
                ..workout.clone()
            };
            let breakdown = details(&workout, Some(285.0));
            assert!(breakdown.steps[0].distance_m >= 1500.0);
            assert_eq!(breakdown.steps.last().unwrap().distance_m, 1000.0);
            assert!((breakdown.distance_m() - 6870.0).abs() < 0.01);
        }
    }

    #[test]
    fn steps_preserve_distance_and_include_recovery_and_pace_ranges() {
        let request = PlanRequest {
            distance: RaceDistance::K10,
            kind: PlanKind::Race,
            target_date: chrono::Utc::now().date_naive() + chrono::Duration::days(56),
            threshold_sec_km: 285.0,
            weekly_frequency: 4,
            longrun_day: 7,
            increase: IncreaseMethod::Moderate,
            current_weekly_km: Some(30.0),
        };
        let (plan, workouts) = super::super::engine::generate(&request, 30.0).unwrap();
        for workout in &workouts {
            let breakdown = details_for_plan(workout, &plan);
            let total: f64 = breakdown
                .steps
                .iter()
                .map(|step| {
                    step.distance_m * step.repeats as f64
                        + step.recovery_m * step.repeats.saturating_sub(1) as f64
                })
                .sum();
            assert!((total - workout.distance_m.unwrap()).abs() < 0.01);
            for step in &breakdown.steps {
                assert!(step.distance_m > 0.0);
                assert!(step.pace_hi_s_km.unwrap() - step.pace_lo_s_km.unwrap() >= 10);
            }
            if workout.kind == WorkoutKind::Intervals {
                let reps = &breakdown.steps[1];
                // Reps of 800 m and up are trimmed in 100 m steps to hit the
                // week's load, so families rather than exact metres.
                assert!(reps.distance_m >= 400.0 && reps.distance_m <= 1600.0);
                assert!([200.0, 400.0].contains(&reps.recovery_m));
                assert!(reps.repeats >= 2);
            }
        }
        let mut easy = workouts[0].clone();
        easy.kind = WorkoutKind::Easy;
        easy.distance_m = Some(10_000.0);
        easy.pace_lo_s_km = Some(295);
        easy.pace_hi_s_km = Some(305);
        assert_eq!(details(&easy, Some(240.0)).estimated_min, Some(50.0));
    }
}
