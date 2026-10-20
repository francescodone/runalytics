use serde::Serialize;

use crate::types::{PlanKind, PlanWorkout, RaceDistance, TrainingPlan, WorkoutKind};
use chrono::{Datelike, NaiveDate};

#[derive(Clone, Copy, Debug)]
pub(super) enum QualityPattern {
    Repeats(u32),
    Continuous,
    Cruise,
    Progressive,
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

pub(super) fn quality_pattern(
    kind: PlanKind,
    distance: RaceDistance,
    week: usize,
    weeks: usize,
) -> QualityPattern {
    use QualityPattern::*;
    let index = week - week / 4;
    // The closing weeks of a build narrow to short repeats: sharpening into a
    // race, and sharpening into an improvement block's time trial. A
    // maintenance block has neither, so it keeps rotating the full mix —
    // otherwise week 7 of a held block would look like a taper it isn't in.
    if kind.has_race() || kind == PlanKind::Improvement {
        if weeks.saturating_sub(week) <= 2 {
            return Repeats(400);
        }
    }
    if week >= weeks / 2 && matches!(distance, RaceDistance::K21 | RaceDistance::K42) {
        [
            Cruise,
            Repeats(1000),
            Progressive,
            Continuous,
            Repeats(1600),
            Continuous,
        ][index % 6]
    } else {
        [
            Repeats(400),
            Continuous,
            Repeats(800),
            Cruise,
            Repeats(600),
            Progressive,
        ][index % 6]
    }
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
    let pattern = quality_pattern(plan.kind, plan.distance, week, weeks);
    // The plan's threshold sharpens week by week (engine::threshold_at); the
    // reconstruction must use the week's value, not the one the plan was
    // created with, or the sized main set no longer fits the workout and every
    // session falls back to the distance-filling legacy path. A maintenance
    // block holds its threshold, which `threshold_at` reports as flat.
    let thr =
        super::engine::threshold_at(plan.kind, plan.distance, plan.threshold_sec_km, weeks, week);
    if pattern.kind() == workout.kind {
        build_details(
            workout,
            Some(thr),
            pattern,
            workout_load(plan, workout),
            quality_offset_for(plan, workout.date),
        )
    } else {
        details(workout, Some(thr))
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
    }
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
    let load_of = |steps: &[WorkoutStep]| steps.iter().map(|s| step_load(s, thr)).sum::<f64>();
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

fn build_details(
    workout: &PlanWorkout,
    threshold: Option<f64>,
    pattern: QualityPattern,
    load: Option<f64>,
    offset: f64,
) -> WorkoutDetails {
    let purpose = match workout.kind {
        WorkoutKind::Intervals => "Controlled fast repetitions with easy jog recoveries. Keep the final repetition as controlled as the first.",
        WorkoutKind::Tempo => "Controlled threshold work, continuous or broken into blocks. Progress gradually and finish without sprinting.",
        WorkoutKind::Easy => "Relaxed aerobic running at a conversational effort.",
        WorkoutKind::LongRun => "Build endurance with a controlled sustained run. Ease off if the effort stops feeling sustainable.",
        WorkoutKind::Race => "Race effort. Start at the slower end of the target range and settle into rhythm.",
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
            steps.push(step("Warm-up - easy jog", warmup, easy_band));
            steps.extend(main);
            steps.push(step("Cool-down - easy jog", COOLDOWN_M, easy_band));
        } else {
            let short_quality = matches!(workout.kind, WorkoutKind::Intervals | WorkoutKind::Tempo);
            steps.push(step(
                if short_quality {
                    "Easy run - reduced session"
                } else {
                    "Continuous run"
                },
                distance,
                if short_quality { easy_band } else { band },
            ));
        }
    }
    let estimated_min = if steps.is_empty() {
        workout.duration_min
    } else {
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
    };
    WorkoutDetails {
        purpose,
        steps,
        estimated_min,
    }
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
            reps.len() >= 5,
            "expected the rotation's rep lengths: {reps:?}"
        );
        assert_eq!(reps.first(), Some(&400));
        assert!(reps.contains(&600));
        assert!(reps.iter().any(|m| (700..=1000).contains(m)));
        assert!(reps.iter().any(|m| *m >= 1300), "no long reps: {reps:?}");
        assert_eq!(
            tempos.len(),
            3,
            "expected continuous, cruise and progressive tempos: {tempos:?}"
        );
        let late: Vec<_> = (12..22)
            .filter(|week| (week + 1) % 4 != 0)
            .map(|week| quality_pattern(PlanKind::Race, RaceDistance::K42, week, 24).kind())
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
                    for baseline in [10.0, 30.0, 65.0] {
                        let (plan, workouts) = super::super::engine::generate(
                            &request(distance, weeks, threshold, baseline),
                            baseline,
                        )
                        .unwrap();
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
                            let load: f64 = breakdown.steps[1..breakdown.steps.len() - 1]
                                .iter()
                                .map(|step| step_load(step, thr_w))
                                .sum();
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
