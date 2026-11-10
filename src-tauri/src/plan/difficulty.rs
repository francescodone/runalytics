use super::session::{estimate_min, set_load, WorkoutDetails};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Easier,
    Harder,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Customization {
    #[serde(default)]
    pub prescription: Option<Prescription>,
    #[serde(default)]
    pub rating: Option<u8>,
    #[serde(default)]
    pub submitted_at: Option<i64>,
    #[serde(default)]
    pub dismissed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prescription {
    pub version: u32,
    pub baseline: WorkoutDetails,
    pub current: WorkoutDetails,
}

fn work_load(details: &WorkoutDetails, threshold: f64) -> f64 {
    let work: Vec<_> = details
        .steps
        .iter()
        .filter(|step| !step.open_ended)
        .cloned()
        .collect();
    set_load(&work, threshold)
}

pub fn nudge(
    current: &WorkoutDetails,
    baseline: &WorkoutDetails,
    threshold: f64,
    cap_minutes: f64,
    direction: Direction,
) -> Option<WorkoutDetails> {
    if current.steps.len() < 3
        || baseline.steps.len() < 3
        || !threshold.is_finite()
        || threshold <= 0.0
    {
        return None;
    }
    let end = current.steps.len() - 1;
    let current_load = work_load(current, threshold);
    let base_load = work_load(baseline, threshold);
    if current_load <= 0.0 || base_load <= 0.0 {
        return None;
    }
    let sign = if direction == Direction::Harder {
        1.0
    } else {
        -1.0
    };
    let target = current_load * (1.0 + sign * 0.05);
    let mut best: Option<(f64, WorkoutDetails)> = None;
    for delta in 1..=10 {
        let mut candidate = current.clone();
        for step in &mut candidate.steps[1..end] {
            if !step.open_ended {
                step.distance_m =
                    (step.distance_m * (1.0 + sign * delta as f64 / 100.0) / 100.0).round() * 100.0;
            }
        }
        let difference = candidate.distance_m() - current.distance_m();
        candidate.steps[0].distance_m -= difference;
        if candidate.steps[0].distance_m < 1500.0
            || candidate.steps[end].distance_m < 1000.0
            || candidate.steps[1..end]
                .iter()
                .any(|step| step.distance_m < 100.0)
        {
            continue;
        }
        let load = work_load(&candidate, threshold);
        let work_minutes: f64 = candidate.steps[1..end]
            .iter()
            .filter(|step| !step.open_ended)
            .map(|step| {
                step.distance_m * step.repeats as f64 / 1000.0
                    * step.pace_hi_s_km.unwrap_or(0) as f64
                    / 60.0
            })
            .sum();
        if sign * (load - current_load) <= 0.0001
            || (load / current_load - 1.0).abs() > 0.10001
            || load < base_load * 0.8 - 0.0001
            || load > base_load * 1.2 + 0.0001
            || work_minutes > cap_minutes + 0.0001
            || (candidate.distance_m() - current.distance_m()).abs() > 0.001
        {
            continue;
        }
        candidate.estimated_min = estimate_min(&candidate.steps, current.estimated_min);
        let error = (load - target).abs();
        if best.as_ref().is_none_or(|(previous, _)| error < *previous) {
            best = Some((error, candidate));
        }
    }
    if best.is_none() {
        for index in 1..end {
            let original = &current.steps[index];
            if original.open_ended
                || original.repeats < 2
                || original.distance_m + sign * 100.0 < 100.0
                || original
                    .recovery_pace_hi_s_km
                    .is_some_and(|pace| (pace as f64) < threshold + 40.0)
            {
                continue;
            }
            let mut candidate = current.clone();
            candidate.steps[index].repeats -= 1;
            let mut last = original.clone();
            last.repeats = 1;
            last.distance_m += sign * 100.0;
            last.recovery_m = 0.0;
            let mut extra = Vec::new();
            if original.recovery_m > 0.0 {
                let mut recovery = last.clone();
                recovery.label = "Recovery - easy jog".into();
                recovery.distance_m = original.recovery_m;
                recovery.pace_lo_s_km = original.recovery_pace_lo_s_km;
                recovery.pace_hi_s_km = original.recovery_pace_hi_s_km;
                recovery.open_ended = true;
                extra.push(recovery);
            }
            extra.push(last);
            candidate.steps.splice(index + 1..index + 1, extra);
            candidate.steps[0].distance_m -= sign * 100.0;
            let load = work_load(&candidate, threshold);
            let minutes: f64 = candidate
                .steps
                .iter()
                .filter(|step| !step.open_ended)
                .map(|step| {
                    step.distance_m * step.repeats as f64 / 1000.0
                        * step.pace_hi_s_km.unwrap_or(0) as f64
                        / 60.0
                })
                .sum();
            if candidate.steps[0].distance_m >= 1500.0
                && (candidate.distance_m() - current.distance_m()).abs() < 0.001
                && sign * (load - current_load) > 0.0001
                && (load / current_load - 1.0).abs() <= 0.10001
                && load >= base_load * 0.8 - 0.0001
                && load <= base_load * 1.2 + 0.0001
                && minutes <= cap_minutes + 0.0001
            {
                candidate.estimated_min = estimate_min(&candidate.steps, current.estimated_min);
                return Some(candidate);
            }
        }
    }
    if best.is_none() && direction == Direction::Harder {
        let current_minutes: f64 = current.steps[1..end]
            .iter()
            .filter(|step| !step.open_ended)
            .map(|step| step.distance_m * step.repeats as f64 / 1000.0
                * step.pace_hi_s_km.unwrap_or(0) as f64 / 60.0)
            .sum();
        for seconds in 1..=10 {
            let mut candidate = current.clone();
            let mut valid = true;
            for step in &mut candidate.steps[1..end] {
                if !step.open_ended {
                    match (step.pace_lo_s_km, step.pace_hi_s_km) {
                        (Some(low), Some(high)) if low - sign as i64 * seconds >= 120 => {
                            step.pace_lo_s_km = Some(low - sign as i64 * seconds);
                            step.pace_hi_s_km = Some(high - sign as i64 * seconds);
                        }
                        _ => valid = false,
                    }
                }
            }
            let load = work_load(&candidate, threshold);
            let minutes: f64 = candidate.steps[1..end]
                .iter()
                .filter(|step| !step.open_ended)
                .map(|step| step.distance_m * step.repeats as f64 / 1000.0
                    * step.pace_hi_s_km.unwrap_or(0) as f64 / 60.0)
                .sum();
            if valid && current_minutes <= cap_minutes + 0.0001
                && sign * (load - current_load) > 0.0001
                && (load / current_load - 1.0).abs() <= 0.10001
                && load >= base_load * 0.8 - 0.0001
                && load <= base_load * 1.2 + 0.0001
                && minutes <= cap_minutes + 0.0001
            {
                candidate.estimated_min = estimate_min(&candidate.steps, current.estimated_min);
                let error = (load - target).abs();
                if best.as_ref().is_none_or(|(previous, _)| error < *previous) {
                    best = Some((error, candidate));
                }
            }
        }
    }
    best.map(|(_, details)| details)
}

use crate::{
    error::{AppError, Result},
    store::Store,
    types::*,
};
use chrono::{Datelike, Local, Utc};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackContext {
    pub plan_id: String,
    pub revision: i64,
    pub sessions: Vec<FeedbackSession>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackSession {
    pub workout_id: String,
    pub activity_id: Option<i64>,
    pub title: String,
    pub date: chrono::NaiveDate,
    pub rating: Option<u8>,
    pub submitted: bool,
    pub pending: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub changed: usize,
    pub unchanged: usize,
    pub reason: String,
}

pub fn eligible(workout: &PlanWorkout) -> bool {
    workout.done
        && workout.status != WorkoutStatus::Skipped
        && (matches!(
            workout.kind,
            WorkoutKind::Tempo | WorkoutKind::Intervals | WorkoutKind::LongRun
        ) || workout.kind == WorkoutKind::Race && workout.title.starts_with("Benchmark: "))
}

pub fn context(store: &Store) -> Result<Option<FeedbackContext>> {
    let Some((plan, workouts)) = store.current_plan()? else {
        return Ok(None);
    };
    let revision = store.current_plan_rev()?.unwrap_or(0);
    let completions = store.calendar_completions()?;
    let started = store.feedback_started_at()?;
    let today = Local::now().date_naive();
    let mut sessions = Vec::new();
    for workout in workouts.iter().filter(|workout| eligible(workout)) {
        let value = workout.customization.clone().unwrap_or_default();
        let activity_id = completions
            .iter()
            .find(|entry| entry.0 == workout.id)
            .map(|entry| entry.1);
        let completed_at = match activity_id {
            Some(id) => store
                .activity_by_id(id)?
                .map(|activity| activity.start_time),
            None => None,
        };
        sessions.push(FeedbackSession {
            workout_id: workout.id.clone(),
            activity_id,
            title: workout.title.clone(),
            date: workout.date,
            rating: value.rating,
            submitted: value.submitted_at.is_some(),
            pending: !value.dismissed
                && value.submitted_at.is_none()
                && completed_at.is_some_and(|time| {
                    time.timestamp() >= started
                        && time.date_naive() >= today - chrono::Duration::days(7)
                        && time.date_naive() <= today
                }),
        });
    }
    sessions.sort_by_key(|session| std::cmp::Reverse(session.date));
    Ok(Some(FeedbackContext {
        plan_id: plan.id,
        revision,
        sessions,
    }))
}

pub fn apply(
    store: &Store,
    thresholds: &Thresholds,
    plan_id: &str,
    revision: i64,
    direction: Option<Direction>,
    feedback: Option<(&str, Option<u8>, bool)>,
) -> Result<Outcome> {
    let (plan, workouts) = store
        .current_plan()?
        .ok_or_else(|| AppError::Other("No active plan".into()))?;
    if plan.id != plan_id || store.current_plan_rev()? != Some(revision) {
        return Err(AppError::Other(
            "The plan changed. Refresh before trying again.".into(),
        ));
    }
    let mut updates = Vec::new();
    let mut requested = direction;
    if let Some((id, rating, dismiss)) = feedback {
        if rating.is_some_and(|value| !(1..=5).contains(&value)) {
            return Err(AppError::Other("Choose a rating from 1 to 5".into()));
        }
        let mut workout = workouts
            .iter()
            .find(|workout| workout.id == id && eligible(workout))
            .cloned()
            .ok_or_else(|| AppError::Other("This session is not eligible for feedback".into()))?;
        let value = workout
            .customization
            .get_or_insert_with(Customization::default);
        requested = None;
        if !dismiss {
            if value.submitted_at.is_none() && rating.is_some() {
                requested = match rating {
                    Some(1 | 2) => Some(Direction::Harder),
                    Some(4 | 5) => Some(Direction::Easier),
                    _ => None,
                };
                value.submitted_at = Some(Utc::now().timestamp());
            }
            value.rating = rating;
        }
        value.dismissed = true;
        updates.push(workout);
    }
    let mut outcome = Outcome {
        changed: 0,
        unchanged: 0,
        reason: "Feedback saved.".into(),
    };
    if let Some(direction) = requested {
        let allowed = direction == Direction::Easier
            || super::pacing::progression_allowed(store, thresholds)?;
        if !allowed {
            outcome.reason = if feedback.is_some() { "Feedback saved. Harder adjustments are unavailable with the current pain, recovery or training-load evidence." }
                else { "No sessions changed: harder adjustments are unavailable with the current pain, recovery or training-load evidence." }.into();
        } else {
            let today = Local::now().date_naive();
            let completions = store.calendar_completions()?;
            for workout in workouts.iter().filter(|workout| {
                workout.date >= today
                    && !workout.done
                    && workout.status == WorkoutStatus::Planned
                    && matches!(workout.kind, WorkoutKind::Tempo | WorkoutKind::Intervals)
                    && !completions.iter().any(|entry| entry.0 == workout.id)
            }) {
                let current = super::session::details_for_plan(workout, &plan);
                let existing = workout
                    .customization
                    .as_ref()
                    .and_then(|value| value.prescription.as_ref());
                let baseline = existing.map(|value| &value.baseline).unwrap_or(&current);
                let weeks = super::engine::weeks_of(&plan).max(1);
                let taper_days = super::engine::method_taper_weeks(
                    plan.kind,
                    plan.distance,
                    weeks,
                    plan.training_method,
                ) as i64
                    * 7;
                let protected = super::engine::week_is_down(&plan, workout.date)
                    || (plan.kind.has_race()
                        && workout.date >= plan.target_date - chrono::Duration::days(taper_days));
                if direction == Direction::Harder
                    && (protected
                        || plan
                            .threshold_state
                            .as_ref()
                            .is_some_and(|state| state.hold_quality))
                {
                    outcome.unchanged += 1;
                    continue;
                }
                let cap = plan
                    .threshold_state
                    .as_ref()
                    .map(|state| state.quality_minutes_cap)
                    .unwrap_or(30.0)
                    .min(if workout.kind == WorkoutKind::Intervals {
                        25.0
                    } else {
                        30.0
                    });
                let Some(adjusted) =
                    nudge(&current, baseline, plan.threshold_sec_km, cap, direction)
                else {
                    outcome.unchanged += 1;
                    continue;
                };
                let monday = workout.date
                    - chrono::Duration::days(workout.date.weekday().num_days_from_monday() as i64);
                let same_week = |other: &&PlanWorkout| {
                    other.date >= monday && other.date < monday + chrono::Duration::days(7)
                };
                let week_metres: f64 = workouts
                    .iter()
                    .filter(same_week)
                    .map(|other| other.distance_m.unwrap_or(0.0))
                    .sum();
                let mut hard_minutes = 0.0;
                for other in workouts.iter().filter(same_week) {
                    let details = if other.id == workout.id {
                        adjusted.clone()
                    } else {
                        let updated = updates
                            .iter()
                            .find(|updated| updated.id == other.id)
                            .unwrap_or(other);
                        super::session::details_for_plan(updated, &plan)
                    };
                    hard_minutes += details
                        .steps
                        .iter()
                        .filter(|step| !step.open_ended && !step.label.contains("easy"))
                        .map(|step| {
                            step.distance_m * step.repeats as f64 / 1000.0
                                * step.pace_hi_s_km.unwrap_or(0) as f64
                                / 60.0
                        })
                        .sum::<f64>();
                    hard_minutes += details
                        .steps
                        .iter()
                        .filter(|step| {
                            step.recovery_pace_hi_s_km
                                .is_some_and(|pace| (pace as f64) < plan.threshold_sec_km + 40.0)
                        })
                        .map(|step| {
                            step.recovery_m * step.repeats.saturating_sub(1) as f64 / 1000.0
                                * step.recovery_pace_hi_s_km.unwrap_or(0) as f64
                                / 60.0
                        })
                        .sum::<f64>();
                }
                let easy = super::engine::easy_pace_s_km(plan.threshold_sec_km);
                let budget = week_metres / 1000.0 * (easy.0 + easy.1) as f64 / 2.0 / 60.0 * 0.2;
                if direction == Direction::Harder && hard_minutes > budget + 0.001 {
                    outcome.unchanged += 1;
                    continue;
                }
                let mut changed = workout.clone();
                changed.duration_min = adjusted.estimated_min;
                changed
                    .customization
                    .get_or_insert_with(Customization::default)
                    .prescription = Some(Prescription {
                    version: 1,
                    baseline: baseline.clone(),
                    current: adjusted,
                });
                updates.push(changed);
                outcome.changed += 1;
            }
            outcome.reason = if outcome.changed == 0 { "No sessions changed: the plan is already at its safe adjustment limits." }
                else if outcome.unchanged > 0 { "Future quality sessions adjusted. Some sessions stayed unchanged to preserve training limits. Weekly mileage is unchanged." }
                else { "Future quality sessions adjusted. Weekly mileage is unchanged." }.into();
        }
    }
    if !updates.is_empty() {
        store.commit_difficulty(plan_id, revision, &updates)?;
    }
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::session::WorkoutStep;

    fn example() -> WorkoutDetails {
        let step = |metres, repeats, open, recovery| WorkoutStep {
            label: String::new(),
            distance_m: metres,
            repeats,
            open_ended: open,
            pace_lo_s_km: Some(if open { 360 } else { 300 }),
            pace_hi_s_km: Some(if open { 380 } else { 310 }),
            recovery_m: recovery,
            recovery_pace_lo_s_km: Some(360),
            recovery_pace_hi_s_km: Some(380),
        };
        WorkoutDetails {
            purpose: String::new(),
            estimated_min: None,
            steps: vec![
                step(2500.0, 1, true, 0.0),
                step(1000.0, 4, false, 200.0),
                step(1000.0, 1, true, 0.0),
            ],
        }
    }

    #[test]
    fn easier_preserves_total_metres_pace_and_recovery() {
        let baseline = example();
        let adjusted = nudge(&baseline, &baseline, 300.0, 30.0, Direction::Easier).unwrap();
        assert_eq!(adjusted.steps[1].distance_m, 900.0);
        assert_eq!(adjusted.steps[0].distance_m, 2900.0);
        assert_eq!(adjusted.distance_m(), 8100.0);
        assert_eq!(adjusted.steps[1].recovery_m, 200.0);
        assert_eq!(adjusted.steps[1].pace_lo_s_km, Some(300));
    }

    #[test]
    fn harder_and_repeated_changes_obey_caps() {
        let baseline = example();
        let harder = nudge(&baseline, &baseline, 300.0, 30.0, Direction::Harder).unwrap();
        assert_eq!(harder.steps[1].distance_m, 1100.0);
        assert_eq!(harder.distance_m(), baseline.distance_m());
        assert!(nudge(&baseline, &baseline, 300.0, 20.0, Direction::Harder).is_none());
        let mut current = baseline.clone();
        for _ in 0..20 {
            if let Some(next) = nudge(&current, &baseline, 300.0, 30.0, Direction::Easier) {
                current = next;
            }
        }
        assert!(work_load(&current, 300.0) >= work_load(&baseline, 300.0) * 0.8 - 0.001);
        assert!(work_load(&current, 300.0) < work_load(&baseline, 300.0) * 0.9);
        assert_eq!(current.distance_m(), baseline.distance_m());
    }

    #[test]
    fn minimum_warmup_can_be_made_repeatedly_harder_at_fixed_mileage() {
        let mut baseline = example();
        baseline.steps[0].distance_m = 1500.0;
        let first = nudge(&baseline, &baseline, 300.0, 30.0, Direction::Harder).unwrap();
        let second = nudge(&first, &baseline, 300.0, 30.0, Direction::Harder).unwrap();
        assert_eq!(first.distance_m(), baseline.distance_m());
        assert_eq!(second.distance_m(), baseline.distance_m());
        assert!(work_load(&first, 300.0) > work_load(&baseline, 300.0));
        assert!(work_load(&second, 300.0) > work_load(&first, 300.0));
        assert!(work_load(&second, 300.0) <= work_load(&baseline, 300.0) * 1.2);
    }

    fn seed() -> (Store, TrainingPlan, Vec<PlanWorkout>) {
        let store = Store::open(
            &std::env::temp_dir().join(format!("difficulty-{}.sqlite", uuid::Uuid::new_v4())),
        )
        .unwrap();
        let today = Local::now().date_naive();
        let request = PlanRequest {
            distance: RaceDistance::K21,
            training_method: TrainingMethod::Norwegian,
            kind: PlanKind::Race,
            start_date: None,
            target_date: today + chrono::Duration::days(112),
            threshold_sec_km: 300.0,
            weekly_frequency: 4,
            longrun_day: 7,
            increase: IncreaseMethod::Moderate,
            current_weekly_km: Some(40.0),
            name: None,
        };
        let (plan, mut workouts) = super::super::engine::generate(&request, 40.0).unwrap();
        let source = workouts
            .iter_mut()
            .find(|workout| workout.kind == WorkoutKind::Tempo)
            .unwrap();
        source.done = true;
        source.date = today;
        store.insert_plan(&plan).unwrap();
        store.insert_workouts(&workouts).unwrap();
        (store, plan, workouts)
    }

    #[test]
    fn unfinished_quality_session_today_is_adjusted() {
        let (store, plan, workouts) = seed();
        let mut upcoming = workouts
            .iter()
            .find(|workout| !workout.done && workout.kind == WorkoutKind::Tempo)
            .unwrap()
            .clone();
        upcoming.date = Local::now().date_naive();
        store.update_workout(&upcoming).unwrap();
        let revision = store.current_plan_rev().unwrap().unwrap();
        apply(&store, &Thresholds::default(), &plan.id, revision, Some(Direction::Easier), None).unwrap();
        let updated = store.plan_workouts(&plan.id).unwrap().into_iter().find(|workout| workout.id == upcoming.id).unwrap();
        assert!(updated.customization.and_then(|value| value.prescription).is_some());
    }

    #[test]
    fn repeated_harder_requests_persist_next_and_later_tempo_sessions() {
        let (store, plan, workouts) = seed();
        store.upsert_health(ProviderKind::Coros, &HealthDaily {
            date: Utc::now().date_naive(), steps: Some(8000), avg_hr: None,
            resting_hr: Some(50), stress_avg: Some(10), sleep_score: Some(100),
            sleep_min: Some(510), deep_min: None, rem_min: None, hrv_avg: Some(70.0),
        }).unwrap();
        store.upsert_training_load(ProviderKind::Coros, &TrainingLoad {
            date: Utc::now().date_naive(), short_term: Some(50.0), long_term: Some(50.0),
            ratio: Some(1.0), comment: None,
        }).unwrap();
        let candidates: Vec<_> = workouts.iter().filter(|workout| {
            !workout.done && workout.kind == WorkoutKind::Tempo && workout.date >= Local::now().date_naive()
        }).collect();
        let earliest = candidates.iter().map(|workout| workout.date).min().unwrap();
        let revision = store.current_plan_rev().unwrap().unwrap();
        let first = apply(&store, &Thresholds::default(), &plan.id, revision, Some(Direction::Harder), None).unwrap();
        assert!(first.changed > 0, "{}", first.reason);
        let after_first = store.plan_workouts(&plan.id).unwrap();
        let revision = store.current_plan_rev().unwrap().unwrap();
        let second = apply(&store, &Thresholds::default(), &plan.id, revision, Some(Direction::Harder), None).unwrap();
        assert!(second.changed > 0, "{}", second.reason);
        let after_second = store.plan_workouts(&plan.id).unwrap();
        let mut increased = Vec::new();
        for before in candidates {
            let first = after_first.iter().find(|workout| workout.id == before.id).unwrap();
            let second = after_second.iter().find(|workout| workout.id == before.id).unwrap();
            let before_load = work_load(&super::super::session::details_for_plan(before, &plan), plan.threshold_sec_km);
            let first_load = work_load(&super::super::session::details_for_plan(first, &plan), plan.threshold_sec_km);
            let second_load = work_load(&super::super::session::details_for_plan(second, &plan), plan.threshold_sec_km);
            if first_load > before_load && second_load > first_load {
                increased.push(before.date);
            }
        }
        assert!(increased.len() >= 2, "Expected repeated increases in upcoming and later tempo sessions: {increased:?}");
        assert!(increased.contains(&earliest), "The next tempo must change after both requests: {increased:?}");
    }

    #[test]
    fn feedback_is_optional_eligible_and_applied_once_at_fixed_mileage() {
        let (store, plan, workouts) = seed();
        let source = workouts.iter().find(|workout| workout.done).unwrap();
        let revision = store.current_plan_rev().unwrap().unwrap();
        let outcome = apply(
            &store,
            &Thresholds::default(),
            &plan.id,
            revision,
            None,
            Some((&source.id, Some(5), false)),
        )
        .unwrap();
        assert!(outcome.changed > 0, "{}", outcome.reason);
        let changed = store.plan_workouts(&plan.id).unwrap();
        for (before, after) in workouts.iter().zip(&changed) {
            assert_eq!(before.distance_m, after.distance_m);
            if let Some(prescription) = after
                .customization
                .as_ref()
                .and_then(|value| value.prescription.as_ref())
            {
                assert_eq!(prescription.current.distance_m(), after.distance_m.unwrap());
                assert!(prescription
                    .current
                    .steps
                    .iter()
                    .all(|step| step.distance_m > 0.0));
            }
        }
        assert!(apply(
            &store,
            &Thresholds::default(),
            &plan.id,
            revision,
            None,
            Some((&source.id, Some(5), false))
        )
        .is_err());
        let revision = store.current_plan_rev().unwrap().unwrap();
        let second = apply(
            &store,
            &Thresholds::default(),
            &plan.id,
            revision,
            None,
            Some((&source.id, Some(1), false)),
        )
        .unwrap();
        assert_eq!(second.changed, 0);
        let revision = store.current_plan_rev().unwrap().unwrap();
        let cleared = apply(
            &store,
            &Thresholds::default(),
            &plan.id,
            revision,
            None,
            Some((&source.id, None, false)),
        )
        .unwrap();
        assert_eq!(cleared.changed, 0);
        let context = context(&store).unwrap().unwrap();
        let saved = context
            .sessions
            .iter()
            .find(|session| session.workout_id == source.id)
            .unwrap();
        assert!(saved.submitted);
        assert_eq!(saved.rating, None);
        assert!(!saved.pending);
    }

    #[test]
    fn ineligible_and_invalid_feedback_never_changes_plan() {
        let (store, plan, workouts) = seed();
        let easy = workouts
            .iter()
            .find(|workout| workout.kind == WorkoutKind::Easy)
            .unwrap();
        let source = workouts.iter().find(|workout| workout.done).unwrap();
        let revision = store.current_plan_rev().unwrap().unwrap();
        assert!(apply(
            &store,
            &Thresholds::default(),
            &plan.id,
            revision,
            None,
            Some((&easy.id, Some(5), false))
        )
        .is_err());
        assert!(apply(
            &store,
            &Thresholds::default(),
            &plan.id,
            revision,
            None,
            Some((&source.id, Some(6), false))
        )
        .is_err());
        assert_eq!(store.current_plan_rev().unwrap(), Some(revision));
        let dismissed = apply(
            &store,
            &Thresholds::default(),
            &plan.id,
            revision,
            None,
            Some((&source.id, None, true)),
        )
        .unwrap();
        assert_eq!(dismissed.changed, 0);
        let context = context(&store).unwrap().unwrap();
        assert!(
            !context
                .sessions
                .iter()
                .find(|session| session.workout_id == source.id)
                .unwrap()
                .submitted
        );
    }

    #[test]
    fn undo_restores_future_prescriptions_without_rearming_feedback() {
        let (store, plan, workouts) = seed();
        let source = workouts.iter().find(|workout| workout.done).unwrap();
        let revision = store.current_plan_rev().unwrap().unwrap();
        assert!(
            apply(
                &store,
                &Thresholds::default(),
                &plan.id,
                revision,
                None,
                Some((&source.id, Some(5), false))
            )
            .unwrap()
            .changed
                > 0
        );
        store.undo_difficulty(&plan.id).unwrap();
        for before in &workouts {
            let after = store
                .plan_workouts(&plan.id)
                .unwrap()
                .into_iter()
                .find(|workout| workout.id == before.id)
                .unwrap();
            assert_eq!(before.distance_m, after.distance_m);
            if !before.done {
                assert!(after.customization.is_none());
            }
        }
        assert!(
            context(&store)
                .unwrap()
                .unwrap()
                .sessions
                .iter()
                .find(|session| session.workout_id == source.id)
                .unwrap()
                .submitted
        );
        assert!(store.undo_difficulty(&plan.id).is_err());
        let revision = store.current_plan_rev().unwrap().unwrap();
        apply(
            &store,
            &Thresholds::default(),
            &plan.id,
            revision,
            Some(Direction::Easier),
            None,
        )
        .unwrap();
        let mut workout = store
            .plan_workouts(&plan.id)
            .unwrap()
            .into_iter()
            .find(|workout| !workout.done)
            .unwrap();
        workout.done = true;
        store.update_workout(&workout).unwrap();
        assert!(store.undo_difficulty(&plan.id).is_err());
    }

    #[test]
    fn long_run_and_benchmark_feedback_are_eligible_but_their_targets_never_change() {
        for kind in [WorkoutKind::LongRun, WorkoutKind::Race] {
            let (store, plan, workouts) = seed();
            let mut source = workouts
                .iter()
                .find(|workout| workout.done)
                .unwrap()
                .clone();
            source.kind = kind;
            source.title = if kind == WorkoutKind::Race {
                "Benchmark: test"
            } else {
                "Long run"
            }
            .into();
            store.update_workout(&source).unwrap();
            assert!(eligible(&source));
            let revision = store.current_plan_rev().unwrap().unwrap();
            assert!(
                apply(
                    &store,
                    &Thresholds::default(),
                    &plan.id,
                    revision,
                    None,
                    Some((&source.id, Some(4), false))
                )
                .unwrap()
                .changed
                    > 0
            );
            let after = store
                .plan_workouts(&plan.id)
                .unwrap()
                .into_iter()
                .find(|workout| workout.id == source.id)
                .unwrap();
            assert_eq!(after.distance_m, source.distance_m);
            assert!(after.customization.unwrap().prescription.is_none());
            source.title = "Target race".into();
            if kind == WorkoutKind::Race {
                assert!(!eligible(&source));
            }
        }
    }

    #[test]
    fn reminders_only_include_new_recordings_and_dismissal_survives_refresh() {
        let (store, plan, workouts) = seed();
        let source = workouts.iter().find(|workout| workout.done).unwrap();
        let started = store.feedback_started_at().unwrap();
        let activity_id = {
            let conn = store.conn.lock().unwrap();
            conn.execute("INSERT INTO activities(provider,provider_record_id,sport,start_ts,duration_s,distance_m) VALUES('coros','feedback-recent','running',?1,1800,6000)", [started]).unwrap();
            conn.last_insert_rowid()
        };
        store.complete_calendar_workout(&source.id, activity_id, "Original prescription").unwrap();
        assert!(context(&store).unwrap().unwrap().sessions.iter().find(|session| session.workout_id == source.id).unwrap().pending);
        let revision = store.current_plan_rev().unwrap().unwrap();
        apply(&store, &Thresholds::default(), &plan.id, revision, None, Some((&source.id, None, true))).unwrap();
        assert!(!context(&store).unwrap().unwrap().sessions.iter().find(|session| session.workout_id == source.id).unwrap().pending);
        let (older, _, workouts) = seed();
        let source = workouts.iter().find(|workout| workout.done).unwrap();
        let before = older.feedback_started_at().unwrap() - 3600;
        let activity_id = {
            let conn = older.conn.lock().unwrap();
            conn.execute("INSERT INTO activities(provider,provider_record_id,sport,start_ts,duration_s,distance_m) VALUES('coros','feedback-older','running',?1,1800,6000)", [before]).unwrap();
            conn.last_insert_rowid()
        };
        older.complete_calendar_workout(&source.id, activity_id, "Original prescription").unwrap();
        assert!(!context(&older).unwrap().unwrap().sessions.iter().find(|session| session.workout_id == source.id).unwrap().pending);
    }

    #[test]
    fn missing_load_blocks_harder_but_saves_the_rating_without_repeat_adjustment() {
        let (store, plan, workouts) = seed();
        let source = workouts.iter().find(|workout| workout.done).unwrap();
        let revision = store.current_plan_rev().unwrap().unwrap();
        let direct = apply(
            &store,
            &Thresholds::default(),
            &plan.id,
            revision,
            Some(Direction::Harder),
            None,
        ).unwrap();
        assert_eq!(direct.changed, 0);
        assert!(direct.reason.contains("pain, recovery or training-load evidence"));
        assert_eq!(store.current_plan_rev().unwrap(), Some(revision));
        let outcome = apply(
            &store,
            &Thresholds::default(),
            &plan.id,
            revision,
            None,
            Some((&source.id, Some(1), false)),
        )
        .unwrap();
        assert_eq!(outcome.changed, 0);
        assert!(outcome.reason.contains("unavailable"));
        let saved = store
            .plan_workouts(&plan.id)
            .unwrap()
            .into_iter()
            .find(|workout| workout.id == source.id)
            .unwrap();
        assert_eq!(saved.customization.unwrap().rating, Some(1));
    }
}
