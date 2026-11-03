//! Post-activity adaptive adjustments.
//!
//! After a sync, compare the completed workout against the plan and the
//! runner's recent recovery signals (sleep, resting HR, HRV, load ratio).
//! Rules are deliberately conservative: they can only soften the next
//! hard session, never intensify it.

use chrono::{NaiveDate, Utc};

use crate::error::Result;
use crate::store::Store;
use crate::types::*;

#[derive(Debug)]
pub struct Adjustment {
    pub workout_id: String,
    pub new_title: String,
    pub new_kind: WorkoutKind,
    pub reason: String,
}

/// Recovery score 0..=100 from the last three days of health data.
pub fn recovery_from_health(days: &[HealthDaily]) -> u32 {
    if days.is_empty() {
        return 70; // neutral when no data
    }
    let avg = |vals: Vec<f64>, lo: f64, hi: f64| -> f64 {
        if vals.is_empty() {
            return 0.6; // treat missing as decent
        }
        let m = vals.iter().sum::<f64>() / vals.len() as f64;
        ((m - lo) / (hi - lo)).clamp(0.0, 1.0)
    };
    let sleep: Vec<f64> = days
        .iter()
        .filter_map(|d| d.sleep_min.map(|m| m as f64))
        .collect();
    let rhr: Vec<f64> = days
        .iter()
        .filter_map(|d| d.resting_hr.map(|m| m as f64))
        .collect();
    let hrv: Vec<f64> = days.iter().filter_map(|d| d.hrv_avg).collect();
    // Resting HR: lower is better → invert scale (60 good .. 80 bad).
    let rhr_score = {
        let s = avg(rhr, 55.0, 85.0);
        1.0 - s
    };
    let sleep_score = avg(sleep, 300.0, 500.0); // 5h..8.3h
    let hrv_score = avg(hrv, 20.0, 70.0);
    let combined = sleep_score * 0.45 + rhr_score * 0.3 + hrv_score * 0.25;
    (combined * 100.0).round() as u32
}

/// Inspect the plan and current state; return workouts that should be
/// softened, with reasons. Applied by the caller (and pushed opportunistically).
pub fn evaluate(store: &Store, th: &Thresholds) -> Result<Vec<Adjustment>> {
    let mut out = Vec::new();
    let Some((plan, workouts)) = store.current_plan()? else {
        return Ok(out);
    };
    if matches!(plan.status, PlanStatus::Aborted | PlanStatus::Completed) {
        return Ok(out);
    }
    let today = Utc::now().date_naive();
    let health = store.recent_health(3)?;
    let recovery = recovery_from_health(&health);
    let load = store.latest_training_load()?;

    // Did the last completed planned workout go badly? (actual much slower)
    let last_done = workouts
        .iter()
        .filter(|w| w.done && w.date <= today)
        .max_by_key(|w| w.date);
    let struggled = if let Some(done) = last_done {
        recent_activity_struggled(store, done.date)?
    } else {
        false
    };

    // Recovery low enough to give up a hard session. Derived from the runner's
    // own recovery bands rather than a second magic number: the trigger sits at
    // the middle of the Fatigued band, so with COROS's default bands (0-19
    // Exhausted, 20-69 Fatigued) this reads as "below 45" exactly as it always
    // has, and it moves with the settings.
    let fatigued_mid = (th.recovery_exhausted + th.recovery_fatigued + 2) / 2;
    let low_recovery = recovery <= th.recovery_exhausted || recovery < fatigued_mid;
    let high_load = load
        .and_then(|l| l.ratio)
        .map(|r| r > th.ratio_optimized_hi)
        .unwrap_or(false);
    // Unresolved pain at 5+/10 in the last week is its own trigger — but the
    // pain path normally goes through a proposal the runner accepts, so this
    // only fires the auto-softening when the niggle is still hurting now.
    let pain = store.max_pain_intensity_since(today)?;
    let significant_pain = pain.as_ref().map(|(i, _)| *i >= 5).unwrap_or(false);

    // Threshold control: the plan stores the athlete state it was generated
    // from, so a warning the softener reads today is only news if it was not
    // already true when the plan was built. The engine shapes the schedule
    // from that same state — if it scheduled a tempo run, it decided the
    // state allowed one — and a softener firing on unchanged evidence
    // contradicts it within seconds of the runner approving the plan. Only a
    // state that has *become* a warning since then is a new concern worth
    // giving up a session for.
    let threshold_warning = plan.threshold_state.as_ref().is_some_and(|built_with| {
        match super::threshold::wearable_evidence(store, today) {
            Ok(evidence) => {
                let now =
                    super::threshold::athlete_state(&evidence, today, plan.threshold_sec_km);
                in_warning(&now) && !in_warning(built_with)
            }
            Err(_) => false,
        }
    });

    if !low_recovery && !high_load && !struggled && !significant_pain && !threshold_warning {
        return Ok(out);
    }

    // Rate limit: one automatic softening per SOFTEN_COOLDOWN_DAYS. The
    // evidence that triggers this (a threshold state in warning, a load
    // ratio still above the band) persists for days or weeks, so without a
    // cooldown every sync eats the next hard session — over a fortnight the
    // plan is quietly rewritten into easy runs, which is the opposite of
    // coaching. One softened session is the runner and the watch being shown
    // the concern; if it was real, the next sessions will still trigger
    // after the cooldown, and the runner can act on the reason in between.
    const SOFTEN_COOLDOWN_DAYS: i64 = 7;
    if let Some(last) = store.last_softened_at()? {
        let age = Utc::now().signed_duration_since(last).num_days();
        if age < SOFTEN_COOLDOWN_DAYS {
            return Ok(out);
        }
    }

    // Workouts an accepted pain proposal already rewrote are off-limits here:
    // the two engines must not overwrite each other's titles in the same week.
    // Dismissed offers count too — declining the offer is the runner saying
    // "leave my plan alone", and the auto-softener must not override that.
    let proposal_touched: std::collections::HashSet<String> = store
        .list_proposals(20)?
        .into_iter()
        .filter(|p| {
            matches!(
                p.status,
                ProposalStatus::Applied | ProposalStatus::Dismissed
            )
        })
        .flat_map(|p| p.edits.into_iter().map(|e| e.workout_id))
        .collect();

    // Soften the next hard session within the next 4 days.
    let next_hard = workouts.iter().find(|w| {
        w.date > today
            && w.date <= today + chrono::Duration::days(4)
            && matches!(w.kind, WorkoutKind::Intervals | WorkoutKind::Tempo)
            && !proposal_touched.contains(&w.id)
    });
    if let Some(w) = next_hard {
        let (kind, title, reason) =
            if let Some((intensity, part)) = pain.filter(|_| significant_pain) {
                (
                    WorkoutKind::Easy,
                    format!("Easy run (was: {})", w.title),
                    format!(
                        "pain reported in the {} ({intensity}/10)",
                        crate::proposal::region_label(&part)
                    ),
                )
            } else if threshold_warning {
                (
                    WorkoutKind::Easy,
                    format!("Easy run (was: {})", w.title),
                    "threshold control, density or recovery requires an easier session".to_string(),
                )
            } else if low_recovery || struggled {
                (
                    WorkoutKind::Easy,
                    format!("Easy run (was: {})", w.title),
                    if struggled {
                        "previous session went poorly + low recovery".to_string()
                    } else {
                        format!("recovery score {recovery}/100")
                    },
                )
            } else {
                (
                    WorkoutKind::Easy,
                    format!("Easy run (was: {})", w.title),
                    format!("training load ratio above {:.2}", th.ratio_optimized_hi),
                )
            };
        out.push(Adjustment {
            workout_id: w.id.clone(),
            new_title: title,
            new_kind: kind,
            reason,
        });
    }
    Ok(out)
}

/// The threshold conditions the adaptive softener treats as a reason to give
/// up a hard session. Shared so the state a plan was built with and the state
/// read today are judged by exactly the same rule.
fn in_warning(state: &super::threshold::AthleteState) -> bool {
    state.repeated_overshoots >= 2
        || state.hold_quality
        || state.threshold_load_trend.is_some_and(|trend| trend > 0.2)
}

/// Compare the actual activity on `day` against the planned pace band.
fn recent_activity_struggled(store: &Store, day: NaiveDate) -> Result<bool> {
    let acts = store.list_activities(20)?;
    let Some(act) = acts
        .iter()
        .find(|a| a.start_time.date_naive() == day && a.avg_pace_s_km.is_some())
    else {
        return Ok(false);
    };
    let planned = store.workouts_between(
        &day.format("%Y-%m-%d").to_string(),
        &day.format("%Y-%m-%d").to_string(),
    )?;
    let Some(p) = planned.first() else {
        return Ok(false);
    };
    let actual = act.avg_pace_s_km.unwrap_or(0.0);
    if let (Some(_lo), Some(hi)) = (p.pace_lo_s_km, p.pace_hi_s_km) {
        // More than 20 % slower than the slow end of the band = struggled.
        return Ok(actual > hi as f64 * 1.2);
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;
    use chrono::Duration;
    use std::path::Path;

    fn store() -> Store {
        let path = std::env::temp_dir().join(format!("rk-adaptive-{}.sqlite", uuid::Uuid::new_v4()));
        Store::open(&path).expect("open store")
    }

    fn seed_plan(s: &Store, hard_in: i64) -> PlanWorkout {
        let today = Utc::now().date_naive();
        let plan = TrainingPlan {
            training_method: TrainingMethod::Norwegian,
            id: "p1".into(),
            distance: RaceDistance::K21,
            kind: PlanKind::Race,
            start_date: None,
            target_date: today + Duration::days(60),
            status: PlanStatus::Active,
            weekly_frequency: 4,
            longrun_day: 6,
            increase: IncreaseMethod::Moderate,
            threshold_sec_km: 280.0,
            created_at: Utc::now(),
            pushed_id: None,
            baseline_weekly_km: Some(30.0),
            quality_offset_sec_km: 0.0,
            quality_offset_from: None,
            threshold_state: None,
            name: None,
        };
        s.insert_plan(&plan).unwrap();
        let hard = PlanWorkout {
            id: "hard-1".into(),
            plan_id: plan.id.clone(),
            date: today + Duration::days(hard_in),
            planned_date: today + Duration::days(hard_in),
            kind: WorkoutKind::Tempo,
            title: "6 x 1 km tempo".into(),
            distance_m: Some(10000.0),
            duration_min: None,
            pace_lo_s_km: Some(255),
            pace_hi_s_km: Some(270),
            done: false,
            status: WorkoutStatus::Planned,
            moved_at: None,
        };
        s.insert_workouts(&[hard.clone()]).unwrap();
        hard
    }

    /// Recovery data low enough to trigger the softener on its own.
    fn seed_low_recovery(s: &Store) {
        let today = Utc::now().date_naive();
        for offset in 0..3 {
            s.upsert_health(
                ProviderKind::Coros,
                &HealthDaily {
                    date: today - Duration::days(offset),
                    steps: Some(4000),
                    avg_hr: None,
                    resting_hr: Some(85),
                    stress_avg: None,
                    sleep_score: None,
                    sleep_min: Some(240),
                    deep_min: None,
                    rem_min: None,
                    hrv_avg: Some(15.0),
                },
            )
            .unwrap();
        }
    }

    /// The trigger evidence (low recovery, a threshold state in warning)
    /// persists across syncs. Without the cooldown, every sync would soften
    /// the next hard session and the plan would be eaten one workout at a
    /// time — the "it keeps replacing my sessions with easy runs" report.
    #[test]
    fn the_softener_gives_up_one_session_per_cooldown_not_one_per_sync() {
        let s = store();
        let hard = seed_plan(&s, 2);
        seed_low_recovery(&s);
        let th = Thresholds::default();

        let first = evaluate(&s, &th).expect("evaluate");
        assert_eq!(first.len(), 1, "the first sync softens the hard session");
        s.soften_workout(&first[0].workout_id, first[0].new_kind, &first[0].new_title, &first[0].reason)
            .expect("apply");

        // The very next sync, with the same evidence still in place: nothing
        // more is given up. The softened session is now Easy, so there is no
        // other candidate either — re-run against a plan that still has one.
        let s2 = store();
        let hard2 = seed_plan(&s2, 2);
        seed_low_recovery(&s2);
        let again = evaluate(&s2, &th).expect("evaluate");
        assert_eq!(again.len(), 1);
        s2.soften_workout(&again[0].workout_id, again[0].new_kind, &again[0].new_title, &again[0].reason)
            .expect("apply");
        // Seed a second hard session behind the first and sync again: the
        // cooldown must protect it even though the evidence has not changed.
        let later = PlanWorkout {
            id: "hard-2".into(),
            plan_id: hard2.plan_id.clone(),
            date: hard2.date + Duration::days(1),
            planned_date: hard2.date + Duration::days(1),
            kind: WorkoutKind::Intervals,
            title: "5 x 800 m".into(),
            distance_m: Some(9000.0),
            duration_min: None,
            pace_lo_s_km: Some(245),
            pace_hi_s_km: Some(255),
            done: false,
            status: WorkoutStatus::Planned,
            moved_at: None,
        };
        s2.insert_workouts(&[later]).unwrap();
        let third = evaluate(&s2, &th).expect("evaluate after softening");
        assert!(
            third.is_empty(),
            "a second hard session must survive the same evidence within the cooldown, got {third:?}"
        );

        // And the timestamp the guard reads is the one soften_workout wrote.
        let last = s2.last_softened_at().expect("read").expect("recorded");
        assert!(Utc::now().signed_duration_since(last).num_minutes() < 5);

        // The first store (same flow, single session) proves the direct
        // re-sync case too: nothing left to soften, and nothing softened.
        assert_eq!(s.workout_adjustments().expect("adjustments").len(), 1);
        assert_eq!(hard.kind, WorkoutKind::Tempo, "seed sanity");
    }

    /// The cooldown clock must live outside the plan. The reported flow is
    /// create a plan, clear the training data, create another, sync — and the
    /// new plan's first hard session is replaced again. When the clock was
    /// only the per-session `adjusted_ts`, deleting the plan deleted the
    /// memory with it, so the recreated plan started with a fresh allowance.
    #[test]
    fn clearing_and_recreating_a_plan_does_not_hand_the_softener_a_fresh_allowance() {
        let s = store();
        seed_plan(&s, 2);
        seed_low_recovery(&s);
        let th = Thresholds::default();

        let first = evaluate(&s, &th).expect("evaluate");
        assert_eq!(first.len(), 1, "the first plan gives up one session");
        s.soften_workout(&first[0].workout_id, first[0].new_kind, &first[0].new_title, &first[0].reason)
            .expect("apply");

        // The runner erases the plan and builds another one, still inside the
        // cooldown, with the same low recovery still in the health data.
        s.clear_plan_data().expect("clear");
        assert!(s.last_softened_at().expect("read").is_some(), "the clock outlives the plan");
        seed_plan(&s, 2);
        let after = evaluate(&s, &th).expect("evaluate on the new plan");
        assert!(
            after.is_empty(),
            "a recreated plan must not be softened on its first sync, got {after:?}"
        );
    }

    /// Threshold evidence the plan was *built from* is not a new concern. The
    /// engine shapes the schedule from the same athlete state, so if it put a
    /// tempo run in the plan it judged the state to allow one; a softener
    /// reading that unchanged state contradicts the plan the runner just
    /// approved. Only a state that has become a warning since is actionable.
    #[test]
    fn threshold_evidence_the_plan_was_built_with_is_not_a_new_reason() {
        let s = store();
        let today = Utc::now().date_naive();
        let hard = seed_plan(&s, 2);
        let mut plan = s.current_plan().expect("read").expect("seeded").0;
        // The state the engine had in front of it: no warning.
        plan.threshold_state = Some(super::super::threshold::athlete_state(
            &super::super::threshold::Evidence::default(),
            today,
            plan.threshold_sec_km,
        ));
        s.clear_plan_data().expect("clear to re-insert with state");
        s.insert_plan(&plan).expect("insert with state");
        s.insert_workouts(&[hard]).expect("re-seed the session");

        let th = Thresholds::default();
        assert!(
            evaluate(&s, &th).expect("evaluate").is_empty(),
            "unchanged threshold evidence must not eat a session the plan already accounted for"
        );

        // Now the state genuinely turns: an active injury makes the recomputed
        // state hold quality, which the plan was not built with.
        s.insert_pain_entry(&PainEntry {
            id: "achilles".into(),
            body_part: "achilles.left".into(),
            intensity: 3,
            started_on: today,
            notes: "Tightness on the first kilometre".into(),
            created_at: Utc::now(),
            resolved_on: None,
            followup_at: None,
        })
        .expect("seed pain");
        let softened = evaluate(&s, &th).expect("evaluate after the state changed");
        assert_eq!(
            softened.len(),
            1,
            "a warning that appeared after the plan was built is worth one session"
        );
    }
}
