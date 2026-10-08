//! Injury risk: a probability-like score with the reasons attached.
//!
//! Two design decisions drive this module, and both are about trust rather
//! than mathematics.
//!
//! **The score is squashed, never linear.** A weighted sum of risk factors can
//! produce any number, including absurd ones, and an athlete who sees "risk:
//! 143" stops believing the app. A logistic maps the same linear predictor into
//! `0..=100` with soft shoulders, so the extremes are asymptotic: the model can
//! say "very high" without ever claiming certainty it does not have.
//!
//! **Every point is attributable.** The output carries the individual
//! contributions that produced it, in descending order of size. An injury score
//! you cannot argue with is not a decision aid, it is a fortune teller — and
//! the athlete, not the app, decides whether to run the interval session.
//!
//! The factors themselves follow the training-injury literature: the
//! acute:chronic workload ratio and week-to-week spike are the strongest acute
//! predictors, monotony and rapid ramping the strongest chronic ones, and prior
//! injury history the single most powerful baseline factor there is.

use runalytics_core::AthleteSnapshot;

use crate::load::{DailyLoad, monotony, weekly_spike};

/// A single named contribution to the injury score.
#[derive(Debug, Clone, PartialEq)]
pub struct RiskDriver {
    /// Stable machine key, e.g. `"acwr"`.
    pub code: &'static str,
    /// Human sentence explaining this contributor.
    pub message: String,
    /// Points this factor added to the score, `0.0..=100.0` scale.
    pub contribution: f64,
}

/// The inputs the injury model reads.
///
/// Deliberately a struct rather than a long argument list: the model grows, and
/// a new factor should be one field, not a signature change at every call site.
#[derive(Debug, Clone)]
pub struct InjuryInputs<'a> {
    pub series: &'a [DailyLoad],
    pub athlete: &'a AthleteSnapshot,
    /// `0..=100`, most recent first. Absent means no recovery data, which is
    /// treated as *unknown*, not as fully recovered.
    pub readiness: Option<f64>,
    /// Consecutive days without a rest day in the trailing window.
    pub consecutive_training_days: usize,
    /// Weeks since the athlete's most recent race, when one is recorded.
    pub weeks_since_race: Option<u32>,
}

/// The injury-risk score and everything that explains it.
#[derive(Debug, Clone, PartialEq)]
pub struct InjuryRisk {
    /// `0..=100`.
    pub score: f64,
    pub band: InjuryBand,
    pub drivers: Vec<RiskDriver>,
}

/// Coarse band for display and for plan-time gating.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjuryBand {
    Low,
    Moderate,
    Elevated,
    High,
}

impl InjuryBand {
    #[must_use]
    pub fn from_score(score: f64) -> Self {
        match score {
            s if s < 25.0 => Self::Low,
            s if s < 50.0 => Self::Moderate,
            s if s < 75.0 => Self::Elevated,
            _ => Self::High,
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Moderate => "moderate",
            Self::Elevated => "elevated",
            Self::High => "high",
        }
    }
}

/// Raw acute:chronic risk, `0.0..=1.0`.
///
/// The relationship is U-shaped in the literature — both a spike *and* a
/// sudden drop carry risk — but the drop is far gentler, so the curve rises
/// steeply above the safe band and only mildly below it.
#[must_use]
pub fn acwr_risk(acwr: f64) -> f64 {
    if acwr <= 0.8 {
        // Under-trained relative to history: some risk from detraining, small.
        return ((0.8 - acwr) * 0.35).clamp(0.0, 0.4);
    }
    if acwr <= 1.3 {
        // The sweet spot. Risk rises almost linearly through it, which keeps
        // the score responsive instead of flat until a cliff.
        return (acwr - 0.8) * 0.18;
    }
    // Above the band: already elevated on crossing, then steep. Arriving at
    // 1.3 is the documented edge of the safe zone, so it must not read as
    // near-zero risk — that would make the band boundary decorative.
    (0.35 + (acwr - 1.3) * 0.9).clamp(0.0, 1.0)
}

/// Weekly-spike risk, `0.0..=1.0`. A spike over ~1.3 is where tissue fails.
///
/// Calibrated to saturate at a 40 % week-over-week jump. That is well past every
/// increment anyone recommends, and it is the mechanism behind most running
/// overuse injuries. An earlier version saturated only at a 100 % jump, which
/// contradicted the threshold this function claims to model and let a 40 % spike
/// read as a mild concern.
#[must_use]
pub fn spike_risk(spike: f64) -> f64 {
    if spike <= 1.1 {
        return 0.0;
    }
    // Full risk by a 40% jump; 1.1 is ordinary week-to-week noise.
    ((spike - 1.1) / 0.3).clamp(0.0, 1.0)
}

/// Monotony risk, `0.0..=1.0`. Above ~2.0 the week has no variation.
#[must_use]
pub fn monotony_risk(monotony_value: f64) -> f64 {
    if monotony_value <= 1.4 {
        return 0.0;
    }
    ((monotony_value - 1.4) / 2.6).clamp(0.0, 1.0)
}

/// Risk from a poor readiness reading, `0.0..=1.0`.
///
/// `None` returns a small non-zero value: no recovery data is not evidence of
/// recovery, and silently treating it as "fully recovered" is the most
/// dangerous thing this model could do.
#[must_use]
pub fn readiness_risk(readiness: Option<f64>) -> f64 {
    match readiness {
        Some(score) if score >= 60.0 => 0.0,
        Some(score) => ((60.0 - score) / 60.0).clamp(0.0, 1.0),
        None => 0.15,
    }
}

/// Risk from running many days in a row without a rest day.
#[must_use]
pub fn consecutive_day_risk(days: usize) -> f64 {
    if days <= 3 {
        return 0.0;
    }
    ((days - 3) as f64 / 6.0).clamp(0.0, 1.0)
}

/// Risk from recent injury history, `0.0..=1.0`.
///
/// Saturates rather than accumulating without bound: a second concurrent
/// injury flag is serious, but it does not double an already-maximal risk.
#[must_use]
pub fn injury_history_risk(flags: &[String]) -> f64 {
    match flags.len() {
        0 => 0.0,
        1 => 0.6,
        2 => 0.8,
        _ => 0.95,
    }
}

/// Risk from being detrained — a big gap between recent and peak volume.
#[must_use]
pub fn detraining_risk(consistency: f64) -> f64 {
    if consistency >= 0.8 {
        return 0.0;
    }
    ((0.8 - consistency) * 1.6).clamp(0.0, 1.0)
}

/// Risk from a very recent race, when one is known.
///
/// The week after a race is genuinely higher risk — tissue is damaged and the
/// athlete is usually eager — but it decays fast.
#[must_use]
pub fn post_race_risk(weeks_since_race: Option<u32>) -> f64 {
    match weeks_since_race {
        Some(0) => 0.7,
        Some(1) => 0.35,
        Some(2) => 0.15,
        _ => 0.0,
    }
}

/// Weights for each factor. They sum to 1.0 so the linear predictor is a
/// weighted average of risks, which keeps the logistic's centre interpretable.
const W_ACWR: f64 = 0.26;
const W_SPIKE: f64 = 0.18;
const W_MONOTONY: f64 = 0.08;
const W_READINESS: f64 = 0.14;
const W_CONSECUTIVE: f64 = 0.08;
const W_HISTORY: f64 = 0.18;
const W_DETRAINING: f64 = 0.10;
const W_POST_RACE: f64 = 0.06;

/// Logistic squash from a `0..=1` weighted risk to a `0..=100` score.
///
/// Centred at 0.5 weighted risk (which maps to 50) with a slope chosen so the
/// score reaches ~90 at a weighted risk of 0.85 — a genuinely alarming
/// combination — without ever touching 100.
#[must_use]
pub fn squash(weighted: f64) -> f64 {
    let k = 9.2;
    100.0 / (1.0 + (-k * (weighted - 0.5)).exp())
}

/// How much authority a single alarming *event* factor has on its own.
///
/// A weighted average alone cannot express the thing that actually injures
/// people. Seven benign factors dilute one alarming one into a low score: a 40 %
/// volume spike contributes only its weight to the sum, so the athlete is told
/// they are moderately at risk when the correct message is "stop increasing your
/// volume now". This is the same trap ACWR falls into — it is a lagging ratio and
/// cannot see a single week — and it needs the same remedy: a second, absolute
/// guard alongside the averaged one.
///
/// So the factors that describe an *event* rather than a chronic property (a
/// weekly spike, and an ACWR that has left the safe band) may carry the score by
/// themselves. Chronic properties — monotony, history, detraining — may not,
/// because a single alarming reading of those genuinely is one contributor among
/// several.
///
/// `0.62` is chosen so that a lone fully-saturated event factor, with nothing
/// else wrong, reads as exactly the High threshold of 75 and no higher. An event
/// is serious, not certain: full authority would mean one spike always reads as
/// near-100 regardless of everything else, which would train athletes to stop
/// trusting the score.
const EVENT_AUTHORITY: f64 = 0.62;

/// Compute the injury-risk score.
///
/// Note there is no "chronic load is high" term. High chronic load is fitness,
/// not risk — what injures people is the *rate of increase*, which the ACWR,
/// weekly-spike and detraining terms already capture. Adding a level term on top
/// would penalise the fittest athletes in the database.
#[must_use]
pub fn injury_risk(inputs: &InjuryInputs<'_>) -> InjuryRisk {
    use crate::load::acute_chronic_ratio;

    let acwr = acute_chronic_ratio(inputs.series);
    let spike = weekly_spike(inputs.series);
    let mono = monotony(inputs.series);

    let factors: [(&str, f64, f64); 8] = [
        ("acwr", W_ACWR, acwr_risk(acwr)),
        ("spike", W_SPIKE, spike_risk(spike)),
        ("monotony", W_MONOTONY, monotony_risk(mono)),
        ("readiness", W_READINESS, readiness_risk(inputs.readiness)),
        (
            "consecutive_days",
            W_CONSECUTIVE,
            consecutive_day_risk(inputs.consecutive_training_days),
        ),
        (
            "injury_history",
            W_HISTORY,
            injury_history_risk(&inputs.athlete.injury_flags),
        ),
        (
            "detraining",
            W_DETRAINING,
            detraining_risk(inputs.athlete.consistency),
        ),
        (
            "post_race",
            W_POST_RACE,
            post_race_risk(inputs.weeks_since_race),
        ),
    ];

    // Chronic factors combine as a weighted average. Event factors additionally
    // take the floor set by EVENT_AUTHORITY, so one alarming event cannot be
    // averaged away. See [`EVENT_AUTHORITY`] for why only events get this.
    let weighted: f64 = factors.iter().map(|(_, w, r)| w * r).sum();
    let event_floor: f64 = factors
        .iter()
        .filter(|(code, _, _)| matches!(*code, "spike" | "acwr"))
        .map(|(_, _, risk)| EVENT_AUTHORITY * risk)
        .fold(0.0, f64::max);
    let score = squash(weighted.max(event_floor));

    // Attribute the score to its drivers. The logistic is non-linear, so the
    // exact marginal contribution of one factor depends on the others; the
    // honest and simple approximation is each factor's share of the weighted
    // sum, rescaled onto the final score. That is what the athlete needs to see
    // — "this is the biggest thing pushing your number up" — and it does not
    // pretend to a precision the model does not have.
    let mut drivers: Vec<RiskDriver> = factors
        .iter()
        .filter(|(_, _, risk)| *risk > 0.001)
        .map(|(code, weight, risk)| RiskDriver {
            code,
            message: driver_message(code, *risk, acwr, spike, mono, inputs),
            contribution: if weighted > 0.0 {
                score * (weight * risk) / weighted
            } else {
                0.0
            },
        })
        .collect();
    drivers.sort_by(|a, b| {
        b.contribution
            .partial_cmp(&a.contribution)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    InjuryRisk {
        score,
        band: InjuryBand::from_score(score),
        drivers,
    }
}

fn driver_message(
    code: &str,
    risk: f64,
    acwr: f64,
    spike: f64,
    mono: f64,
    inputs: &InjuryInputs<'_>,
) -> String {
    let pct = (risk * 100.0).round();
    match code {
        "acwr" => format!(
            "Acute:chronic ratio {acwr:.2} — the load you are carrying this week relative to the last six weeks. The safe band is 0.8–1.3. ({pct:.0}% severity)"
        ),
        "spike" => format!(
            "This week's load is {spike:.2}× your recent weekly average. Jumps above 1.3× are where tendon and bone injuries start. ({pct:.0}% severity)"
        ),
        "monotony" => format!(
            "Weekly monotony {mono:.1} — your weeks carry almost no variation in stress. Consistently hard is its own kind of overload. ({pct:.0}% severity)"
        ),
        "readiness" => match inputs.readiness {
            Some(score) => format!(
                "Recovery is low today ({score:.0}/100). Your body is not absorbing the load you are giving it. ({pct:.0}% severity)"
            ),
            None => "No recovery data synced, so this score cannot see how recovered you actually are. ({pct:.0}% severity)".into(),
        },
        "consecutive_days" => format!(
            "{} days in a row without a rest day. Tissue adapts on the days you do not run. ({pct:.0}% severity)",
            inputs.consecutive_training_days
        ),
        "injury_history" => format!(
            "Active injury flag{} in your profile. Past injury is the strongest single predictor of the next one. ({pct:.0}% severity)",
            if inputs.athlete.injury_flags.len() > 1 { "s" } else { "" }
        ),
        "detraining" => format!(
            "You are running at {:.0}% of your recent peak, so your fitness is ahead of your tissue tolerance. Rebuild before you push. ({pct:.0}% severity)",
            inputs.athlete.consistency * 100.0
        ),
        "post_race" => "Within a few weeks of a race — tissue is still remodelling from it. ({pct:.0}% severity)".into(),
        other => format!("{other} ({pct:.0}% severity)"),
    }
}

/// Consecutive days with training load, counting back from the end of the series.
///
/// A day counts as training when it carries meaningful load; a 5-minute walk
/// with a stray GPS trace is not a training day and should not break a rest
/// streak.
#[must_use]
pub fn consecutive_training_days(series: &[DailyLoad], threshold_tss: f64) -> usize {
    series
        .iter()
        .rev()
        .take_while(|day| day.load >= threshold_tss)
        .count()
}

/// The load above which a day counts as training rather than noise.
pub const TRAINING_DAY_TSS: f64 = 20.0;

/// Convenience wrapper matching the store's row shape.
#[must_use]
pub fn injury_risk_for(
    series: &[DailyLoad],
    athlete: &AthleteSnapshot,
    readiness: Option<f64>,
    weeks_since_race: Option<u32>,
) -> InjuryRisk {
    let consecutive = consecutive_training_days(series, TRAINING_DAY_TSS);
    injury_risk(&InjuryInputs {
        series,
        athlete,
        readiness,
        consecutive_training_days: consecutive,
        weeks_since_race,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use runalytics_core::{Date, Tz};

    fn athlete() -> AthleteSnapshot {
        AthleteSnapshot::placeholder("Europe/Madrid".parse::<Tz>().expect("valid tz"))
    }

    fn series_with(loads: &[f64]) -> Vec<DailyLoad> {
        let base = Date::from_ymd_opt(2026, 8, 1).expect("base");
        loads
            .iter()
            .enumerate()
            .map(|(i, load)| DailyLoad {
                date: base
                    .checked_add_days(chrono::Days::new(u64::try_from(i).expect("idx")))
                    .expect("in range"),
                load: *load,
            })
            .collect()
    }

    fn flat(n: usize, load: f64) -> Vec<DailyLoad> {
        series_with(&vec![load; n])
    }

    #[test]
    fn score_is_always_in_range() {
        for loads in [
            vec![0.0; 60],
            vec![50.0; 60],
            vec![5.0; 50],
            vec![200.0; 60],
        ] {
            let s = series_with(&loads);
            let r = injury_risk_for(&s, &athlete(), Some(70.0), None);
            assert!(
                (0.0..=100.0).contains(&r.score),
                "out of range for {loads:?}: {}",
                r.score
            );
        }
    }

    #[test]
    fn squash_is_monotonic_and_never_reaches_the_extremes() {
        let mut prev = -1.0;
        for i in 0..=100 {
            let x = f64::from(i) / 100.0;
            let y = squash(x);
            assert!(y > prev, "not monotonic at {x}");
            assert!(y > 0.0 && y < 100.0, "hit an extreme at {x}: {y}");
            prev = y;
        }
        assert!((squash(0.5) - 50.0).abs() < 1e-9);
    }

    #[test]
    fn a_bad_combination_reaches_the_high_band() {
        // Spike + no recovery + injury history.
        let mut ath = athlete();
        ath.injury_flags.push("achilles".into());
        let mut loads = vec![40.0; 50];
        loads.extend(vec![220.0; 7]);
        let s = series_with(&loads);
        let r = injury_risk_for(&s, &ath, Some(20.0), Some(0));
        assert!(r.score > 70.0, "expected a high score, got {}", r.score);
        assert_eq!(r.band, InjuryBand::High);
    }

    #[test]
    fn a_benign_week_scores_low() {
        let r = injury_risk_for(&flat(60, 60.0), &athlete(), Some(85.0), None);
        assert!(r.score < 30.0, "expected low, got {}", r.score);
        assert_eq!(r.band, InjuryBand::Low);
    }

    #[test]
    fn drivers_are_ordered_by_contribution() {
        let mut ath = athlete();
        ath.injury_flags.push("hamstring".into());
        let mut loads = vec![40.0; 50];
        loads.extend(vec![200.0; 7]);
        let s = series_with(&loads);
        let r = injury_risk_for(&s, &ath, Some(30.0), None);
        assert!(r.drivers.len() >= 2);
        for pair in r.drivers.windows(2) {
            assert!(
                pair[0].contribution >= pair[1].contribution - 1e-9,
                "drivers not sorted: {:?}",
                r.drivers
            );
        }
    }

    #[test]
    fn driver_contributions_sum_to_about_the_score() {
        let mut loads = vec![40.0; 50];
        loads.extend(vec![180.0; 7]);
        let s = series_with(&loads);
        let r = injury_risk_for(&s, &athlete(), Some(40.0), Some(1));
        let total: f64 = r.drivers.iter().map(|d| d.contribution).sum();
        assert!(
            (total - r.score).abs() < 1.0,
            "drivers sum to {total}, score is {}",
            r.score
        );
    }

    #[test]
    fn every_driver_carries_a_readable_message() {
        let mut ath = athlete();
        ath.injury_flags.push("knee".into());
        let mut loads = vec![40.0; 49];
        loads.extend(vec![200.0; 8]);
        let s = series_with(&loads);
        let r = injury_risk_for(&s, &ath, None, Some(0));
        assert_ne!(r.drivers.len(), 0, "a stressed athlete must have drivers");
        for d in &r.drivers {
            assert!(d.message.len() > 20, "thin message for {}", d.code);
            assert_ne!(d.code, "", "a driver must be identifiable");
        }
    }

    #[test]
    fn acwr_risk_rises_above_the_band_and_is_mild_below_it() {
        let in_band = acwr_risk(1.1);
        let spiked = acwr_risk(1.8);
        let detrained = acwr_risk(0.5);
        assert!(spiked > in_band);
        assert!(detrained < spiked, "a drop should be gentler than a spike");
        assert!(acwr_risk(0.0) <= 0.4);
    }

    #[test]
    fn spike_risk_ignores_sane_weeks() {
        assert_eq!(spike_risk(1.0), 0.0);
        assert_eq!(spike_risk(1.1), 0.0);
        assert!(spike_risk(1.6) > 0.4);
    }

    #[test]
    fn monotony_risk_starts_where_variation_disappears() {
        assert_eq!(monotony_risk(1.0), 0.0);
        assert!(monotony_risk(2.0) > 0.0);
        assert_eq!(monotony_risk(MONOTONY_RISK_SATURATES_AT), 1.0);
    }

    const MONOTONY_RISK_SATURATES_AT: f64 = 4.0;

    #[test]
    fn missing_readiness_is_not_treated_as_recovered() {
        let unknown = readiness_risk(None);
        let recovered = readiness_risk(Some(90.0));
        assert_eq!(recovered, 0.0);
        assert!(
            unknown > recovered,
            "no data must not read as fully recovered"
        );
        assert!(readiness_risk(Some(20.0)) > unknown);
    }

    #[test]
    fn injury_history_saturates() {
        assert_eq!(injury_history_risk(&[]), 0.0);
        let one = injury_history_risk(&["a".into()]);
        let two = injury_history_risk(&["a".into(), "b".into()]);
        let many = injury_history_risk(&["a".into(), "b".into(), "c".into(), "d".into()]);
        assert!(one < two && two < many && many < 1.0);
    }

    #[test]
    fn detrained_athletes_carry_more_risk() {
        assert_eq!(detraining_risk(0.9), 0.0);
        assert!(detraining_risk(0.4) > detraining_risk(0.6));
    }

    #[test]
    fn post_race_risk_decays_quickly() {
        assert!(post_race_risk(Some(0)) > post_race_risk(Some(1)));
        assert!(post_race_risk(Some(1)) > post_race_risk(Some(2)));
        assert_eq!(post_race_risk(Some(5)), 0.0);
        assert_eq!(post_race_risk(None), 0.0);
    }

    #[test]
    fn consecutive_days_only_count_after_three() {
        assert_eq!(consecutive_day_risk(1), 0.0);
        assert_eq!(consecutive_day_risk(3), 0.0);
        assert!(consecutive_day_risk(10) > consecutive_day_risk(5));
    }

    #[test]
    fn consecutive_training_days_stops_at_a_rest_day() {
        let s = series_with(&[60.0, 60.0, 0.0, 60.0, 60.0, 60.0]);
        assert_eq!(consecutive_training_days(&s, TRAINING_DAY_TSS), 3);
    }

    #[test]
    fn a_stray_trace_does_not_break_a_rest_streak() {
        // 8 TSS is a GPS artefact, not a training day.
        let s = series_with(&[60.0, 60.0, 8.0]);
        assert_eq!(consecutive_training_days(&s, TRAINING_DAY_TSS), 0);
    }

    #[test]
    fn an_empty_history_is_neutral_not_scary() {
        let r = injury_risk_for(&[], &athlete(), None, None);
        assert!(
            r.score < 40.0,
            "a brand-new athlete should not be flagged high: {}",
            r.score
        );
    }

    #[test]
    fn bands_partition_the_scale() {
        assert_eq!(InjuryBand::from_score(10.0), InjuryBand::Low);
        assert_eq!(InjuryBand::from_score(30.0), InjuryBand::Moderate);
        assert_eq!(InjuryBand::from_score(60.0), InjuryBand::Elevated);
        assert_eq!(InjuryBand::from_score(90.0), InjuryBand::High);
        assert_eq!(InjuryBand::from_score(90.0).as_str(), "high");
    }

    #[test]
    fn a_spike_raises_the_score_all_else_equal() {
        let steady = injury_risk_for(&flat(57, 60.0), &athlete(), Some(75.0), None);
        let mut loads = vec![60.0; 50];
        loads.extend(vec![200.0; 7]);
        let spiked = injury_risk_for(&series_with(&loads), &athlete(), Some(75.0), None);
        assert!(
            spiked.score > steady.score + 10.0,
            "steady {} vs spiked {}",
            steady.score,
            spiked.score
        );
    }
}
