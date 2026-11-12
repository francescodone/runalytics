//! Dashboard scoring: training score, recovery, global score,
//! plus the per-day sub-score history behind the readiness sparkline cards.
//! All scores are 0..=100 and derived only from locally stored data.

use chrono::{NaiveDate, Utc};

use crate::error::Result;
use crate::plan::adaptive::recovery_from_health;
use crate::store::Store;
use crate::types::*;

/// Acute:chronic balance factor (0..=1) for a load ratio; the runner's
/// Maintaining-and-up band scores 1.0. Notes when the ratio is missing or well
/// out of range.
fn balance_factor(ratio: Option<f64>, notes: &mut Vec<String>, th: &Thresholds) -> f64 {
    match ratio {
        Some(r) => {
            if r < th.ratio_decreasing_hi - 0.2 {
                0.3
            } else if r < th.ratio_decreasing_hi {
                0.6
            } else if r < th.ratio_maintaining_hi {
                // Resuming/Performance band: coming back up, not yet building.
                0.75
            } else if r <= th.ratio_optimized_hi {
                1.0
            } else {
                notes.push(format!("Load ratio {r:.2} is well above the safe range."));
                0.2
            }
        }
        None => {
            notes.push("No training load data yet.".into());
            0.5
        }
    }
}

/// Training-status verdict for an acute:chronic load ratio, cut at the
/// boundaries the runner configured in Settings. Kept as a pure function of
/// ratio + thresholds so the chart's bands and the headline verdict can never
/// disagree, and so moving one boundary moves both at once.
///
/// `prev` is the previous day's ratio, used exactly where COROS uses it: the
/// Resuming and Performance bands share the same ratio range, and COROS
/// separates them by trend — dropping into the range from Maintaining or
/// above reads as Performance (a taper you are ready to race through),
/// anything else reads as Resuming. Without a previous ratio the band
/// defaults to Resuming, COROS's own fallback reading.
pub fn status_from_ratio(ratio: f64, prev: Option<f64>, th: &Thresholds) -> TrainingStatus {
    if ratio < th.ratio_decreasing_hi {
        TrainingStatus {
            state: "decreasing".into(),
            label: "Decreasing".into(),
            reason: format!(
                "Recent load is {:.2}× your recent average — well below what your body is adapted to, so base fitness is declining.",
                ratio
            ),
            tone: "bad".into(),
            ratio,
        }
    } else if ratio < th.ratio_resuming_hi {
        let tapering = prev
            .map(|p| p >= th.ratio_resuming_hi && ratio < p)
            .unwrap_or(false);
        if tapering {
            TrainingStatus {
                state: "performance".into(),
                label: "Performance".into(),
                reason: format!(
                    "Recent load is {:.2}× your recent average — you have come down from a higher load, and the reduced volume leaves you ready for a high-performance effort.",
                    ratio
                ),
                tone: "good".into(),
                ratio,
            }
        } else {
            TrainingStatus {
                state: "resuming".into(),
                label: "Resuming".into(),
                reason: format!(
                    "Recent load is {:.2}× your recent average — below maintenance, but rising load is rebuilding your adaptation to training.",
                    ratio
                ),
                tone: "warn".into(),
                ratio,
            }
        }
    } else if ratio < th.ratio_maintaining_hi {
        TrainingStatus {
            state: "maintaining".into(),
            label: "Maintaining".into(),
            reason: format!(
                "Recent load is {:.2}× your recent average — moderate load that holds your base fitness steady without building it.",
                ratio
            ),
            tone: "good".into(),
            ratio,
        }
    } else if ratio <= th.ratio_optimized_hi {
        TrainingStatus {
            state: "optimized".into(),
            label: "Optimized".into(),
            reason: format!(
                "Recent load is {:.2}× your recent average — the productive zone where base fitness is increasing.",
                ratio
            ),
            tone: "good".into(),
            ratio,
        }
    } else {
        TrainingStatus {
            state: "excessive".into(),
            label: "Excessive".into(),
            reason: format!(
                "Recent load is {:.2}× your recent average — well above your base. Recent training may be overreaching; ease off or take a down week.",
                ratio
            ),
            tone: "bad".into(),
            ratio,
        }
    }
}

/// The latest verdict, or None when no load ratio has ever synced. `prev` is
/// the previous day's ratio from the load history — see [`status_from_ratio`]
/// for why the shared Resuming/Performance band needs it.
pub fn training_status(
    load: &Option<TrainingLoad>,
    prev: Option<f64>,
    th: &Thresholds,
) -> Option<TrainingStatus> {
    load.as_ref()
        .and_then(|l| l.ratio)
        .map(|r| status_from_ratio(r, prev, th))
}

/// Which recovery band a 0..=100 score falls in, cut at the configured
/// boundaries. COROS's published scale (Exhausted / Fatigued / Normal / Fresh)
/// is the default; the numbers come from the thresholds so the dashboard's
/// verdict and the Settings screen always agree.
pub fn recovery_band(score: u32, th: &Thresholds) -> RecoveryBand {
    if score <= th.recovery_exhausted {
        RecoveryBand {
            state: "exhausted".into(),
            label: "Exhausted".into(),
            tone: "bad".into(),
            min: 0,
            max: th.recovery_exhausted,
        }
    } else if score <= th.recovery_fatigued {
        RecoveryBand {
            state: "fatigued".into(),
            label: "Fatigued".into(),
            tone: "warn".into(),
            min: th.recovery_exhausted + 1,
            max: th.recovery_fatigued,
        }
    } else if score <= th.recovery_normal {
        RecoveryBand {
            state: "normal".into(),
            label: "Normal".into(),
            tone: "good".into(),
            min: th.recovery_fatigued + 1,
            max: th.recovery_normal,
        }
    } else {
        RecoveryBand {
            state: "fresh".into(),
            label: "Fresh".into(),
            tone: "good".into(),
            min: th.recovery_normal + 1,
            max: 100,
        }
    }
}

/// The 7-day Training Load to aim at, on COROS's 3:1 cycle — three weeks of
/// increasing load, then a fourth deliberately lower for recovery.
///
/// The cycle position is read from the runner's own history rather than the
/// calendar: how many complete weeks in a row have increased (newest first in
/// `volumes`). Once that streak reaches `build_weeks`, this week is the down
/// week. The target itself is the chronic load — what the body is adapted to —
/// scaled by the cycle position, so it is None for a freshly-connected watch
/// with nothing to scale from rather than a fabricated number.
pub fn load_recommendation(
    load: &Option<TrainingLoad>,
    volumes: &[(String, f64)],
    th: &Thresholds,
) -> Option<LoadRecommendation> {
    let chronic = load.as_ref().and_then(|l| l.long_term)?;
    if chronic <= 0.0 {
        return None;
    }
    let cycle = th.build_weeks + 1;
    let mut increases = 0u32;
    for pair in volumes.windows(2) {
        if pair[0].1 > pair[1].1 * 1.02 {
            increases += 1;
        } else {
            break;
        }
    }
    let down_week = increases >= th.build_weeks;
    let week_in_cycle = if down_week { cycle } else { increases + 1 };
    let target = if down_week {
        chronic * th.down_week_factor
    } else {
        chronic
    };
    let note = if down_week {
        format!(
            "{} weeks of increasing load in a row — week {cycle} of the cycle is a down week at {:.0}% of your usual load.",
            increases,
            th.down_week_factor * 100.0
        )
    } else if increases == 0 {
        "Hold around your recent average to start the build.".into()
    } else {
        format!(
            "Week {week_in_cycle} of {cycle} — {increases} increasing week(s) so far, keep building.",
        )
    };
    Some(LoadRecommendation {
        target: (target * 10.0).round() / 10.0,
        down_week,
        week_in_cycle,
        cycle_length: cycle,
        note,
    })
}

/// Training score 0..=100: consistency across `volumes` (complete weeks,
/// newest first) weighted against acute:chronic `balance`.
fn training_score_from(volumes: &[(String, f64)], balance: f64, notes: &mut Vec<String>) -> f64 {
    let consistency = if volumes.len() >= 4 {
        // Weeks with at least 60 % of the best recent week count as consistent.
        let best = volumes.iter().map(|(_, d)| *d).fold(0.0f64, f64::max);
        if best <= 0.0 {
            0.0
        } else {
            let good = volumes.iter().filter(|(_, d)| *d >= best * 0.6).count();
            good as f64 / volumes.len() as f64
        }
    } else {
        notes.push("Not enough history for a training score.".into());
        0.0
    };
    (consistency * 0.55 + balance * 0.45) * 100.0
}

pub fn compute_scores(store: &Store, th: &Thresholds) -> Result<DashboardScores> {
    let mut notes = Vec::new();

    // --- training score: consistency + load balance -----------------------
    let volumes = store.weekly_volumes(8)?; // newest first
    let load = store.latest_training_load()?;
    let ratio = load.as_ref().and_then(|l| l.ratio);
    let balance = balance_factor(ratio, &mut notes, th);
    let training_score = training_score_from(&volumes, balance, &mut notes);

    let health = store.recent_health(3)?;
    let recovery = recovery_from_health(&health);

    // --- global score --------------------------------------------------------
    let global = readiness_score(training_score, recovery);

    Ok(DashboardScores {
        training_score: training_score.round() as u32,
        recovery_score: recovery,
        global_score: global.round() as u32,
        notes,
    })
}

/// Per-day history of readiness, training and recovery, oldest→newest, for the
/// dashboard sparkline cards. Re-evaluates the same formulas as
/// [`compute_scores`] as of each day, with the newest point pinned to the headline.
pub fn score_history(
    store: &Store,
    days: i64,
    headline: &DashboardScores,
    th: &Thresholds,
) -> Result<Vec<ScorePoint>> {
    let days = days.clamp(2, 30);
    let today = ScorePoint {
        date: Utc::now().date_naive(),
        readiness: headline.global_score,
        training: headline.training_score,
        recovery: headline.recovery_score,
    };
    let start = today.date - chrono::Duration::days(days - 1);
    // One health query for the whole window; the newest day is included so
    // the recovery series ends where the hero number was measured.
    let health = store.health_between(start, today.date)?;
    let by_day: std::collections::BTreeMap<NaiveDate, HealthDaily> =
        health.iter().cloned().map(|h| (h.date, h)).collect();

    let mut out = Vec::with_capacity(days as usize);
    for back in (0..days).rev() {
        let day = today.date - chrono::Duration::days(back);
        // Recovery: the three days ending on `day`, mirroring recent_health(3)
        // — missing days are simply absent and recovery_from_health averages
        // what arrived.
        let window: Vec<HealthDaily> = (0..3)
            .filter_map(|i| by_day.get(&(day - chrono::Duration::days(i))).cloned())
            .collect();
        let recovery = recovery_from_health(&window);

        // Training as of `day`. The per-day notes would be noise next
        // to the headline's — the throwaway vec just satisfies the API.
        let volumes = store.weekly_volumes_as_of(day, 8)?;
        let ratio = store.training_load_on_or_before(day)?.and_then(|l| l.ratio);
        let mut silent = Vec::new();
        let balance = balance_factor(ratio, &mut silent, th);
        let training = training_score_from(&volumes, balance, &mut silent);

        out.push(ScorePoint {
            date: day,
            readiness: readiness_score(training, recovery).round() as u32,
            training: training.round() as u32,
            recovery,
        });
    }
    if let Some(last) = out.last_mut() {
        *last = today;
    }
    Ok(out)
}

fn readiness_score(training: f64, recovery: u32) -> f64 {
    ((training * 0.45 + recovery as f64 * 0.35) / 0.8).clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn th() -> Thresholds {
        Thresholds::default()
    }

    /// The chart's bands and the headline verdict come from one function, so
    /// the boundaries are asserted here rather than duplicated in the UI.
    /// The band edges are COROS's published Intensity Trend scale:
    /// Decreasing <0.5, Resuming/Performance 0.5-0.79, Maintaining 0.8-0.99,
    /// Optimized 1.0-1.5, Excessive >1.5.
    #[test]
    fn status_boundaries_match_the_coros_scale() {
        let t = th();
        assert_eq!(status_from_ratio(0.2, None, &t).state, "decreasing");
        assert_eq!(status_from_ratio(0.49, None, &t).state, "decreasing");
        assert_eq!(status_from_ratio(0.5, None, &t).state, "resuming");
        assert_eq!(status_from_ratio(0.79, None, &t).state, "resuming");
        assert_eq!(status_from_ratio(0.8, None, &t).state, "maintaining");
        assert_eq!(status_from_ratio(0.99, None, &t).state, "maintaining");
        assert_eq!(status_from_ratio(1.0, None, &t).state, "optimized");
        assert_eq!(status_from_ratio(1.5, None, &t).state, "optimized");
        assert_eq!(status_from_ratio(1.51, None, &t).state, "excessive");
        assert_eq!(status_from_ratio(2.4, None, &t).state, "excessive");
    }

    /// COROS splits the 0.5-0.79 range by trend, not by ratio: dropping into
    /// it from Maintaining reads as Performance (a taper), climbing into it
    /// from Decreasing reads as Resuming. Without history it defaults to
    /// Resuming.
    #[test]
    fn resuming_and_performance_split_on_trend() {
        let t = th();
        // Came down from ≥0.8: Performance.
        assert_eq!(status_from_ratio(0.6, Some(1.1), &t).state, "performance");
        assert_eq!(status_from_ratio(0.6, Some(0.8), &t).state, "performance");
        // Rose from <0.5, or moved sideways inside the band: Resuming.
        assert_eq!(status_from_ratio(0.6, Some(0.4), &t).state, "resuming");
        assert_eq!(status_from_ratio(0.6, Some(0.55), &t).state, "resuming");
        // Falling but not from Maintaining: still Resuming.
        assert_eq!(status_from_ratio(0.6, Some(0.75), &t).state, "resuming");
        // No previous ratio at all: Resuming.
        assert_eq!(status_from_ratio(0.6, None, &t).state, "resuming");
    }

    /// A tone is what the UI colours come from; the risky ends must never
    /// render green.
    #[test]
    fn status_tone_follows_the_zone() {
        let t = th();
        assert_eq!(status_from_ratio(0.3, None, &t).tone, "bad");
        assert_eq!(status_from_ratio(0.6, None, &t).tone, "warn");
        assert_eq!(status_from_ratio(0.6, Some(1.1), &t).tone, "good");
        assert_eq!(status_from_ratio(0.9, None, &t).tone, "good");
        assert_eq!(status_from_ratio(1.2, None, &t).tone, "good");
        assert_eq!(status_from_ratio(1.8, None, &t).tone, "bad");
    }

    /// The whole point of editable thresholds: the same ratio must land in a
    /// different zone once the runner moves the boundary.
    #[test]
    fn custom_thresholds_move_the_zones() {
        let t = Thresholds {
            ratio_decreasing_hi: 0.4,
            ratio_resuming_hi: 0.6,
            ratio_maintaining_hi: 0.9,
            ratio_optimized_hi: 1.2,
            ..Thresholds::default()
        };
        assert_eq!(status_from_ratio(0.35, None, &t).state, "decreasing");
        assert_eq!(status_from_ratio(0.5, None, &t).state, "resuming");
        assert_eq!(status_from_ratio(0.75, None, &t).state, "maintaining");
        assert_eq!(status_from_ratio(1.0, None, &t).state, "optimized");
        assert_eq!(status_from_ratio(1.3, None, &t).state, "excessive");
    }

    /// Recovery bands follow COROS's published scale by default and follow the
    /// runner's numbers once edited.
    #[test]
    fn recovery_bands_follow_the_thresholds() {
        let t = th();
        assert_eq!(recovery_band(10, &t).state, "exhausted");
        assert_eq!(recovery_band(19, &t).state, "exhausted");
        assert_eq!(recovery_band(20, &t).state, "fatigued");
        assert_eq!(recovery_band(69, &t).state, "fatigued");
        assert_eq!(recovery_band(70, &t).state, "normal");
        assert_eq!(recovery_band(89, &t).state, "normal");
        assert_eq!(recovery_band(90, &t).state, "fresh");
        assert_eq!(recovery_band(100, &t).state, "fresh");
        // Bands stay contiguous: every score falls in exactly one.
        for s in 0..=100u32 {
            let b = recovery_band(s, &t);
            assert!(
                s >= b.min && s <= b.max,
                "{s} outside {}..={}",
                b.min,
                b.max
            );
        }
        let edited = Thresholds {
            recovery_exhausted: 29,
            recovery_fatigued: 79,
            recovery_normal: 94,
            ..Thresholds::default()
        };
        assert_eq!(recovery_band(25, &edited).state, "exhausted");
        assert_eq!(recovery_band(75, &edited).state, "fatigued");
        assert_eq!(recovery_band(99, &edited).state, "fresh");
    }

    /// Out-of-order settings must not be able to empty a band: normalisation
    /// repairs the ordering instead.
    #[test]
    fn thresholds_normalisation_repairs_bad_input() {
        let bad = Thresholds {
            ratio_decreasing_hi: 2.0,
            ratio_resuming_hi: 0.1,
            ratio_maintaining_hi: 0.2,
            ratio_optimized_hi: 0.3,
            recovery_exhausted: 80,
            recovery_fatigued: 10,
            recovery_normal: 5,
            build_weeks: 0,
            down_week_factor: 5.0,
            ..Thresholds::default()
        }
        .normalized();
        assert!(bad.ratio_decreasing_hi < bad.ratio_resuming_hi);
        assert!(bad.ratio_resuming_hi < bad.ratio_maintaining_hi);
        assert!(bad.ratio_maintaining_hi < bad.ratio_optimized_hi);
        assert!(bad.recovery_exhausted < bad.recovery_fatigued);
        assert!(bad.recovery_fatigued < bad.recovery_normal);
        assert_eq!(bad.build_weeks, 1);
        assert!((bad.down_week_factor - 1.0).abs() < f64::EPSILON);
    }

    /// A 3:1 cycle: three increasing weeks then a down week at the configured
    /// fraction of chronic load. Volumes are newest-first, like the store's.
    #[test]
    fn load_recommendation_holds_a_down_week_every_fourth() {
        let t = th();
        let load = Some(TrainingLoad {
            date: Utc::now().date_naive(),
            short_term: Some(60.0),
            long_term: Some(50.0),
            ratio: Some(1.2),
            comment: None,
        });
        let wk = |v: f64| ("2026-01-05".to_string(), v);
        // Flat history: no increasing streak, so the target is the chronic load.
        let flat = vec![wk(40.0), wk(40.0), wk(40.0), wk(40.0)];
        let r = load_recommendation(&load, &flat, &t).expect("recommendation");
        assert!(!r.down_week);
        assert_eq!(r.week_in_cycle, 1);
        assert!((r.target - 50.0).abs() < 0.1, "target {}", r.target);

        // Three consecutive increases -> this is the fourth week: a down week.
        let build = vec![wk(52.0), wk(48.0), wk(44.0), wk(40.0)];
        let r = load_recommendation(&load, &build, &t).expect("recommendation");
        assert!(r.down_week, "three increases must trigger the down week");
        assert_eq!(r.cycle_length, 4);
        assert!(
            (r.target - 35.0).abs() < 0.1,
            "50 * 0.7 = 35, got {}",
            r.target
        );

        // No chronic load -> no recommendation, rather than a made-up number.
        assert!(load_recommendation(&None, &build, &t).is_none());
    }

    /// No synced load means no verdict — the chart shows a prompt, not a
    /// fabricated "optimal".
    #[test]
    fn status_absent_without_a_ratio() {
        let t = th();
        assert!(training_status(&None, None, &t).is_none());
        let no_ratio = TrainingLoad {
            date: Utc::now().date_naive(),
            short_term: Some(40.0),
            long_term: Some(0.0),
            ratio: None,
            comment: None,
        };
        assert!(training_status(&Some(no_ratio), None, &t).is_none());
    }

    #[test]
    fn readiness_uses_only_training_and_recovery() {
        assert_eq!(readiness_score(80.0, 40), 62.5);
        assert_eq!(readiness_score(100.0, 100), 100.0);
        assert_eq!(readiness_score(0.0, 0), 0.0);
    }

    #[test]
    fn readiness_and_history_do_not_blend_pain_or_serialize_risk() {
        let path = std::env::temp_dir().join(format!("readiness-{}.sqlite", uuid::Uuid::new_v4()));
        let store = Store::open(&path).unwrap();
        let thresholds = th();
        let before = compute_scores(&store, &thresholds).unwrap();
        let history_before = score_history(&store, 7, &before, &thresholds).unwrap();
        store.insert_pain_entry(&crate::types::PainEntry {
            id: "reported-pain".into(),
            body_part: "knee.left".into(),
            intensity: 9,
            started_on: Utc::now().date_naive() - chrono::Duration::days(3),
            notes: String::new(),
            created_at: Utc::now(),
            resolved_on: None,
            followup_at: None,
        }).unwrap();
        let after = compute_scores(&store, &thresholds).unwrap();
        let history_after = score_history(&store, 7, &after, &thresholds).unwrap();
        let serialized = serde_json::to_value(&after).unwrap();
        assert_eq!(serde_json::to_value(&before).unwrap(), serialized);
        assert!(serialized.get("injuryRisk").is_none());
        assert_eq!(serde_json::to_value(&history_before).unwrap(), serde_json::to_value(&history_after).unwrap());
        for point in serde_json::to_value(&history_after).unwrap().as_array().unwrap() {
            assert!(point.get("risk").is_none());
            assert!(point.get("readiness").is_some());
        }
        assert_eq!(store.latest_pain().unwrap().unwrap().intensity, 9);
    }

}
