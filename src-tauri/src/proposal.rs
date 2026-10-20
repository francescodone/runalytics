//! Deterministic plan-change proposals for logged pain.
//!
//! When the runner logs pain, the app offers a concrete, reviewable change to
//! the training plan aimed at making the pain settle — and asks before
//! applying it. This module is pure: it reads the store and returns an offer,
//! never mutates anything. The assistant (assistant.rs) may *explain* a
//! proposal but never authors one, so the edit set stays auditable and works
//! identically with the LLM switched off.
//!
//! Posture mirrors plan::adaptive: an offer may only reduce load (softer kind,
//! shorter distance), never increase it, and never touches a race-day workout.

use chrono::{Datelike, NaiveDate, Utc};

use crate::error::Result;
use crate::store::Store;
use crate::types::*;

/// How far ahead an offer may rewrite the plan.
const HORIZON_DAYS: i64 = 7;
/// An identical dismissal suppresses the same offer for this long.
const DISMISS_SUPPRESSION_DAYS: i64 = 7;

/// Severity tier chosen from the reported intensity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// 5..=6 — downgrade the next hard session, trim the week by 25 %.
    Moderate,
    /// 7..=10 — two rest days, hard sessions downgraded, week cut by 40 %.
    Severe,
    /// 0..=4 — downgrade hard sessions only; volume untouched (monitor).
    Monitor,
}

impl Tier {
    fn from_intensity(i: u32) -> Tier {
        if i >= 7 {
            Tier::Severe
        } else if i >= 5 {
            Tier::Moderate
        } else {
            Tier::Monitor
        }
    }
    fn key(self) -> &'static str {
        match self {
            Tier::Severe => "deload40",
            Tier::Moderate => "deload25",
            Tier::Monitor => "soften",
        }
    }
    /// Weekly-volume multiplier applied to the next 7 days (1.0 = no cut).
    fn volume_factor(self) -> f64 {
        match self {
            Tier::Severe => 0.6,
            Tier::Moderate => 0.75,
            Tier::Monitor => 1.0,
        }
    }
    /// How many of the next days become forced rest (0 = none).
    fn rest_days(self) -> i64 {
        match self {
            Tier::Severe => 2,
            _ => 0,
        }
    }
}

fn is_hard(k: WorkoutKind) -> bool {
    matches!(k, WorkoutKind::Intervals | WorkoutKind::Tempo)
}

/// Monday of the week containing `d` — the week anchor inside a signature, so
/// re-logging the same pain in the same week collapses into one offer.
fn monday_of(d: NaiveDate) -> NaiveDate {
    d - chrono::Duration::days(d.weekday().num_days_from_monday() as i64)
}

/// Build the offer for a freshly logged pain entry, if one is warranted.
/// Returns `None` when there is no active plan, nothing scheduled in the
/// horizon, the offer would change nothing, or an identical offer was recently
/// dismissed (the suppression rule that keeps the app from nagging).
pub fn build(store: &Store, entry: &PainEntry) -> Result<Option<PlanProposal>> {
    let Some((plan, workouts)) = store.current_plan()? else {
        return Ok(None); // nothing to propose against; the journal still logs
    };
    if matches!(plan.status, PlanStatus::Aborted | PlanStatus::Completed) {
        return Ok(None);
    }

    let today = Utc::now().date_naive();
    let horizon_end = today + chrono::Duration::days(HORIZON_DAYS);
    let tier = Tier::from_intensity(entry.intensity);

    // Suppression: an identical offer dismissed recently stays dismissed
    // unless the runner reports worse pain (higher intensity) since.
    let week_anchor = monday_of(today).format("%Y-%m-%d").to_string();
    let signature = format!("{}:{}:{}", tier.key(), entry.body_part, week_anchor);
    if let Some(dismissed) = store.last_dismissed(&signature)? {
        let older_than_window =
            Utc::now() - dismissed > chrono::Duration::days(DISMISS_SUPPRESSION_DAYS);
        if !older_than_window && entry.created_at <= dismissed {
            return Ok(None);
        }
    }
    // Never stack a second unanswered offer on top of the first.
    if store.has_pending_signature_like(&entry.body_part, &week_anchor)? {
        return Ok(None);
    }

    let upcoming: Vec<&PlanWorkout> = workouts
        .iter()
        .filter(|w| !w.done && w.date > today && w.date <= horizon_end)
        .collect();
    if upcoming.is_empty() {
        return Ok(None);
    }

    let mut edits: Vec<WorkoutEdit> = Vec::new();
    let mut rest_count = 0u32;
    let mut soften_count = 0u32;
    let mut trimmed_km = 0.0f64;
    let factor = tier.volume_factor();

    for w in upcoming.iter() {
        // Race day is never rewritten — that is a decision only the runner
        // makes, and the summary can advise on it instead.
        if w.kind == WorkoutKind::Race {
            continue;
        }
        let forced_rest = w.date <= today + chrono::Duration::days(tier.rest_days());
        // Every tier downgrades hard sessions; Severe additionally forces the
        // first days to rest, and Severe/Moderate trim the week's volume.
        let downgrade = forced_rest || is_hard(w.kind);

        let (kind, title) = if forced_rest {
            (WorkoutKind::Rest, format!("Rest (was: {})", w.title))
        } else if downgrade {
            (WorkoutKind::Easy, format!("Easy run (was: {})", w.title))
        } else {
            (w.kind, w.title.clone())
        };

        // Volume trim: only Severe/Moderate, only on workouts that still run.
        let (dist, dur) = if kind != WorkoutKind::Rest && factor < 1.0 {
            let nd = w.distance_m.map(|d| (d * factor / 500.0).round() * 500.0);
            let nu = w.duration_min.map(|d| (d * factor / 5.0).round() * 5.0);
            if let (Some(old), Some(new)) = (w.distance_m, nd) {
                trimmed_km += (old - new).max(0.0) / 1000.0;
            }
            (nd, nu)
        } else {
            (w.distance_m, w.duration_min)
        };

        // Pace zones: a downgraded session loses its pace targets; a trimmed
        // session keeps them (still an easy/tempo zone, just shorter).
        let (lo, hi) = if kind == WorkoutKind::Rest || is_hard(w.kind) && kind == WorkoutKind::Easy
        {
            (None, None)
        } else {
            (w.pace_lo_s_km, w.pace_hi_s_km)
        };

        let changed = kind != w.kind
            || title != w.title
            || dist != w.distance_m
            || dur != w.duration_min
            || lo != w.pace_lo_s_km
            || hi != w.pace_hi_s_km;
        if !changed {
            continue;
        }
        if forced_rest {
            rest_count += 1;
        } else if kind == WorkoutKind::Easy && w.kind != WorkoutKind::Easy {
            soften_count += 1;
        }
        edits.push(WorkoutEdit {
            workout_id: w.id.clone(),
            kind,
            title,
            distance_m: if kind == WorkoutKind::Rest {
                None
            } else {
                dist
            },
            duration_min: if kind == WorkoutKind::Rest { None } else { dur },
            pace_lo_s_km: if kind == WorkoutKind::Rest { None } else { lo },
            pace_hi_s_km: if kind == WorkoutKind::Rest { None } else { hi },
            date: Some(w.date),
            old_kind: Some(w.kind),
            old_title: Some(w.title.clone()),
            old_distance_m: w.distance_m,
        });
    }

    if edits.is_empty() {
        return Ok(None);
    }

    // Human-readable headline + reason quoting the runner's own numbers.
    let region = region_label(&entry.body_part);
    let days_hurting = (today - entry.started_on).num_days().max(0);
    let mut parts: Vec<String> = Vec::new();
    if rest_count > 0 {
        parts.push(format!(
            "{rest_count} rest day{s}",
            s = if rest_count == 1 { "" } else { "s" }
        ));
    }
    if soften_count > 0 {
        parts.push(format!(
            "{soften_count} hard session{s} downgraded to easy",
            s = if soften_count == 1 { "" } else { "s" }
        ));
    }
    if trimmed_km > 0.5 {
        parts.push(format!("the week trimmed by ~{trimmed_km:.0} km"));
    }
    let summary = format!(
        "Give your {} a chance to settle: {}.",
        region,
        parts.join(", ")
    );

    let mut reason = format!(
        "{intensity}/10 {region} pain, hurting since {started} ({days} day{ds}).",
        intensity = entry.intensity,
        days = days_hurting,
        started = entry.started_on.format("%b %e"),
        ds = if days_hurting == 1 { "" } else { "s" }
    );
    let load = store.latest_training_load()?;
    if let Some(r) = load.as_ref().and_then(|l| l.ratio) {
        reason.push_str(&format!(" Load ratio is {r:.2}."));
    }
    let last7 = store.distance_window(7, 0)? / 1000.0;
    let prior = store.distance_window(28, 7)? / 1000.0 / 3.0;
    if prior > 1.0 {
        reason.push_str(&format!(
            " Last 7 days: {last7:.0} km vs a {prior:.0} km baseline."
        ));
    }
    let days_to_race = (plan.target_date - today).num_days();
    if (0..14).contains(&days_to_race) && entry.intensity >= 7 {
        // Only a build that ends somewhere has something to withdraw from. A
        // maintenance block's end date is just the end of a holding period, so
        // warning about it would invent a decision the runner isn't facing.
        let milestone = match plan.kind {
            PlanKind::Race => "Your race",
            PlanKind::Improvement => "Your time trial",
            PlanKind::Maintenance => "",
        };
        if !milestone.is_empty() {
            reason.push_str(&format!(
                " {milestone} is in {days_to_race} days — with pain this strong, consider skipping it and see a clinician."
            ));
        }
    }

    Ok(Some(PlanProposal {
        id: uuid::Uuid::new_v4().to_string(),
        body_part: entry.body_part.clone(),
        source: ProposalSource::Pain,
        signature,
        summary,
        reason,
        edits,
        plan_quality_offset_sec_km: None,
        plan_quality_offset_from: None,
        status: ProposalStatus::Pending,
        created_at: Utc::now(),
        acted_at: None,
    }))
}

/// Human label for a body-region key. Mirrors BODY_REGIONS in
/// src/lib/bodymap.ts; unknown keys render as-is so legacy or `other` values
/// never show up blank.
pub fn region_label(key: &str) -> String {
    let (base, side) = match key.split_once('.') {
        Some((b, s @ ("left" | "right"))) => (b, Some(s)),
        _ => (key, None),
    };
    let name = match base {
        "neck" => "neck",
        "shoulder" => "shoulder",
        "chest" => "chest",
        "forearm_wrist" => "wrist/forearm",
        "upper_back" => "upper back",
        "lower_back" => "lower back",
        "quad" => "quad",
        "knee" => "knee",
        "shin" => "shin",
        "glute_hip" => "hip/glute",
        "hamstring" => "hamstring",
        "calf" => "calf",
        "achilles" => "Achilles",
        "ankle_foot" => "ankle/foot",
        "other" => "other area",
        _ => return key.replace('_', " "),
    };
    match side {
        Some(s) => format!("{s} {name}"),
        None => name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    fn store() -> Store {
        let path = std::env::temp_dir().join(format!("rk-prop-{}.sqlite", uuid::Uuid::new_v4()));
        Store::open(&path).expect("open store")
    }

    fn entry(intensity: u32, part: &str) -> PainEntry {
        PainEntry {
            id: uuid::Uuid::new_v4().to_string(),
            body_part: part.into(),
            intensity,
            started_on: Utc::now().date_naive() - chrono::Duration::days(2),
            notes: String::new(),
            created_at: Utc::now(),
            resolved_on: None,
            followup_at: None,
        }
    }

    fn plan_with_workouts(s: &Store, days: &[(i64, WorkoutKind, f64)]) {
        let today = Utc::now().date_naive();
        let plan = TrainingPlan {
            id: uuid::Uuid::new_v4().to_string(),
            distance: RaceDistance::K21,
            kind: PlanKind::Race,
            target_date: today + chrono::Duration::days(60),
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
        };
        s.insert_plan(&plan).unwrap();
        let ws = days
            .iter()
            .map(|(off, kind, km)| PlanWorkout {
                id: uuid::Uuid::new_v4().to_string(),
                plan_id: plan.id.clone(),
                date: today + chrono::Duration::days(*off),
                kind: *kind,
                title: format!("{kind:?} run"),
                distance_m: Some(*km * 1000.0),
                duration_min: Some(*km * 6.0),
                pace_lo_s_km: if is_hard(*kind) { Some(270) } else { None },
                pace_hi_s_km: if is_hard(*kind) { Some(290) } else { None },
                done: false,
            })
            .collect::<Vec<_>>();
        s.insert_workouts(&ws).unwrap();
    }

    #[test]
    fn no_plan_no_proposal() {
        let s = store();
        assert!(build(&s, &entry(8, "knee.left")).unwrap().is_none());
    }

    #[test]
    fn severe_tier_adds_rest_and_cuts_volume_never_increases() {
        let s = store();
        plan_with_workouts(
            &s,
            &[
                (1, WorkoutKind::Intervals, 10.0),
                (3, WorkoutKind::Easy, 8.0),
                (5, WorkoutKind::Tempo, 12.0),
            ],
        );
        let p = build(&s, &entry(8, "knee.left"))
            .unwrap()
            .expect("severe offer");
        let (_, workouts) = s.current_plan().unwrap().unwrap();
        for e in &p.edits {
            let w = workouts.iter().find(|w| w.id == e.workout_id).unwrap();
            assert_ne!(
                e.kind,
                WorkoutKind::Intervals,
                "hard sessions must not stay hard"
            );
            assert_ne!(e.kind, WorkoutKind::Tempo);
            if let (Some(old), Some(new)) = (w.distance_m, e.distance_m) {
                assert!(new <= old, "an offer may never increase distance");
            }
        }
        assert!(p.edits.iter().any(|e| e.kind == WorkoutKind::Rest));
        assert!(p.summary.contains("rest day"));
    }

    #[test]
    fn monitor_tier_keeps_volume() {
        let s = store();
        plan_with_workouts(&s, &[(2, WorkoutKind::Tempo, 12.0)]);
        let p = build(&s, &entry(3, "calf.right"))
            .unwrap()
            .expect("soften offer");
        assert_eq!(p.edits.len(), 1);
        let e = &p.edits[0];
        assert_eq!(e.kind, WorkoutKind::Easy);
        assert_eq!(e.distance_m, Some(12_000.0), "monitor tier trims nothing");
        assert!(!p.summary.contains("rest"));
    }

    #[test]
    fn race_day_is_never_touched() {
        let s = store();
        plan_with_workouts(&s, &[(3, WorkoutKind::Race, 21.0)]);
        assert!(
            build(&s, &entry(9, "knee.left")).unwrap().is_none(),
            "only a race in range -> no edits"
        );
    }

    #[test]
    fn pending_offer_suppresses_duplicates() {
        let s = store();
        plan_with_workouts(&s, &[(1, WorkoutKind::Intervals, 10.0)]);
        let p = build(&s, &entry(8, "knee.left"))
            .unwrap()
            .expect("first offer");
        s.insert_proposal(&p).unwrap();
        assert!(
            build(&s, &entry(9, "knee.left")).unwrap().is_none(),
            "no stacking"
        );
    }

    #[test]
    fn dismissed_offer_suppresses_unless_worse() {
        let s = store();
        plan_with_workouts(&s, &[(1, WorkoutKind::Intervals, 10.0)]);
        let p = build(&s, &entry(5, "shin.right")).unwrap().expect("offer");
        let sig = p.signature.clone();
        s.insert_proposal(&p).unwrap();
        s.set_proposal_status(&p.id, ProposalStatus::Dismissed, Some(Utc::now()), None)
            .unwrap();
        // Same intensity, logged before the dismissal -> suppressed.
        let mut same = entry(5, "shin.right");
        same.created_at = Utc::now() - chrono::Duration::hours(2);
        assert!(build(&s, &same).unwrap().is_none());
        // Worse pain reported after the dismissal -> offer again.
        let mut worse = entry(8, "shin.right");
        worse.created_at = Utc::now();
        let p2 = build(&s, &worse)
            .unwrap()
            .expect("escalation re-opens the offer");
        assert_ne!(p2.signature, sig, "escalation is a different tier");
    }

    #[test]
    fn region_labels_read_like_prose() {
        assert_eq!(region_label("knee.left"), "left knee");
        assert_eq!(region_label("lower_back"), "lower back");
        assert_eq!(region_label("achilles.right"), "right Achilles");
        assert_eq!(region_label("weird_new_thing"), "weird new thing");
    }
}
