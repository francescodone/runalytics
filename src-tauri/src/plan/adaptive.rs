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

    if !low_recovery && !high_load && !struggled && !significant_pain {
        return Ok(out);
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
