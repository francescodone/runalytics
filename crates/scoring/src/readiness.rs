//! Daily readiness: how recovered the athlete is *today*.
//!
//! Readiness answers one question — "should the planned hard session happen
//! today, or should it slide?" — and it is deliberately separate from fitness.
//! An athlete can be in the best shape of their life and still be the wrong
//! person to run intervals this morning.
//!
//! The model is a weighted blend of normalised subscores, each mapped to
//! `0.0..=1.0` against a personal baseline rather than a population average.
//! Two properties matter more than the exact weights:
//!
//! * **Missing data must not read as good data.** A day with no sleep record is
//!   not a day of perfect sleep. Signals that are absent are dropped and their
//!   weight is redistributed across the signals that *are* present, and the
//!   result carries a `confidence` so the UI can say "this is a thin estimate"
//!   instead of presenting a guess as a measurement.
//! * **Deviation is from the athlete's own baseline.** A resting HR of 48 is
//!   excellent for one athlete and a red flag for another. Each signal is
//!   scored against a trailing baseline of that athlete's own history, so the
//!   number means something without needing to know anything about them.

use runalytics_core::{Date, HealthDay};

/// A trailing baseline for one signal, built from the athlete's own history.
#[derive(Debug, Clone, Copy)]
pub struct Baseline {
    pub mean: f64,
    pub sd: f64,
    pub samples: usize,
}

impl Baseline {
    /// Build a baseline from observed values, ignoring none.
    ///
    /// Fewer than two samples yields no spread, and a spread of zero would make
    /// every deviation infinite — so a thin baseline reports `sd = None` and
    /// callers fall back to an absolute tolerance instead of a z-score.
    #[must_use]
    pub fn new(values: &[f64]) -> Option<Self> {
        if values.is_empty() {
            return None;
        }
        let n = values.len() as f64;
        let mean = values.iter().sum::<f64>() / n;
        if values.len() < 2 {
            return Some(Self {
                mean,
                sd: 0.0,
                samples: values.len(),
            });
        }
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0);
        Some(Self {
            mean,
            sd: variance.sqrt(),
            samples: values.len(),
        })
    }

    /// How far `value` sits from the baseline, in standard deviations.
    /// `None` when the baseline is too thin to say.
    #[must_use]
    pub fn z_score(&self, value: f64) -> Option<f64> {
        if self.sd <= f64::EPSILON {
            return None;
        }
        Some((value - self.mean) / self.sd)
    }
}

/// Which signals contributed to a readiness score, and how much each pulled.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ReadinessComponents {
    /// Sleep duration and efficiency, 0-1. `None` when no sleep was recorded.
    pub sleep: Option<f64>,
    /// HRV against baseline, 0-1.
    pub hrv: Option<f64>,
    /// Resting heart rate against baseline, 0-1 (lower is better).
    pub resting_hr: Option<f64>,
    /// Daytime stress against baseline, 0-1 (lower is better).
    pub stress: Option<f64>,
    /// Lowest overnight heart rate against baseline, 0-1 (lower is better).
    /// A rising lowest-HR is one of the earliest signs of illness.
    pub lowest_hr: Option<f64>,
}

impl ReadinessComponents {
    /// The signals that actually contributed, with their weights.
    ///
    /// Weights are renormalised over present signals only. This is the whole
    /// point: a partial sync must produce a *less confident* score, not a
    /// score padded with full marks for the missing parts.
    #[must_use]
    pub fn weighted(&self) -> Vec<(&'static str, f64, f64)> {
        const W_SLEEP: f64 = 0.35;
        const W_HRV: f64 = 0.25;
        const W_REST: f64 = 0.20;
        const W_STRESS: f64 = 0.10;
        const W_LOWEST: f64 = 0.10;

        let mut parts: Vec<(&'static str, f64, f64)> = Vec::new();
        if let Some(v) = self.sleep {
            parts.push(("sleep", W_SLEEP, v));
        }
        if let Some(v) = self.hrv {
            parts.push(("hrv", W_HRV, v));
        }
        if let Some(v) = self.resting_hr {
            parts.push(("resting_hr", W_REST, v));
        }
        if let Some(v) = self.stress {
            parts.push(("stress", W_STRESS, v));
        }
        if let Some(v) = self.lowest_hr {
            parts.push(("lowest_hr", W_LOWEST, v));
        }
        parts
    }

    /// Share of the full weight set that was actually observed, `0.0..=1.0`.
    #[must_use]
    pub fn coverage(&self) -> f64 {
        self.weighted().iter().map(|(_, w, _)| w).sum()
    }
}

/// A computed readiness score.
#[derive(Debug, Clone, PartialEq)]
pub struct Readiness {
    pub date: Date,
    /// `0..=100`.
    pub score: f64,
    pub components: ReadinessComponents,
    /// `0.0..=1.0` — how much of the signal set was present.
    pub confidence: f64,
}

/// Map a z-score to `0.0..=1.0` with `0` (on baseline) scoring `0.75`.
///
/// The curve is deliberately asymmetric around a comfortable middle: sitting
/// exactly on one's baseline is *fine*, not *optimal*, so it earns 75 % and
/// leaves room for the genuinely well-recovered days to stand out. Each
/// standard deviation of deviation costs a fixed amount, and the score saturates
/// rather than running off to zero — a single bad night should not read as
/// "do not train".
#[must_use]
fn score_from_z(z: f64, direction: Direction) -> f64 {
    // `direction` tells us whether a higher value is good or bad.
    let signed = match direction {
        Direction::HigherIsBetter => z,
        Direction::LowerIsBetter => -z,
    };
    let base = 0.75;
    let raw = if signed >= 0.0 {
        // Above baseline: modest upside, capped. Being 2 SD better than usual
        // is as good as this signal gets.
        base + (signed * 0.125).min(0.25)
    } else {
        // Below baseline: steeper penalty than the upside. Recovery debt is
        // more consequential than recovery surplus.
        base - (signed.abs() * 0.25).min(0.75)
    };
    raw.clamp(0.0, 1.0)
}

#[derive(Debug, Clone, Copy)]
enum Direction {
    HigherIsBetter,
    LowerIsBetter,
}

/// Score sleep on duration and efficiency.
///
/// Duration is scored against a 7.5 h target because that is where the
/// performance-recovery literature puts the knee of the curve; efficiency
/// below 85 % is the clinical threshold for insomnia and is penalised
/// separately, since time in bed is not the same as sleep.
#[must_use]
fn score_sleep(day: &HealthDay) -> Option<f64> {
    let sleep = day.sleep.as_ref()?;
    let hours = f64::from(sleep.total.as_u32()) / 3600.0;
    if hours <= 0.0 {
        return None;
    }
    let target = 7.5;
    // Generous above target, unforgiving below: 1.5 h short costs a third.
    let duration_score = if hours >= target {
        1.0
    } else {
        (hours / target).clamp(0.0, 1.0)
    };
    let efficiency = sleep.efficiency();
    let efficiency_score = if efficiency >= 0.85 {
        1.0
    } else {
        (efficiency / 0.85).clamp(0.0, 1.0)
    };
    // Naps are a genuine recovery credit, but a small one — they do not
    // substitute for consolidated overnight sleep.
    let nap_credit = (f64::from(sleep.nap.as_u32()) / 3600.0 * 0.05).min(0.05);
    Some((duration_score * 0.7 + efficiency_score * 0.3).min(1.0) + nap_credit)
}

/// Compute readiness for one day against the athlete's trailing baselines.
///
/// `history` should be the days *before* `day`, oldest first, so the baselines
/// never include the day being scored — scoring a day against a baseline that
/// contains itself would flatten every deviation to zero.
#[must_use]
pub fn readiness_for(day: &HealthDay, history: &[HealthDay]) -> Readiness {
    let hrv_values: Vec<f64> = history
        .iter()
        .filter_map(|d| d.sleep.as_ref().and_then(|s| s.hrv))
        .collect();
    let rest_values: Vec<f64> = history
        .iter()
        .filter_map(|d| d.resting_hr.map(|hr| f64::from(hr.as_u16())))
        .collect();
    let stress_values: Vec<f64> = history.iter().filter_map(|d| d.avg_stress).collect();
    let lowest_values: Vec<f64> = history
        .iter()
        .filter_map(|d| {
            d.sleep
                .as_ref()
                .and_then(|s| s.lowest_hr)
                .map(|hr| f64::from(hr.as_u16()))
        })
        .collect();

    let components = ReadinessComponents {
        sleep: score_sleep(day),
        hrv: day
            .sleep
            .as_ref()
            .and_then(|s| s.hrv)
            .and_then(|v| score_against(v, Baseline::new(&hrv_values), Direction::HigherIsBetter)),
        resting_hr: day
            .resting_hr
            .map(|hr| f64::from(hr.as_u16()))
            .and_then(|v| score_against(v, Baseline::new(&rest_values), Direction::LowerIsBetter)),
        stress: day.avg_stress.and_then(|v| {
            score_against(v, Baseline::new(&stress_values), Direction::LowerIsBetter)
        }),
        lowest_hr: day
            .sleep
            .as_ref()
            .and_then(|s| s.lowest_hr)
            .map(|hr| f64::from(hr.as_u16()))
            .and_then(|v| {
                score_against(v, Baseline::new(&lowest_values), Direction::LowerIsBetter)
            }),
    };

    Readiness {
        date: day.date,
        score: readiness_score(&components),
        components,
        confidence: components.coverage(),
    }
}

/// Score a value against a baseline, falling back to an absolute tolerance when
/// the baseline is too thin to produce a z-score.
fn score_against(value: f64, baseline: Option<Baseline>, direction: Direction) -> Option<f64> {
    let baseline = baseline?;
    match baseline.z_score(value) {
        Some(z) => Some(score_from_z(z, direction)),
        // Not enough history for a spread. Assume the athlete's first
        // observations *are* their baseline, so the score is neutral rather
        // than accidentally alarming.
        None => Some(0.75),
    }
}

/// Combine components into a `0..=100` score.
///
/// With no signal at all the score is `50.0` — the neutral midpoint. Reporting
/// zero would tell an athlete with a dead battery that they are unwell, which
/// is a lie the wearable did not earn the right to tell.
#[must_use]
pub fn readiness_score(components: &ReadinessComponents) -> f64 {
    let parts = components.weighted();
    if parts.is_empty() {
        return 50.0;
    }
    let weight_sum: f64 = parts.iter().map(|(_, w, _)| w).sum();
    let weighted: f64 = parts.iter().map(|(_, w, v)| w * v).sum();
    (weighted / weight_sum * 100.0).clamp(0.0, 100.0)
}

/// A coarse band for display, so the UI can colour without re-deriving thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadinessBand {
    /// Ready for the hardest session in the plan.
    Primed,
    /// Training as planned.
    Ready,
    /// Hard sessions will feel hard; consider easing.
    Guarded,
    /// Recovery day.
    Depleted,
}

impl ReadinessBand {
    #[must_use]
    pub fn from_score(score: f64) -> Self {
        match score {
            s if s >= 80.0 => Self::Primed,
            s if s >= 60.0 => Self::Ready,
            s if s >= 40.0 => Self::Guarded,
            _ => Self::Depleted,
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Primed => "primed",
            Self::Ready => "ready",
            Self::Guarded => "guarded",
            Self::Depleted => "depleted",
        }
    }
}

/// Resting-heart-rate drift over a trailing window, in bpm.
///
/// Exposed separately because a *trend* is far more informative than a level:
/// a resting HR climbing 5 bpm over a week is the most reliable early warning
/// of illness or overreaching that a consumer wearable produces.
#[must_use]
pub fn resting_hr_drift(history: &[HealthDay]) -> Option<f64> {
    let values: Vec<f64> = history
        .iter()
        .filter_map(|d| d.resting_hr.map(|hr| f64::from(hr.as_u16())))
        .collect();
    if values.len() < 4 {
        return None;
    }
    let recent: f64 = values.iter().rev().take(3).sum::<f64>() / 3.0;
    let older: Vec<f64> = values.iter().rev().skip(3).take(7).copied().collect();
    if older.is_empty() {
        return None;
    }
    let older_mean = older.iter().sum::<f64>() / older.len() as f64;
    Some(recent - older_mean)
}

/// Whether a resting-HR drift is large enough to matter.
#[must_use]
pub fn drift_is_concerning(drift: Option<f64>) -> bool {
    drift.is_some_and(|d| d >= 4.0)
}

/// Convenience: readiness for a day where the resting-HR trend should also
/// pull the score down.
///
/// Kept separate from [`readiness_for`] so the pure component blend stays
/// testable in isolation.
#[must_use]
pub fn readiness_with_drift(day: &HealthDay, history: &[HealthDay]) -> Readiness {
    let mut readiness = readiness_for(day, history);
    if let Some(drift) = resting_hr_drift(history) {
        // Each bpm of sustained elevation costs 3 points, capped so a genuine
        // illness reading cannot drive the score to zero on its own.
        let penalty = (drift.max(0.0) * 3.0).min(20.0);
        readiness.score = (readiness.score - penalty).clamp(0.0, 100.0);
    }
    readiness
}

/// The trailing window used to build baselines.
pub const BASELINE_WINDOW_DAYS: usize = 28;

/// Trim history to the trailing window used for baselines.
#[must_use]
pub fn baseline_window(history: &[HealthDay]) -> Vec<HealthDay> {
    history
        .iter()
        .rev()
        .take(BASELINE_WINDOW_DAYS)
        .rev()
        .cloned()
        .collect()
}

/// A day with no recovery signal at all cannot support a readiness score.
#[must_use]
pub fn day_is_scoreable(day: &HealthDay) -> bool {
    day.has_recovery_signal()
}

#[cfg(test)]
mod tests {
    use super::*;
    use runalytics_core::{DurationSecs, HeartRate, SleepDay};

    fn date(d: u32) -> Date {
        Date::from_ymd_opt(2026, 10, 1)
            .expect("base")
            .checked_add_days(chrono::Days::new(u64::from(d)))
            .expect("in range")
    }

    fn sleep(total_min: u32) -> SleepDay {
        SleepDay {
            date: date(0),
            total: DurationSecs::from_minutes(total_min),
            deep: DurationSecs::from_minutes(total_min / 5),
            light: DurationSecs::from_minutes(total_min / 2),
            rem: DurationSecs::from_minutes(total_min / 4),
            awake: DurationSecs::ZERO,
            nap: DurationSecs::ZERO,
            score: None,
            lowest_hr: None,
            hrv: None,
            respiratory_rate: None,
        }
    }

    fn day(d: u32, rest_hr: u16, hrv: f64, sleep_min: u32) -> HealthDay {
        let mut s = sleep(sleep_min);
        s.date = date(d);
        s.hrv = Some(hrv);
        HealthDay {
            id: runalytics_core::HealthDayId::new(),
            account: runalytics_core::ProviderAccountId::new(),
            date: date(d),
            resting_hr: Some(HeartRate::new(rest_hr)),
            avg_stress: Some(30.0),
            high_stress_minutes: None,
            sleep: Some(s),
            steps: Some(8000),
            provider_readiness: None,
            basal_energy: None,
        }
    }

    /// A realistic history: a real athlete's resting HR and HRV wobble day to
    /// day, and the mean of the requested values is preserved.
    ///
    /// This matters more than it looks. A perfectly flat history has a standard
    /// deviation of zero, and a z-score against a zero spread is undefined — so
    /// the model correctly abstains on every HR component and readiness is
    /// decided by sleep alone. Fixtures that are too clean therefore test a
    /// model that is not the one shipping.
    fn history(n: usize, rest_hr: u16, hrv: f64) -> Vec<HealthDay> {
        const REST_OFFSETS: [i16; 3] = [-1, 0, 1];
        const HRV_OFFSETS: [f64; 3] = [-5.0, 0.0, 5.0];
        (0..n)
            .map(|d| {
                let i = d % REST_OFFSETS.len();
                day(
                    d as u32,
                    u16::try_from(i32::from(rest_hr) + i32::from(REST_OFFSETS[i]))
                        .expect("in range"),
                    hrv + HRV_OFFSETS[i],
                    450,
                )
            })
            .collect()
    }

    #[test]
    fn baseline_needs_two_samples_for_a_spread() {
        let thin = Baseline::new(&[50.0]).expect("one sample");
        assert_eq!(thin.sd, 0.0);
        assert!(thin.z_score(60.0).is_none());
        let wide = Baseline::new(&[40.0, 60.0]).expect("two samples");
        assert!((wide.mean - 50.0).abs() < 1e-9);
        assert!((wide.z_score(60.0).expect("z") - std::f64::consts::FRAC_1_SQRT_2).abs() < 0.01);
        assert!(Baseline::new(&[]).is_none());
    }

    #[test]
    fn on_baseline_scores_comfortable_not_perfect() {
        let score = score_from_z(0.0, Direction::HigherIsBetter);
        assert!((score - 0.75).abs() < 1e-9);
    }

    #[test]
    fn upside_saturates_before_downside() {
        let good = score_from_z(4.0, Direction::HigherIsBetter);
        let bad = score_from_z(-4.0, Direction::HigherIsBetter);
        assert!((good - 1.0).abs() < 1e-9, "capped at 1.0, got {good}");
        assert!(bad < 0.1, "steep downside, got {bad}");
    }

    #[test]
    fn direction_flips_the_meaning_of_a_z_score() {
        // A resting HR 2 SD *above* baseline is bad.
        let high_rest = score_from_z(2.0, Direction::LowerIsBetter);
        let high_hrv = score_from_z(2.0, Direction::HigherIsBetter);
        assert!(high_rest < 0.75);
        assert!(high_hrv > 0.75);
    }

    #[test]
    fn full_sleep_scores_high() {
        let d = day(0, 55, 60.0, 480); // 8 h
        assert!(score_sleep(&d).expect("sleep") > 0.95);
    }

    #[test]
    fn short_sleep_scores_lower() {
        let short = day(0, 55, 60.0, 300); // 5 h
        let long = day(0, 55, 60.0, 480); // 8 h
        assert!(score_sleep(&short).expect("s") < score_sleep(&long).expect("l"));
    }

    #[test]
    fn missing_sleep_drops_the_sleep_component() {
        let mut d = day(0, 55, 60.0, 450);
        d.sleep = None;
        assert!(score_sleep(&d).is_none());
    }

    #[test]
    fn a_depleted_day_scores_below_a_normal_one() {
        let hist = history(28, 55, 60.0);
        let normal = readiness_for(&day(28, 55, 60.0, 450), &hist);
        let depleted = readiness_for(&day(28, 63, 25.0, 280), &hist);
        assert!(
            depleted.score < normal.score - 15.0,
            "depleted {} should be well below normal {}",
            depleted.score,
            normal.score
        );
    }

    #[test]
    fn no_data_at_all_is_neutral_not_zero() {
        let empty = ReadinessComponents::default();
        assert_eq!(readiness_score(&empty), 50.0);
        assert_eq!(empty.coverage(), 0.0);
    }

    #[test]
    fn partial_data_reduces_confidence_not_inflates_score() {
        let hist = history(28, 55, 60.0);
        let full = readiness_for(&day(28, 55, 60.0, 450), &hist);
        let mut thin_day = day(28, 55, 60.0, 450);
        thin_day.sleep = None; // lose sleep + hrv + lowest-hr
        let thin = readiness_for(&thin_day, &hist);
        assert!(thin.confidence < full.confidence);
        // Losing the good sleep signal must not make the athlete look *better*.
        assert!(thin.score <= full.score + 1.0);
    }

    #[test]
    fn coverage_is_the_sum_of_present_weights() {
        let c = ReadinessComponents {
            sleep: Some(0.9),
            hrv: Some(0.8),
            ..Default::default()
        };
        assert!((c.coverage() - 0.60).abs() < 1e-9);
    }

    #[test]
    fn bands_partition_the_scale() {
        assert_eq!(ReadinessBand::from_score(90.0), ReadinessBand::Primed);
        assert_eq!(ReadinessBand::from_score(70.0), ReadinessBand::Ready);
        assert_eq!(ReadinessBand::from_score(50.0), ReadinessBand::Guarded);
        assert_eq!(ReadinessBand::from_score(10.0), ReadinessBand::Depleted);
        assert_eq!(ReadinessBand::from_score(0.0).as_str(), "depleted");
    }

    #[test]
    fn rising_resting_hr_shows_as_positive_drift() {
        let mut hist = history(10, 55, 60.0);
        // Last three days climb.
        for day in hist.iter_mut().skip(7) {
            day.resting_hr = Some(HeartRate::new(62));
        }
        let drift = resting_hr_drift(&hist).expect("drift");
        assert!(drift > 4.0, "expected a rise, got {drift}");
        assert!(drift_is_concerning(Some(drift)));
    }

    #[test]
    fn stable_resting_hr_is_not_concerning() {
        let hist = history(14, 55, 60.0);
        let drift = resting_hr_drift(&hist).expect("drift");
        assert!(drift.abs() < 0.5, "flat history, got {drift}");
        assert!(!drift_is_concerning(Some(drift)));
    }

    #[test]
    fn too_few_samples_give_no_drift() {
        assert!(resting_hr_drift(&history(3, 55, 60.0)).is_none());
        assert!(resting_hr_drift(&[]).is_none());
    }

    #[test]
    fn drift_penalty_lowers_readiness_but_cannot_zero_it() {
        let mut hist = history(28, 55, 60.0);
        for day in hist.iter_mut().skip(25) {
            day.resting_hr = Some(HeartRate::new(80));
        }
        let with = readiness_with_drift(&day(28, 55, 60.0, 450), &hist);
        let without = readiness_for(&day(28, 55, 60.0, 450), &hist);
        assert!(with.score < without.score);
        assert!(with.score > 0.0, "penalty is capped, got {}", with.score);
    }

    #[test]
    fn steps_alone_are_not_scoreable() {
        let mut d = day(0, 55, 60.0, 450);
        d.resting_hr = None;
        d.avg_stress = None;
        d.sleep = None;
        assert!(!day_is_scoreable(&d));
        assert!(day_is_scoreable(&day(0, 55, 60.0, 450)));
    }

    #[test]
    fn baseline_window_keeps_the_trailing_days() {
        let hist = history(60, 55, 60.0);
        let window = baseline_window(&hist);
        assert_eq!(window.len(), BASELINE_WINDOW_DAYS);
        assert_eq!(
            window.last().expect("last").date,
            hist.last().expect("l").date
        );
    }

    #[test]
    fn score_is_always_in_range() {
        let hist = history(28, 55, 60.0);
        for (rest, hrv, mins) in [(40, 200.0, 700), (90, 5.0, 120), (55, 60.0, 450)] {
            let r = readiness_for(&day(28, rest, hrv, mins), &hist);
            assert!(
                (0.0..=100.0).contains(&r.score),
                "out of range: {}",
                r.score
            );
        }
    }
}
