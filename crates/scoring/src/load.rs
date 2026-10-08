//! Training load: the numbers that describe *how much* work an athlete did.
//!
//! Everything here is built on one scalar — a session's **intensity factor**
//! (IF), the fraction of the athlete's threshold the session was sustained at —
//! and the Coggan load identity `TSS = 100 · IF² · hours`. That single identity
//! is what makes a 20-minute interval session and a 90-minute long run
//! comparable, and it is the quantity the chronic/acute model then smooths.
//!
//! The intensity factor is derived from the best data available, in a fixed
//! order of trustworthiness:
//!
//! 1. **Pace**, when a threshold pace is known — the most direct measure for
//!    running, and unaffected by heat, caffeine or sleep the way heart rate is.
//! 2. **Heart-rate reserve** (Karvonen), when only heart rate is available.
//! 3. **Session-RPE**, when neither is recorded — the validated
//!    `IF ≈ %RPE` equivalence, which is why a watch-less athlete is not locked
//!    out of the model.
//!
//! The EMA windows are imported from `runalytics-plan-engine` rather than
//! redefined. That is deliberate: the plan projects a week's ACWR and the
//! dashboard measures the realised ACWR, and if the two used different
//! smoothing the athlete would see the same week scored two different ways.

use runalytics_core::{Activity, AthleteSnapshot, Date, HeartRate, Pace};
use runalytics_plan_engine::{ACWR_TARGET_HIGH, ACWR_TARGET_LOW, ema};

/// Days behind the acute load — the "how tired are they today" window.
pub const ACUTE_WINDOW_DAYS: f64 = 7.0;
/// Days behind the chronic load — the "what are they fit for" window.
pub const CHRONIC_WINDOW_DAYS: f64 = 42.0;

/// Intensity factor from pace: threshold pace divided by average pace.
///
/// Both are seconds-per-kilometre, so running *faster* (a smaller average)
/// yields a larger factor, which is the direction we want. A session run
/// exactly at threshold scores IF = 1.0.
#[must_use]
pub fn intensity_from_pace(avg_pace: Pace, threshold: Pace) -> f64 {
    if avg_pace.as_secs_per_km() <= 0.0 {
        return 0.0;
    }
    threshold.as_secs_per_km() / avg_pace.as_secs_per_km()
}

/// Intensity factor from heart rate, on the heart-rate *reserve* scale.
///
/// Reserve (Karvonen) rather than raw `%HRmax`: two athletes at 150 bpm can be
/// doing very different work depending on their resting rate, and reserve
/// corrects for that. Returns `None` when the inputs are sentinels so callers
/// never divide by a zero-width reserve.
#[must_use]
pub fn intensity_from_hr(avg_hr: HeartRate, athlete: &AthleteSnapshot) -> Option<f64> {
    let max = f64::from(athlete.max_hr.as_u16());
    let rest = f64::from(athlete.resting_hr.as_u16());
    let avg = f64::from(avg_hr.as_u16());
    let reserve = max - rest;
    if !avg_hr.is_recorded() || reserve <= 0.0 {
        return None;
    }
    Some((avg - rest) / reserve)
}

/// Intensity factor from session rate of perceived exertion (Foster, 1-10).
///
/// Uses the validated near-equivalence of `%RPE` and `%HRmax`. Deliberately a
/// blunt instrument — it is the fallback when nothing better exists, and it is
/// honest about that by being coarse.
#[must_use]
pub fn intensity_from_rpe(rpe: u8) -> f64 {
    f64::from(rpe).clamp(1.0, 10.0) / 10.0
}

/// The intensity factor for an activity, from the best data available.
///
/// Pace and heart rate are *not* averaged. They are normalised on different
/// scales — pace against threshold, heart rate against heart-rate reserve — and
/// averaging two quantities that share neither units nor zero point produces a
/// number that describes neither. An easy run on a hot day would score as if it
/// were tempo work, and a tempo run with a low cardiac drift would score as if
/// it were easy: exactly backwards, because the two signals diverge *most* when
/// one of them is lying.
///
/// Instead the higher of the two wins. This is the conservative choice in both
/// directions that matter: it never under-reports the strain a session placed on
/// the body (which is what feeds injury risk), and it never lets a corrupted or
/// missing signal quietly halve an athlete's load. The cost is symmetric — a
/// genuinely hot day inflates the load — and that cost is the honest one,
/// because heat *is* extra strain.
#[must_use]
pub fn intensity_factor(activity: &Activity, athlete: &AthleteSnapshot, threshold: Pace) -> f64 {
    let pace_if = activity
        .summary
        .avg_pace
        .map(|pace| intensity_from_pace(pace, threshold));
    let hr_if = activity
        .summary
        .avg_hr
        .and_then(|hr| intensity_from_hr(hr, athlete));

    let raw = match (pace_if, hr_if) {
        (Some(p), Some(h)) => p.max(h),
        (Some(p), None) => p,
        (None, Some(h)) => h,
        // No objective signal: fall back to the provider's own intensity label,
        // which is itself a model but is better than assuming a flat 0.5.
        (None, None) => activity.intensity.load_factor(),
    };
    // Clamp to a physiologically sane band. A single session cannot be run at
    // an average of 1.5× threshold for its whole duration; anything above ~1.2
    // is a data artefact (a stopped watch, a downhill GPS spike) and would
    // otherwise dominate the acute load.
    raw.clamp(0.0, 1.2)
}

/// Training Stress Score for an activity: `100 · IF² · hours`.
#[must_use]
pub fn training_stress_score(
    activity: &Activity,
    athlete: &AthleteSnapshot,
    threshold: Pace,
) -> f64 {
    let if_ = intensity_factor(activity, athlete, threshold);
    let hours = f64::from(activity.summary.duration.as_u32()) / 3600.0;
    100.0 * if_ * if_ * hours
}

/// Load for an activity when only an RPE is known (no HR, no pace).
///
/// Kept as a separate entry point so the caller is explicit about the lower
/// confidence of an RPE-only load.
#[must_use]
pub fn load_from_rpe(rpe: u8, duration_secs: u32) -> f64 {
    let if_ = intensity_from_rpe(rpe);
    let hours = f64::from(duration_secs) / 3600.0;
    100.0 * if_ * if_ * hours
}

/// Load for a single activity, preferring the provider's own training-load
/// number when it published one and we have no better signal.
///
/// We do *not* blindly trust the provider number: it is computed on their
/// thresholds, not ours, so it would drift from our CTL the moment we changed
/// an athlete's threshold. It is used only as a last resort.
#[must_use]
pub fn activity_load(activity: &Activity, athlete: &AthleteSnapshot, threshold: Pace) -> f64 {
    let has_objective = activity.summary.avg_pace.is_some() || activity.summary.avg_hr.is_some();
    if !has_objective && let Some(load) = activity.summary.training_load {
        return load.max(0.0);
    }
    training_stress_score(activity, athlete, threshold)
}

/// A day's total load, the unit the chronic/acute model smooths.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DailyLoad {
    pub date: Date,
    pub load: f64,
}

/// Sum activity loads into a per-day series, ordered by date.
///
/// Multiple activities on one day (a morning track session and an evening
/// recovery jog) accumulate, which is correct: the body experiences the day's
/// total stress, not each session in isolation.
#[must_use]
pub fn daily_loads(
    activities: &[Activity],
    athlete: &AthleteSnapshot,
    threshold: Pace,
) -> Vec<DailyLoad> {
    let mut by_date: std::collections::BTreeMap<Date, f64> = std::collections::BTreeMap::new();
    for activity in activities {
        *by_date.entry(activity.local_date).or_insert(0.0) +=
            activity_load(activity, athlete, threshold);
    }
    by_date
        .into_iter()
        .map(|(date, load)| DailyLoad { date, load })
        .collect()
}

/// The chronic (42-day) load at the end of the series.
#[must_use]
pub fn chronic_load(series: &[DailyLoad]) -> f64 {
    let loads: Vec<f64> = series.iter().map(|d| d.load).collect();
    ema(&loads, CHRONIC_WINDOW_DAYS)
}

/// The acute (7-day) load at the end of the series.
#[must_use]
pub fn acute_load(series: &[DailyLoad]) -> f64 {
    let loads: Vec<f64> = series.iter().map(|d| d.load).collect();
    ema(&loads, ACUTE_WINDOW_DAYS)
}

/// Training stress balance: `CTL − ATL`. Positive means fresher than fit.
#[must_use]
pub fn training_stress_balance(series: &[DailyLoad]) -> f64 {
    chronic_load(series) - acute_load(series)
}

/// Acute:chronic workload ratio, `ATL / CTL`.
///
/// Returns `1.0` for an empty series — the neutral point — so a brand-new
/// athlete with no history is neither flagged safe nor flagged at risk.
#[must_use]
pub fn acute_chronic_ratio(series: &[DailyLoad]) -> f64 {
    let chronic = chronic_load(series);
    if chronic <= 0.0 {
        return 1.0;
    }
    acute_load(series) / chronic
}

/// Whether the ACWR sits inside the safe training band.
#[must_use]
pub fn acwr_in_band(acwr: f64) -> bool {
    (ACWR_TARGET_LOW..=ACWR_TARGET_HIGH).contains(&acwr)
}

/// Upper bound reported for monotony.
///
/// A perfectly monotonous week has zero standard deviation, so the ratio is
/// mathematically infinite. Infinity cannot be serialised to JSON (it becomes
/// `null` and silently breaks the dashboard), and no coach reads "infinite"
/// anyway — the actionable message is simply "no variation at all". So the
/// scale is capped at the point where it is already maximally alarming.
pub const MONOTONY_CAP: f64 = 10.0;

/// Weekly monotony (Foster): mean daily load over its standard deviation.
///
/// High monotony means every week looks the same — no variation in stress,
/// which is an independent injury risk factor beyond total load. Returns `0.0`
/// when there is too little data to judge.
#[must_use]
pub fn monotony(series: &[DailyLoad]) -> f64 {
    let window: Vec<f64> = series.iter().rev().take(7).map(|d| d.load).collect();
    if window.len() < 2 {
        return 0.0;
    }
    let mean = window.iter().sum::<f64>() / window.len() as f64;
    if mean <= 0.0 {
        return 0.0;
    }
    let variance = window.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / window.len() as f64;
    let sd = variance.sqrt();
    if sd <= f64::EPSILON {
        return MONOTONY_CAP;
    }
    (mean / sd).min(MONOTONY_CAP)
}

/// Weekly spike: this week's load over the mean of the preceding four weeks.
///
/// The single most-cited acute injury signal. A spike above ~1.3 — doing far
/// more this week than the body has recently adapted to — is where tendons and
/// bone fail. Returns `1.0` when there is no prior week to compare against.
#[must_use]
pub fn weekly_spike(series: &[DailyLoad]) -> f64 {
    let weekly = weekly_totals(series);
    if weekly.is_empty() {
        return 1.0;
    }
    let current = *weekly.last().expect("non-empty");
    let prior: Vec<f64> = weekly[..weekly.len() - 1]
        .iter()
        .rev()
        .take(4)
        .copied()
        .collect();
    if prior.is_empty() {
        return 1.0;
    }
    let mean = prior.iter().sum::<f64>() / prior.len() as f64;
    if mean <= 0.0 {
        return 1.0;
    }
    current / mean
}

/// Collapse a daily series into whole-week totals (7-day buckets, oldest first).
///
/// Buckets are trailing 7-day windows, not calendar weeks, so a partial first
/// week does not distort the spike ratio.
#[must_use]
pub fn weekly_totals(series: &[DailyLoad]) -> Vec<f64> {
    let loads: Vec<f64> = series.iter().map(|d| d.load).collect();
    if loads.is_empty() {
        return Vec::new();
    }
    let mut weeks = Vec::new();
    let mut idx = loads.len();
    while idx > 0 {
        let start = idx.saturating_sub(7);
        let sum: f64 = loads[start..idx].iter().sum();
        weeks.push(sum);
        idx = start;
    }
    weeks.reverse();
    weeks
}

#[cfg(test)]
mod tests {
    use super::*;
    use runalytics_core::Tz;
    use runalytics_core::{ActivitySummary, DurationSecs, Intensity, VolumeKm};

    fn athlete() -> AthleteSnapshot {
        AthleteSnapshot::placeholder("Europe/Madrid".parse::<Tz>().expect("valid tz"))
    }

    fn threshold() -> Pace {
        Pace::new(270.0) // 4:30/km
    }

    fn activity(distance: f64, minutes: u32, avg_pace: f64, avg_hr: u16) -> Activity {
        Activity {
            id: runalytics_core::ActivityId::new(),
            account: runalytics_core::ProviderAccountId::new(),
            provider_activity_id: "a".into(),
            name: "run".into(),
            started_at: runalytics_core::Timestamp::default(),
            local_date: Date::from_ymd_opt(2026, 10, 5).expect("date"),
            summary: ActivitySummary {
                distance: VolumeKm(distance),
                duration: DurationSecs::from_minutes(minutes),
                avg_pace: Some(Pace::new(avg_pace)),
                avg_hr: if avg_hr == 0 {
                    None
                } else {
                    Some(HeartRate::new(avg_hr))
                },
                max_hr: None,
                elevation_gain: None,
                avg_cadence: None,
                training_load: None,
            },
            laps: vec![],
            intensity: Intensity::Aerobic,
            matched_session: None,
            fetched_at: runalytics_core::Timestamp::default(),
        }
    }

    #[test]
    fn pace_at_threshold_is_if_one() {
        assert!((intensity_from_pace(Pace::new(270.0), threshold()) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn faster_pace_is_higher_intensity() {
        assert!(
            intensity_from_pace(Pace::new(240.0), threshold())
                > intensity_from_pace(Pace::new(300.0), threshold())
        );
    }

    #[test]
    fn hr_reserve_uses_resting_rate() {
        // max 185, rest 55 -> reserve 130. avg 150 -> (150-55)/130 = 0.73
        let a = athlete();
        let if_ = intensity_from_hr(HeartRate::new(150), &a).expect("valid");
        assert!((if_ - 0.7308).abs() < 0.01);
        assert!(intensity_from_hr(HeartRate::NONE, &a).is_none());
    }

    #[test]
    fn rpe_maps_to_a_fraction() {
        assert!((intensity_from_rpe(6) - 0.6).abs() < 1e-9);
        assert_eq!(intensity_from_rpe(0), 0.1); // clamped up to 1/10
        assert_eq!(intensity_from_rpe(20), 1.0); // clamped down to 10/10
    }

    #[test]
    fn tss_is_if_squared_times_hours() {
        // 60 min at threshold -> IF 1.0 -> TSS 100
        let a = activity(13.3, 60, 270.0, 0);
        let tss = training_stress_score(&a, &athlete(), threshold());
        assert!((tss - 100.0).abs() < 0.5, "got {tss}");
    }

    #[test]
    fn a_harder_shorter_session_can_match_an_easier_longer_one() {
        // 30 min at IF ~1.13 (pace 240) -> 100*1.28*0.5 = 64
        let hard = activity(7.5, 30, 240.0, 0);
        // 60 min at IF 0.8 (pace 337.5) -> 100*0.64*1 = 64
        let easy = activity(10.7, 60, 337.5, 0);
        let ath = athlete();
        let hard_tss = training_stress_score(&hard, &ath, threshold());
        let easy_tss = training_stress_score(&easy, &ath, threshold());
        assert!(
            (hard_tss - easy_tss).abs() < 1.0,
            "{hard_tss} vs {easy_tss}"
        );
    }

    #[test]
    fn intensity_is_clamped_below_artefact_range() {
        // Absurdly fast pace (a GPS glitch) must not produce IF > 1.2.
        let glitch = activity(10.0, 60, 60.0, 0);
        let if_ = intensity_factor(&glitch, &athlete(), threshold());
        assert!(if_ <= 1.2);
    }

    #[test]
    fn provider_load_is_a_last_resort_only() {
        let mut a = activity(10.0, 60, 0.0, 0);
        a.summary.avg_pace = None;
        a.summary.training_load = Some(77.0);
        // No objective signal -> trust the provider number.
        assert_eq!(activity_load(&a, &athlete(), threshold()), 77.0);
        // Once we have pace, our own number wins over theirs.
        a.summary.avg_pace = Some(Pace::new(270.0));
        assert!((activity_load(&a, &athlete(), threshold()) - 100.0).abs() < 0.5);
    }

    #[test]
    fn daily_loads_accumulate_same_day_sessions() {
        let ath = athlete();
        let acts = vec![activity(6.0, 40, 300.0, 0), activity(6.0, 40, 300.0, 0)];
        let series = daily_loads(&acts, &ath, threshold());
        assert_eq!(series.len(), 1, "both on the same local date");
        let single = training_stress_score(&acts[0], &ath, threshold());
        assert!((series[0].load - 2.0 * single).abs() < 0.5);
    }

    #[test]
    fn empty_series_is_neutral() {
        assert_eq!(chronic_load(&[]), 0.0);
        assert_eq!(acute_chronic_ratio(&[]), 1.0);
        assert_eq!(weekly_spike(&[]), 1.0);
        assert_eq!(monotony(&[]), 0.0);
    }

    #[test]
    fn steady_load_sits_near_acwr_one() {
        let ath = athlete();
        let acts: Vec<Activity> = (0..60)
            .map(|d| {
                let mut a = activity(8.0, 45, 300.0, 0);
                a.local_date = Date::from_ymd_opt(2026, 8, 1)
                    .expect("base")
                    .checked_add_days(chrono::Days::new(d))
                    .expect("in range");
                a
            })
            .collect();
        let series = daily_loads(&acts, &ath, threshold());
        let acwr = acute_chronic_ratio(&series);
        assert!(
            (acwr - 1.0).abs() < 0.05,
            "steady load should be ~1.0, got {acwr}"
        );
        assert!(acwr_in_band(acwr));
    }

    #[test]
    fn a_sudden_jump_pushes_acwr_up() {
        let ath = athlete();
        let mut acts = Vec::new();
        for d in 0..50 {
            let mut a = activity(6.0, 35, 320.0, 0);
            a.local_date = Date::from_ymd_opt(2026, 8, 1)
                .expect("base")
                .checked_add_days(chrono::Days::new(d))
                .expect("in range");
            acts.push(a);
        }
        // Then a week of double volume.
        for d in 50..57 {
            let mut a = activity(14.0, 80, 300.0, 0);
            a.local_date = Date::from_ymd_opt(2026, 8, 1)
                .expect("base")
                .checked_add_days(chrono::Days::new(d))
                .expect("in range");
            acts.push(a);
        }
        let series = daily_loads(&acts, &ath, threshold());
        assert!(
            acute_chronic_ratio(&series) > 1.3,
            "spike should exceed the band"
        );
        assert!(weekly_spike(&series) > 1.3);
    }

    #[test]
    fn monotony_is_high_when_every_day_is_identical() {
        let ath = athlete();
        let acts: Vec<Activity> = (0..7)
            .map(|d| {
                let mut a = activity(8.0, 45, 300.0, 0);
                a.local_date = Date::from_ymd_opt(2026, 10, 1)
                    .expect("base")
                    .checked_add_days(chrono::Days::new(d))
                    .expect("in range");
                a
            })
            .collect();
        let series = daily_loads(&acts, &ath, threshold());
        // Identical days -> zero variance -> the scale saturates rather than
        // returning infinity.
        assert_eq!(monotony(&series), MONOTONY_CAP);
    }

    #[test]
    fn weekly_totals_bucket_by_seven_days() {
        let ath = athlete();
        let acts: Vec<Activity> = (0..14)
            .map(|d| {
                let mut a = activity(8.0, 45, 300.0, 0);
                a.local_date = Date::from_ymd_opt(2026, 10, 1)
                    .expect("base")
                    .checked_add_days(chrono::Days::new(d))
                    .expect("in range");
                a
            })
            .collect();
        let series = daily_loads(&acts, &ath, threshold());
        let weeks = weekly_totals(&series);
        assert_eq!(weeks.len(), 2);
        assert!((weeks[0] - weeks[1]).abs() < 0.5);
    }
}
