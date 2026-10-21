//! Dashboard scoring: training score, injury risk, recovery, global score,
//! plus the per-day sub-score history behind the readiness sparkline cards.
//! All scores are 0..=100 and derived only from locally stored data.

use chrono::{NaiveDate, Utc};

use crate::error::Result;
use crate::plan::adaptive::recovery_from_health;
use crate::store::Store;
use crate::types::*;

/// Mean of the values that actually arrived, or None when the provider sent
/// nothing for the window. Summing `filter_map` results over `rows.len()`
/// divides by the row count instead of the observation count, so a field the
/// provider never returns averages to 0 — and 0 is the *worst* reading for
/// sleep score and stress, which silently inflates injury risk and prints
/// "0/100 avg" on the dashboard. Callers skip the factor when this is None.
fn mean_observed(mut vals: impl Iterator<Item = f64>) -> Option<f64> {
    let (mut sum, mut n) = (0.0f64, 0usize);
    for v in vals.by_ref() {
        sum += v;
        n += 1;
    }
    if n == 0 {
        None
    } else {
        Some(sum / n as f64)
    }
}

/// How hard one run was, as a fraction of threshold effort: 1.0 means the
/// whole session sat at lactate threshold. Pace against the stored threshold
/// first — the sharpest signal the sync carries; heart rate against a generic
/// 190 bpm ceiling as a fallback (0 at 124 bpm, 1.0 at 172); a run with
/// neither signal counts as 0.6 — evidence of *some* run, no evidence it was
/// hard.
fn run_intensity(run: &Activity, threshold_s_km: Option<f64>) -> f64 {
    if let (Some(t), Some(p)) = (threshold_s_km, run.avg_pace_s_km) {
        if p > 0.0 {
            return (t / p).clamp(0.0, 1.3);
        }
    }
    match run.avg_hr {
        Some(hr) => ((hr as f64 - 124.0) / 48.0).clamp(0.0, 1.3),
        None => 0.6,
    }
}

/// Pressure from the runs of the last seven days: effort-weighted minutes
/// (time x intensity^2, the same currency the plan's session stress uses), a
/// penalty for hard days stacked back-to-back, and an interaction term when
/// hard sessions land on top of reported pain — running hard on an injury is
/// worse than either signal alone.
/// Two components: the *session's own* effort counts with no floor (an easy
/// jog adds a little, a threshold session adds a lot — this is what makes
/// every banked run move the risk, so every activity has a delta to show),
/// and the *week's* pressure beyond a comfortable 90 hard-minutes, which is
/// where the pile-up actually starts to hurt. `self_id` picks the run being
/// explained out of `runs` (None = no single session is the subject, so no
/// self term — used when measuring the week as if that run had not happened).
/// Returns the raw risk contribution and a human-readable detail.
fn recent_sessions_pressure(
    runs: &[Activity],
    threshold_s_km: Option<f64>,
    pain: Option<u32>,
    self_id: Option<i64>,
) -> (f64, String) {
    let mut hard = 0.0f64;
    let mut self_units = 0.0f64;
    let mut hard_days: Vec<(NaiveDate, bool)> = Vec::new();
    for run in runs {
        let minutes = run.duration_s / 60.0;
        let intensity = run_intensity(run, threshold_s_km);
        let units = minutes * intensity * intensity;
        hard += units;
        if self_id.is_some_and(|id| id == run.id) {
            self_units = units;
        }
        // A "hard day" is a session that was genuinely pushed (40 hard-
        // minutes: an hour at threshold, not an hour of easy volume).
        if units >= 40.0 {
            hard_days.push((run.start_time.date_naive(), self_id.is_some_and(|id| id == run.id)));
        }
    }
    hard_days.sort();
    let stacked = hard_days
        .windows(2)
        .filter(|w| (w[1].0 - w[0].0).num_days() == 1)
        .count();
    // The session's own effort: an hour at threshold (60 units) is +0.12 raw
    // (~+7 risk points), half an hour of easy jogging ~+1 point. The week's
    // excess over comfort adds on top, and stacked hard days add 0.15 each.
    let mut raw = self_units * 0.002 + (hard - 90.0).max(0.0) * 0.01 + stacked as f64 * 0.15;
    let mut detail = if runs.is_empty() {
        String::from("no runs in the last 7 days")
    } else {
        format!("{hard:.0} effort-weighted run minutes in the last 7 days")
    };
    if stacked > 0 {
        detail.push_str(&format!(" · {stacked} back-to-back hard-day pair(s)"));
    }
    if let Some(p) = pain {
        let extra = (p as f64 / 10.0) * 0.15 * (hard / 180.0).clamp(0.0, 1.0);
        if extra > 0.001 {
            raw += extra;
            detail.push_str(" · hard sessions on top of reported pain");
        }
    }
    (raw, detail)
}

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

/// Injury risk 0..=100 as of `day`, from the same ingredients the headline
/// uses: load ratio, volume spike across `volumes` (newest first), recovery,
/// and the worst pain still hurting on that day.
fn injury_risk_from(
    store: &Store,
    day: NaiveDate,
    ratio: Option<f64>,
    volumes: &[(String, f64)],
    recovery: u32,
    notes: &mut Vec<String>,
    th: &Thresholds,
) -> Result<f64> {
    let mut risk = 10.0f64;
    if let Some(r) = ratio {
        risk += ((r - th.ratio_optimized_hi).max(0.0)) * 90.0;
    }
    // Rapid volume spike: last week vs mean of previous three.
    if volumes.len() >= 4 {
        let last = volumes[0].1;
        let prior: f64 = volumes[1..4].iter().map(|(_, d)| *d).sum::<f64>() / 3.0;
        if prior > 1.0 {
            let spike = last / prior;
            if spike > 1.3 {
                risk += (spike - 1.3) * 120.0;
                notes.push(format!(
                    "Last week's volume was {:+.0}% vs baseline.",
                    (spike - 1.0) * 100.0
                ));
            }
        }
    }
    risk += (100 - recovery) as f64 * 0.25;
    // Reported pain still hurting on `day`: a 0..=10 self-report maps onto up
    // to +30 risk points. Resolved episodes are excluded by the store query,
    // so a long-healed injury cannot inflate the score forever.
    if let Some((intensity, part)) = store.max_pain_intensity_since(day)? {
        risk += intensity as f64 * 3.0;
        notes.push(format!(
            "Pain reported in the {} ({intensity}/10).",
            crate::proposal::region_label(&part)
        ));
    }
    Ok(risk.clamp(0.0, 100.0))
}

pub fn compute_scores(store: &Store, th: &Thresholds) -> Result<DashboardScores> {
    let mut notes = Vec::new();

    // --- training score: consistency + load balance -----------------------
    let volumes = store.weekly_volumes(8)?; // newest first
    let load = store.latest_training_load()?;
    let ratio = load.as_ref().and_then(|l| l.ratio);
    let balance = balance_factor(ratio, &mut notes, th);
    let training_score = training_score_from(&volumes, balance, &mut notes);

    // --- recovery + injury risk ----------------------------------------------
    let health = store.recent_health(3)?;
    let recovery = recovery_from_health(&health);
    let injury_risk = injury_risk_from(
        store,
        Utc::now().date_naive(),
        ratio,
        &volumes,
        recovery,
        &mut notes,
        th,
    )?;

    // --- global score --------------------------------------------------------
    let global = (training_score * 0.45 + recovery as f64 * 0.35 + (100.0 - injury_risk) * 0.20)
        .clamp(0.0, 100.0);

    Ok(DashboardScores {
        training_score: training_score.round() as u32,
        injury_risk: injury_risk.round() as u32,
        recovery_score: recovery,
        global_score: global.round() as u32,
        notes,
    })
}

/// Per-day history of the three readiness sub-scores, oldest→newest, for the
/// dashboard sparkline cards. Re-evaluates the same formulas as
/// [`compute_scores`] as of each day. The risk series is the point-in-time
/// formula *before* the dashboard command blends in the range-aware sum: the
/// blend depends on the selected time range, and a trend line must not change
/// shape when the user switches ranges.
///
/// `today` pins the newest point to the headline numbers the hero shows, so a
/// card's big figure always equals its own chart's last sample — including
/// the blended risk and whatever health rows had synced when the headline was
/// computed.
pub fn score_history(
    store: &Store,
    days: i64,
    headline: &DashboardScores,
    th: &Thresholds,
) -> Result<Vec<ScorePoint>> {
    let days = days.clamp(2, 30);
    let today = ScorePoint {
        date: Utc::now().date_naive(),
        training: headline.training_score,
        recovery: headline.recovery_score,
        risk: headline.injury_risk,
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

        // Training + risk as of `day`. The per-day notes would be noise next
        // to the headline's — the throwaway vec just satisfies the API.
        let volumes = store.weekly_volumes_as_of(day, 8)?;
        let ratio = store.training_load_on_or_before(day)?.and_then(|l| l.ratio);
        let mut silent = Vec::new();
        let balance = balance_factor(ratio, &mut silent, th);
        let training = training_score_from(&volumes, balance, &mut silent);
        let risk = injury_risk_from(store, day, ratio, &volumes, recovery, &mut silent, th)?;

        out.push(ScorePoint {
            date: day,
            training: training.round() as u32,
            recovery,
            risk: risk.round() as u32,
        });
    }
    if let Some(last) = out.last_mut() {
        *last = today;
    }
    Ok(out)
}

/// Injury-risk heuristics for the dashboard, decomposed into named factors.
/// Each factor carries its own *severity* on 0..=1 — how far past its comfort
/// threshold that one signal is, saturated at its own "maxed out" point — so
/// the bars read honestly: an empty bar means that signal is quiet, and a
/// full bar means it alone is at worst. (The old model normalised factors into
/// shares of the total, which rendered every bar empty whenever the runner was
/// healthy — all raw values clamped to 0 — and inflated one tiny factor into a
/// 100% bar.) The raw pre-clamp sum still comes back so the caller can derive
/// the headline percentage exactly as before.
pub fn compute_risk_factors(
    store: &Store,
    days: i64,
    th: &Thresholds,
) -> Result<(Vec<RiskFactor>, f64)> {
    let mut factors: Vec<RiskFactor> = Vec::new();
    let mut total = 0.0f64;
    // raw: keep the headline sum; severity: what the bar shows.
    let mut push = |label: &str, detail: String, raw: f64, span: f64| {
        total += raw;
        factors.push(RiskFactor {
            label: label.into(),
            detail,
            contribution: (raw / span).clamp(0.0, 1.0),
        });
    };

    // Load ratio (ACWR): raw keeps its 0.9 weight for the headline; the bar
    // saturates 0.5 ratio above the optimized ceiling.
    let load = store.latest_training_load()?;
    let ratio = load.as_ref().and_then(|l| l.ratio).unwrap_or(1.0);
    push(
        "Load ratio (ACWR)",
        format!(
            "{:.2} — {}",
            ratio,
            if ratio <= th.ratio_optimized_hi {
                "within optimized range"
            } else {
                "above optimized range"
            }
        ),
        ((ratio - th.ratio_optimized_hi).max(0.0)) * 0.9,
        0.9 * 0.5,
    );

    // Volume spike: last 7 d vs mean of the three previous weeks.
    let last7 = store.distance_window(7, 0)? / 1000.0;
    let prior = store.distance_window(28, 7)? / 1000.0 / 3.0;
    let spike = if prior > 1.0 { last7 / prior } else { 1.0 };
    push(
        "Volume spike",
        if spike > 1.3 {
            format!("+{:.0}% vs 3-week baseline", (spike - 1.0) * 100.0)
        } else {
            format!("within baseline ({:.0}%)", spike * 100.0)
        },
        ((spike - 1.3).max(0.0)) * 1.2,
        1.2 * 0.7,
    );

    // Recovery debt: the same recovery score the headline uses, so the panel
    // explains the part of the risk that comes from sleep/HRV/RHR rather than
    // training. Quiet until recovery drops under 60, and saturated at 20 — a
    // 20/100 recovery is as bad as it gets, so the bar reaches full well
    // before the score hits zero. Pushed after the closure so it can touch
    // `factors` directly, and deliberately left out of `total` —
    // `injury_risk_from` already folds recovery into the headline, so counting
    // it here too would double it.
    let recovery = recovery_from_health(&store.recent_health(3)?);
    let recovery_debt = ((60.0 - recovery as f64).max(0.0) / 40.0).clamp(0.0, 1.0);

    // Wearable-derived factors over the selected window. Sleep saturates 25
    // points under the comfort line rather than 40: a 45/100 week-average is a
    // real risk signal, and on the wider span the bar only moved at sleep
    // scores few wearables ever report.
    let win = store.recent_health(days.max(1))?;
    if let Some(sleep) = mean_observed(win.iter().filter_map(|h| h.sleep_score.map(|v| v as f64))) {
        push(
            "Sleep quality",
            format!("{:.0}/100 avg", sleep),
            ((70.0 - sleep).max(0.0)) * 0.02,
            0.02 * 25.0,
        );
    }

    // HRV: a flat tolerance band first. Night-to-night HRV scatters by well
    // over 10%, so a drop inside the band is noise, not a trend — and a bar
    // that grows with any dip would sit half-full for a healthy runner.
    // Only the excess past the band counts, saturating 30% below it.
    // The baseline is the older half of the 28-day window (rows are newest
    // first); with less than a month synced there is no older half, so the
    // whole window stands in rather than the row vanishing.
    let hrv_now = mean_observed(win.iter().filter_map(|h| h.hrv_avg));
    let base = store.recent_health(28)?;
    let hrv_base = mean_observed(base.iter().skip(14).filter_map(|h| h.hrv_avg))
        .or_else(|| mean_observed(base.iter().filter_map(|h| h.hrv_avg)));
    if let (Some(now), Some(b)) = (hrv_now, hrv_base) {
        let b = b.max(1.0);
        push(
            "HRV trend",
            format!("{:.0} ms vs {:.0} ms baseline", now, b),
            ((b * 0.9 - now).max(0.0) / b) * 0.8,
            0.8 * 0.3,
        );
    }

    // Resting HR: same shape — nothing until it drifts more than 3 bpm above
    // baseline, maxed out 15 bpm past that.
    let rhr_now = mean_observed(win.iter().filter_map(|h| h.resting_hr).map(|v| v as f64));
    let rhr_base = mean_observed(
        base.iter()
            .skip(14)
            .filter_map(|h| h.resting_hr)
            .map(|v| v as f64),
    )
    .or_else(|| mean_observed(base.iter().filter_map(|h| h.resting_hr).map(|v| v as f64)));
    if let (Some(now), Some(b)) = (rhr_now, rhr_base) {
        push(
            "Resting HR drift",
            format!("{:.0} bpm vs {:.0} bpm baseline", now, b),
            ((now - b - 3.0).max(0.0)) * 0.06,
            0.06 * 15.0,
        );
    }

    if let Some(stress) = mean_observed(win.iter().filter_map(|h| h.stress_avg.map(|v| v as f64))) {
        push(
            "Stress load",
            format!("{:.0}/100 avg", stress),
            ((stress - 40.0).max(0.0)) * 0.015,
            0.015 * 40.0,
        );
    }

    // The runner's own report, weighted like any other factor.
    let pain_intensity = store
        .max_pain_intensity_since(Utc::now().date_naive())?
        .map(|(i, _)| i);
    if let Some((intensity, part)) = store.max_pain_intensity_since(Utc::now().date_naive())? {
        push(
            "Reported pain",
            format!("{} {}/10", crate::proposal::region_label(&part), intensity),
            intensity as f64 / 10.0,
            1.0,
        );
    }

    // Recent sessions: the runs of the last week themselves, weighted by how
    // hard each was — and how they interact with the pain above. This is what
    // makes the risk move when a run is banked, instead of waiting for the
    // provider's ratio to catch up.
    let now_ts = Utc::now().timestamp();
    let runs = store.runs_between(now_ts - 7 * 86400, now_ts + 86400)?;
    let threshold = store.latest_fitness()?.and_then(|f| f.threshold_sec_km);
    // No single session is being explained here — the newest run's own effort
    // stands in, which is the dashboard's "what did the last run cost me".
    let (sessions_raw, sessions_detail) = recent_sessions_pressure(&runs, threshold, pain_intensity, None);
    push("Recent sessions", sessions_detail, sessions_raw, 0.15 * 1.2);

    factors.push(RiskFactor {
        label: "Recovery debt".into(),
        detail: format!("{recovery}/100 recovery from sleep, HRV and resting HR"),
        contribution: recovery_debt,
    });

    Ok((factors, total))
}

/// Same heuristics as [`compute_risk_factors`], evaluated *as of* a past day
/// instead of today: every window is anchored on `day`, so the factors explain
/// the risk the runner was carrying when they did that activity.
/// `session_ts` (unix seconds) is the run being explained, when the caller
/// knows it — it pins the recent-sessions window to the session's own start
/// instead of the day's noon, and that run's own effort counts in the week's
/// pressure. Returns the severity-scaled factors plus the raw risk sum.
pub fn compute_risk_factors_as_of(
    store: &Store,
    day: NaiveDate,
    window_days: i64,
    th: &Thresholds,
    session_ts: Option<i64>,
) -> Result<(Vec<RiskFactor>, f64)> {
    risk_factors_as_of_inner(store, day, window_days, th, session_ts, None)
}

/// What one session cost the runner, in risk points: the headline risk as of
/// `day` with the run counted, minus the same headline with the run removed
/// from the week's runs. Unlike a snapshot-to-snapshot difference — which
/// compares two days and drifts whenever the model changes — this says what
/// *this* activity did: an easy jog moves the risk a little, a threshold
/// session a lot, and a light run that dilutes a heavy week can even come out
/// negative.
pub fn session_risk_impact(
    store: &Store,
    day: NaiveDate,
    window_days: i64,
    th: &Thresholds,
    session_ts: i64,
    session_id: i64,
) -> Result<i64> {
    let with = risk_headline_inner(store, day, window_days, th, Some(session_ts), None)?;
    let without =
        risk_headline_inner(store, day, window_days, th, Some(session_ts), Some(session_id))?;
    Ok(with as i64 - without as i64)
}

/// The dashboard's headline injury-risk percentage, evaluated *as of* a day:
/// the same blend the dashboard hero shows — the point-in-time formula
/// (ratio, volume spike, recovery, pain) averaged with the range-aware factor
/// sum — anchored on that day instead of today. The run snapshots, the
/// activity detail sheet, and the calendar descriptions all use this, so a
/// run banked today carries the same risk number the dashboard shows today.
pub fn risk_headline_as_of(
    store: &Store,
    day: NaiveDate,
    window_days: i64,
    th: &Thresholds,
    session_ts: Option<i64>,
) -> Result<u32> {
    risk_headline_inner(store, day, window_days, th, session_ts, None)
}

fn risk_headline_inner(
    store: &Store,
    day: NaiveDate,
    window_days: i64,
    th: &Thresholds,
    session_ts: Option<i64>,
    exclude_activity: Option<i64>,
) -> Result<u32> {
    // The point-in-time half of the dashboard's blend, as of `day` — the
    // same ingredients `score_history` rebuilds per day.
    let ratio = store.training_load_on_or_before(day)?.and_then(|l| l.ratio);
    let volumes = store.weekly_volumes_as_of(day, 8)?;
    let window = store.health_between(day - chrono::Duration::days(2), day)?;
    let recovery = recovery_from_health(&window);
    let mut silent = Vec::new();
    let base = injury_risk_from(store, day, ratio, &volumes, recovery, &mut silent, th)?;
    // The range-aware half, with the same session-aware recent-sessions
    // window the factor bars show.
    let (_, total) =
        risk_factors_as_of_inner(store, day, window_days, th, session_ts, exclude_activity)?;
    let range = (10.0 + total * 55.0).clamp(0.0, 100.0).round() as u32;
    Ok(((base.round() as u32 + range) / 2).clamp(0, 100))
}

fn risk_factors_as_of_inner(
    store: &Store,
    day: NaiveDate,
    window_days: i64,
    th: &Thresholds,
    session_ts: Option<i64>,
    exclude_activity: Option<i64>,
) -> Result<(Vec<RiskFactor>, f64)> {
    let day_ts = day.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
    let secs = |d: i64| 64800 + d * 86400; // half a day, so "7 days" is inclusive
    let span = |from: i64, to: i64| store.distance_between(day_ts - secs(from), day_ts - secs(to));
    let back = |d: i64| day - chrono::Duration::days(d);
    let win = |from: i64, to: i64| store.health_between(back(from), back(to));

    let mut factors: Vec<RiskFactor> = Vec::new();
    let mut total = 0.0f64;
    let mut push = |label: &str, detail: String, raw: f64, sat: f64| {
        total += raw;
        factors.push(RiskFactor {
            label: label.into(),
            detail,
            contribution: (raw / sat).clamp(0.0, 1.0),
        });
    };

    // Load ratio (ACWR) as recorded on/before that day.
    let load = store.training_load_on_or_before(day)?;
    let ratio = load.as_ref().and_then(|l| l.ratio).unwrap_or(1.0);
    push(
        "Load ratio (ACWR)",
        format!(
            "{:.2} — {}",
            ratio,
            if ratio <= th.ratio_optimized_hi {
                "within optimized range"
            } else {
                "above optimized range"
            }
        ),
        ((ratio - th.ratio_optimized_hi).max(0.0)) * 0.9,
        0.9 * 0.5,
    );

    // Volume spike: the 7 days up to that session vs the 3 weeks before them.
    let last7 = span(7, 1)? / 1000.0;
    let prior = span(28, 8)? / 1000.0 / 3.0;
    let spike = if prior > 1.0 { last7 / prior } else { 1.0 };
    push(
        "Volume spike",
        if spike > 1.3 {
            format!("+{:.0}% vs 3-week baseline", (spike - 1.0) * 100.0)
        } else {
            format!("within baseline ({:.0}%)", spike * 100.0)
        },
        ((spike - 1.3).max(0.0)) * 1.2,
        1.2 * 0.7,
    );

    // Wearable-derived factors in the window before the session.
    let w = win(window_days.max(1), 0)?;
    if let Some(sleep) = mean_observed(w.iter().filter_map(|h| h.sleep_score.map(|v| v as f64))) {
        push(
            "Sleep quality",
            format!("{:.0}/100 avg", sleep),
            ((70.0 - sleep).max(0.0)) * 0.02,
            0.02 * 25.0,
        );
    }

    let base = win(28, 0)?;
    let hrv_now = mean_observed(w.iter().filter_map(|h| h.hrv_avg));
    let hrv_base = mean_observed(base.iter().skip(14).filter_map(|h| h.hrv_avg))
        .or_else(|| mean_observed(base.iter().filter_map(|h| h.hrv_avg)));
    if let (Some(now), Some(b)) = (hrv_now, hrv_base) {
        let b = b.max(1.0);
        push(
            "HRV trend",
            format!("{:.0} ms vs {:.0} ms baseline", now, b),
            ((b * 0.9 - now).max(0.0) / b) * 0.8,
            0.8 * 0.3,
        );
    }

    let rhr_now = mean_observed(w.iter().filter_map(|h| h.resting_hr).map(|v| v as f64));
    let rhr_base = mean_observed(
        base.iter()
            .skip(14)
            .filter_map(|h| h.resting_hr)
            .map(|v| v as f64),
    )
    .or_else(|| mean_observed(base.iter().filter_map(|h| h.resting_hr).map(|v| v as f64)));
    if let (Some(now), Some(b)) = (rhr_now, rhr_base) {
        push(
            "Resting HR drift",
            format!("{:.0} bpm vs {:.0} bpm baseline", now, b),
            ((now - b - 3.0).max(0.0)) * 0.06,
            0.06 * 15.0,
        );
    }

    if let Some(stress) = mean_observed(w.iter().filter_map(|h| h.stress_avg.map(|v| v as f64))) {
        push(
            "Stress load",
            format!("{:.0}/100 avg", stress),
            ((stress - 40.0).max(0.0)) * 0.015,
            0.015 * 40.0,
        );
    }

    // The pain still hurting on that day, and the runs of the week that led
    // up to it — including the session being explained, whose own intensity
    // counts here. This is the factor that makes two runs on the same ratio
    // carry different risk: an easy jog and a threshold session move the
    // number differently, and stacking hard days (or running hard on pain)
    // moves it further. `exclude_activity` drops one run from the week — the
    // counterfactual `session_risk_impact` measures against.
    let pain_intensity = store.max_pain_intensity_since(day)?.map(|(i, _)| i);
    if let Some((intensity, part)) = store.max_pain_intensity_since(day)? {
        push(
            "Reported pain",
            format!("{} {}/10", crate::proposal::region_label(&part), intensity),
            intensity as f64 / 10.0,
            1.0,
        );
    }
    // Anchored on the session's own start when the caller knows it, so the
    // run being explained is inside its own window whatever timezone it was
    // banked in; the day's noon UTC stands in otherwise.
    let anchor = session_ts.unwrap_or(day_ts + 64_800);
    let mut runs = store.runs_between(anchor - 7 * 86_400, anchor + 60)?;
    if let Some(excluded) = exclude_activity {
        runs.retain(|run| run.id != excluded);
    }
    let threshold = store.fitness_on_or_before(day)?.and_then(|f| f.threshold_sec_km);
    let self_id = session_ts.and_then(|ts| {
        runs.iter()
            .find(|run| run.start_time.timestamp() == ts)
            .map(|run| run.id)
    });
    let (sessions_raw, sessions_detail) =
        recent_sessions_pressure(&runs, threshold, pain_intensity, self_id);
    push("Recent sessions", sessions_detail, sessions_raw, 0.15 * 1.2);

    Ok((factors, total))
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

    // ---- risk factor bars ------------------------------------------------

    /// One day of synced health; `back` is days before today.
    fn health_day(back: i64, sleep: i64, resting_hr: i64, hrv: f64) -> HealthDaily {
        HealthDaily {
            date: Utc::now().date_naive() - chrono::Duration::days(back),
            steps: None,
            avg_hr: None,
            resting_hr: Some(resting_hr),
            stress_avg: None,
            sleep_score: Some(sleep),
            sleep_min: Some(480),
            deep_min: None,
            rem_min: None,
            hrv_avg: Some(hrv),
        }
    }

    /// A store holding the given health days and nothing else: no runs and no
    /// synced load, so every training factor is quiet by construction and a
    /// lit bar is attributable to exactly one signal.
    fn health_store(days: &[HealthDaily]) -> Store {
        let path = std::env::temp_dir().join(format!(
            "runalytics-metrics-{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let store = Store::open(&path).expect("open store");
        for h in days {
            store.upsert_health(ProviderKind::Coros, h).expect("health");
        }
        store
    }

    fn severity(factors: &[RiskFactor], label: &str) -> f64 {
        factors
            .iter()
            .find(|f| f.label == label)
            .unwrap_or_else(|| panic!("no {label} factor in {factors:?}"))
            .contribution
    }

    /// Each severity bar is normalised on its own scale, so each needs its own
    /// absolute anchor — a scale-free assertion keeps passing while one bar is
    /// silently pinned near zero. A 50/100 week-average is a genuinely bad
    /// week and must light the sleep bar, and only the sleep bar.
    #[test]
    fn a_bad_sleep_week_lights_the_sleep_bar_and_nothing_else() {
        let days: Vec<HealthDaily> = (0..28).map(|back| health_day(back, 50, 50, 50.0)).collect();
        let store = health_store(&days);
        let (factors, _) = compute_risk_factors(&store, 7, &th()).expect("factors");

        // (70 - 50) * 0.02 over a 0.02 * 25 span = 0.8.
        assert!(
            (severity(&factors, "Sleep quality") - 0.8).abs() < 0.01,
            "sleep severity {}",
            severity(&factors, "Sleep quality")
        );
        for label in ["HRV trend", "Resting HR drift", "Recovery debt"] {
            assert!(
                severity(&factors, label) < 0.05,
                "{label} should be quiet, got {}",
                severity(&factors, label)
            );
        }
    }

    /// The other half of the complaint: a 78/100 sleep average sits above the
    /// comfort line, so its bar *should* be empty — the UI labels that state
    /// "quiet" rather than leaving the runner to guess. Asserting the zero
    /// here is what keeps that label honest instead of papering over a bar
    /// that cannot move.
    #[test]
    fn a_healthy_week_leaves_every_bar_quiet() {
        let days: Vec<HealthDaily> = (0..28).map(|back| health_day(back, 78, 50, 50.0)).collect();
        let store = health_store(&days);
        let (factors, _) = compute_risk_factors(&store, 7, &th()).expect("factors");
        assert!(!factors.is_empty());
        for f in &factors {
            assert!(
                f.contribution < 0.05,
                "{} should be quiet, got {}",
                f.label,
                f.contribution
            );
        }
    }

    /// Night-to-night HRV scatters by well over 10%, so a small dip is noise:
    /// the tolerance band keeps a healthy runner's bar empty and only a real
    /// drop registers, saturating 30% below baseline.
    #[test]
    fn hrv_band_ignores_noise_and_reacts_to_a_real_drop() {
        let t = th();
        let build = |recent: f64| {
            let days: Vec<HealthDaily> = (0..28)
                .map(|back| health_day(back, 80, 50, if back < 14 { recent } else { 50.0 }))
                .collect();
            let store = health_store(&days);
            let (factors, _) = compute_risk_factors(&store, 7, &t).expect("factors");
            severity(&factors, "HRV trend")
        };
        // 4% below baseline: inside the band, quiet.
        assert!(build(48.0) < 0.05, "a 4% dip is noise");
        // 20% below: (45 - 40) / 50 * 0.8 over 0.8 * 0.3 = 0.33.
        let twenty = build(40.0);
        assert!((twenty - 0.333).abs() < 0.01, "a 20% dip, got {twenty}");
        // 30% below or worse: maxed out.
        assert!(build(30.0) > 0.99, "a 40% dip should max the bar");
    }

    /// Same shape for resting HR: a few bpm of day-to-day variation is not
    /// drift, +15 over baseline is.
    #[test]
    fn resting_hr_band_ignores_noise_and_reacts_to_drift() {
        let t = th();
        let build = |recent: i64| {
            let days: Vec<HealthDaily> = (0..28)
                .map(|back| health_day(back, 80, if back < 14 { recent } else { 50 }, 50.0))
                .collect();
            let store = health_store(&days);
            let (factors, _) = compute_risk_factors(&store, 7, &t).expect("factors");
            severity(&factors, "Resting HR drift")
        };
        assert!(build(52) < 0.05, "2 bpm is noise");
        // (60 - 50 - 3) * 0.06 over 0.06 * 15 = 0.47.
        let ten = build(60);
        assert!((ten - 0.467).abs() < 0.01, "+10 bpm, got {ten}");
        assert!(build(68) > 0.99, "+18 bpm should max the bar");
    }

    /// Recovery debt saturates at a 20/100 recovery score — about the floor of
    /// what the formula reports in practice — rather than at 0, which it
    /// effectively never reaches.
    #[test]
    fn recovery_debt_saturates_where_recovery_actually_bottoms_out() {
        let t = th();
        let debt = |sleep_min: i64, resting_hr: i64, hrv: f64| {
            let days: Vec<HealthDaily> = (0..28)
                .map(|back| HealthDaily {
                    sleep_min: Some(sleep_min),
                    ..health_day(back, 80, resting_hr, hrv)
                })
                .collect();
            let store = health_store(&days);
            let (factors, _) = compute_risk_factors(&store, 7, &t).expect("factors");
            severity(&factors, "Recovery debt")
        };
        assert!(debt(480, 50, 50.0) < 0.05, "a good night is quiet");
        // 4 h sleep, 80 bpm resting, 20 ms HRV: recovery sits near the floor.
        let wrecked = debt(240, 80, 20.0);
        assert!(wrecked > 0.95, "a wrecked week, got {wrecked}");
    }
}
