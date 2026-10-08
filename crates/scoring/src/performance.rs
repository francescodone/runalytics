//! Performance: how fast the athlete is becoming, independent of how fresh
//! they are on the day.
//!
//! The distinction is the whole point. A Tuesday interval session run on three
//! hours of sleep tells you almost nothing about whether the athlete is
//! improving; the same session run in a normal state tells you a great deal.
//! So every signal here is either taken from a *representative* effort or
//! smoothed hard enough that the bad days wash out.
//!
//! Three signals, in descending order of trustworthiness:
//!
//! * **Race results** — the ground truth. One 10 k time trial beats a month of
//!   estimated thresholds, so when a race exists it dominates.
//! * **Critical speed / threshold pace** — derived from the best efforts at
//!   durations of 3–20 minutes, which is where a maximal aerobic effort is
//!   measurable without needing a lab.
//! * **VO2max trend** — the provider's own estimate, used as a corroborating
//!   direction rather than an absolute, because providers disagree wildly on the
//!   number while agreeing reasonably on whether it is going up.
//!
//! The composite `performance_index` is a `0..=100` percentile-style score
//! against the athlete's *own* history, not a population norm. Runalytics does
//! not know whether you are fast; it knows whether you are faster than last
//! month, which is the only comparison it can honestly make.

use runalytics_core::{Activity, Date, Pace, VolumeKm};

/// The longest effort duration used for critical-speed fitting, seconds.
pub const MAX_EFFORT_SECONDS: u32 = 1200;
/// The shortest effort duration used — below this, anaerobic capacity dominates
/// and the estimate stops meaning "aerobic fitness".
pub const MIN_EFFORT_SECONDS: u32 = 180;

/// An effort extracted from an activity for performance modelling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Effort {
    pub date: Date,
    pub duration_s: u32,
    pub distance_km: f64,
}

impl Effort {
    #[must_use]
    pub fn pace(&self) -> Option<Pace> {
        if self.distance_km < 0.05 {
            return None;
        }
        Some(Pace::new(f64::from(self.duration_s) / self.distance_km))
    }
}

/// The best raw pace among a set of efforts, and the fallback whenever a fit is
/// impossible.
///
/// Honest about its weakness: with one point there is no line to fit, and that
/// one point may be a tailwind. The caller has no better option, so the result is
/// reported plainly rather than dressed up as a model output.
fn best_raw_pace(efforts: &[&Effort]) -> Option<Pace> {
    efforts.iter().filter_map(|e| e.pace()).min_by(|a, b| {
        a.as_secs_per_km()
            .partial_cmp(&b.as_secs_per_km())
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

/// Best average pace over a target duration, from a set of efforts.
///
/// Uses the two-parameter critical-speed model, `distance = CS·(t − W′)`,
/// rather than a raw best-average-pace lookup. The reason is that raw best
/// efforts are noisy in exactly the direction that flatters: the one day an
/// athlete caught a tailwind becomes their "threshold" forever. The fit uses
/// every qualifying effort and is anchored by the shape of the distance–duration
/// relationship.
///
/// Falls back to the best raw pace when fewer than two qualifying efforts
/// exist, which is honest: with one point there is no line to fit.
#[must_use]
pub fn best_effort_pace(efforts: &[Effort], target_seconds: u32) -> Option<Pace> {
    let qualifying: Vec<&Effort> = efforts
        .iter()
        .filter(|e| e.duration_s >= MIN_EFFORT_SECONDS && e.duration_s <= MAX_EFFORT_SECONDS)
        .filter(|e| e.distance_km > 0.05)
        .collect();
    if qualifying.is_empty() {
        return None;
    }
    if qualifying.len() < 2 {
        return best_raw_pace(&qualifying);
    }

    // Least-squares fit of d = CS·t − CS·W′, i.e. a line d = a·t + b with
    // slope a = CS and intercept b = −CS·W′. Threshold pace is the pace at
    // critical speed, `1/CS` km/s converted to s/km.
    let n = qualifying.len() as f64;
    let sum_t: f64 = qualifying.iter().map(|e| f64::from(e.duration_s)).sum();
    let sum_d: f64 = qualifying.iter().map(|e| e.distance_km).sum();
    let sum_tt: f64 = qualifying
        .iter()
        .map(|e| f64::from(e.duration_s).powi(2))
        .sum();
    let sum_t_d: f64 = qualifying
        .iter()
        .map(|e| f64::from(e.duration_s) * e.distance_km)
        .sum();
    let denom = n * sum_tt - sum_t * sum_t;
    if denom.abs() < 1e-9 {
        // All efforts the same duration — no slope to recover.
        return best_raw_pace(&qualifying);
    }
    let slope_km_per_s = (n * sum_t_d - sum_t * sum_d) / denom;
    if slope_km_per_s <= 1e-6 {
        return None;
    }
    // Critical speed is the asymptotic pace; the effort actually sustained at
    // `target_seconds` is slightly slower, and reporting CS directly would
    // overstate fitness. Solve the model at the target duration instead.
    let intercept = (sum_d - slope_km_per_s * sum_t) / n;
    let distance_at_target = slope_km_per_s * f64::from(target_seconds) + intercept;
    if distance_at_target <= 0.05 {
        return None;
    }
    Some(Pace::new(f64::from(target_seconds) / distance_at_target))
}

/// Extract qualifying efforts from activities.
///
/// Whole activities only. Splitting a session into best-3-minute windows needs
/// lap-level data and is a later refinement; using the whole session is
/// conservative in the right direction — it can understate a threshold but
/// cannot invent one from a lucky 400 m.
#[must_use]
pub fn efforts_from(activities: &[Activity]) -> Vec<Effort> {
    activities
        .iter()
        .filter(|a| a.summary.duration.as_u32() >= MIN_EFFORT_SECONDS)
        .map(|a| Effort {
            date: a.local_date,
            duration_s: a.summary.duration.as_u32(),
            distance_km: a.summary.distance.as_f64(),
        })
        .collect()
}

/// A race result, the highest-confidence performance signal there is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaceResult {
    pub date: Date,
    pub distance_km: f64,
    pub duration_s: u32,
}

impl RaceResult {
    #[must_use]
    pub fn pace(&self) -> Option<Pace> {
        if self.distance_km < 0.5 || self.duration_s == 0 {
            return None;
        }
        Some(Pace::new(f64::from(self.duration_s) / self.distance_km))
    }
}

/// Predicted race pace from a threshold pace, using a distance-dependent factor.
///
/// The factors encode the well-observed fact that athletes hold a *higher*
/// fraction of threshold over a marathon than over a 5 k — the shorter the
/// effort, the more it draws on capacity above threshold.
#[must_use]
pub fn predicted_race_pace(threshold: Pace, distance_km: f64) -> Pace {
    let factor = if distance_km >= 42.0 {
        1.08
    } else if distance_km >= 21.0 {
        1.04
    } else if distance_km >= 10.0 {
        0.98
    } else if distance_km >= 5.0 {
        0.94
    } else {
        0.90
    };
    threshold.scaled(factor)
}

/// Predicted finish time for a race distance from a threshold pace.
#[must_use]
pub fn predicted_finish(
    threshold: Pace,
    distance_km: f64,
) -> Option<runalytics_core::DurationSecs> {
    if distance_km < 0.5 {
        return None;
    }
    let pace = predicted_race_pace(threshold, distance_km);
    Some(runalytics_core::DurationSecs::new(
        (pace.as_secs_per_km() * distance_km).round() as u32,
    ))
}

/// Performance signals for one date.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PerformanceSignals {
    pub threshold_pace: Option<Pace>,
    pub vo2max_estimate: Option<f64>,
    /// `0..=100`, against the athlete's own history.
    pub index: f64,
    /// Percentage change in the index over the trailing window.
    pub trend_pct: f64,
}

/// Estimate VO2max from a threshold pace.
///
/// The ACSM running equation: gross oxygen cost in ml/kg/min is
/// `3.5 + 0.2 · v` with **v in metres per minute**. The unit matters — feeding
/// it km/h instead produces a number around 6 for a world-class runner, which
/// then silently disappears into the clamp below and reports "20" for anyone
/// faster than a walking pace.
///
/// Reasonable to ±2, which is why it is used for trend and never as an absolute
/// prescription.
#[must_use]
pub fn vo2max_from_threshold(threshold: Pace) -> f64 {
    let velocity_m_per_min = 60_000.0 / threshold.as_secs_per_km();
    (3.5 + 0.2 * velocity_m_per_min).clamp(20.0, 90.0)
}

/// Threshold pace from a VO2max estimate, the inverse of the above.
#[must_use]
pub fn threshold_from_vo2max(vo2max: f64) -> Pace {
    let velocity_m_per_min = (vo2max - 3.5) / 0.2;
    if velocity_m_per_min <= 0.0 {
        return Pace::WALK;
    }
    Pace::new(60_000.0 / velocity_m_per_min)
}

/// A performance index on `0..=100` from a threshold pace, anchored to a
/// population reference band.
///
/// This is the one place a population anchor is unavoidable: without it there
/// is no way to put a raw pace on a 0-100 scale at all. The band is deliberately
/// wide and the docstring honest — the *level* is only loosely meaningful, while
/// the *change* in the index is exact, because the anchor cancels out.
#[must_use]
pub fn performance_index(threshold: Pace) -> f64 {
    // 3:00/km (elite-amateur) to 7:00/km (new runner) mapped to 100..=0.
    let fast = 180.0;
    let slow = 420.0;
    let clamped = threshold.as_secs_per_km().clamp(fast, slow);
    ((slow - clamped) / (slow - fast) * 100.0).clamp(0.0, 100.0)
}

/// Percentage change between two index values, guarding a zero baseline.
#[must_use]
pub fn percent_change(from: f64, to: f64) -> f64 {
    if from.abs() < 1e-6 {
        return 0.0;
    }
    (to - from) / from * 100.0
}

/// A dated performance observation, for building a trend.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PerformancePoint {
    pub date: Date,
    pub threshold: Pace,
    pub index: f64,
}

/// Build a performance series from dated threshold observations.
#[must_use]
pub fn performance_series(points: &[(Date, Pace)]) -> Vec<PerformancePoint> {
    let mut sorted: Vec<(Date, Pace)> = points.to_vec();
    sorted.sort_by_key(|(d, _)| *d);
    sorted
        .into_iter()
        .map(|(date, threshold)| PerformancePoint {
            date,
            threshold,
            index: performance_index(threshold),
        })
        .collect()
}

/// Trend over a trailing window: percent change from the window's start to now.
///
/// Compares means of the first and last thirds rather than single endpoints, so
/// one anomalous week cannot manufacture a trend.
#[must_use]
pub fn performance_trend(series: &[PerformancePoint]) -> f64 {
    if series.len() < 4 {
        return 0.0;
    }
    let third = (series.len() / 3).max(1);
    let early: f64 = series.iter().take(third).map(|p| p.index).sum::<f64>() / third as f64;
    let late: f64 = series
        .iter()
        .rev()
        .take(third)
        .map(|p| p.index)
        .sum::<f64>()
        / third as f64;
    percent_change(early, late)
}

/// The most recent race result at or before `date`, if any.
#[must_use]
pub fn latest_race(races: &[RaceResult], date: Date) -> Option<&RaceResult> {
    races
        .iter()
        .filter(|r| r.date <= date)
        .max_by_key(|r| r.date)
}

/// Weeks since the most recent race, used by the injury model.
#[must_use]
pub fn weeks_since_race(races: &[RaceResult], date: Date) -> Option<u32> {
    let race = latest_race(races, date)?;
    let days = date.signed_duration_since(race.date).num_days();
    // `latest_race` already guarantees `days >= 0`; saturating keeps that
    // guarantee total without a fallible conversion in the middle of the chain.
    Some(u32::try_from(days / 7).unwrap_or(u32::MAX))
}

/// Whether a race result should override the modelled threshold.
///
/// Only recent races count. A personal best from three years ago describes the
/// athlete the person used to be, and letting it set today's training zones is
/// how people get injured.
pub const RACE_VALIDITY_WEEKS: i64 = 12;

#[must_use]
pub fn race_is_current(race_date: Date, today: Date) -> bool {
    let days = today.signed_duration_since(race_date).num_days();
    (0..RACE_VALIDITY_WEEKS * 7).contains(&days)
}

/// Resolve the threshold pace to train against, preferring the most
/// trustworthy available source.
#[must_use]
pub fn resolve_threshold(
    race: Option<&RaceResult>,
    today: Date,
    modelled: Option<Pace>,
    provider_vo2max: Option<f64>,
) -> Option<Pace> {
    // Carry the distance alongside the pace so the distance factor can be
    // applied without a nested `if let`.
    let from_race = race
        .filter(|r| race_is_current(r.date, today))
        .and_then(|r| r.pace().map(|pace| (pace, r.distance_km)));
    if let Some((pace, distance_km)) = from_race {
        // Race pace is *race* pace; back it off to the threshold it implies
        // using the same distance factor used for the inverse prediction.
        return Some(threshold_from_race_pace(pace, distance_km));
    }
    modelled.or_else(|| provider_vo2max.map(threshold_from_vo2max))
}

/// The threshold pace implied by a race pace at a given distance.
#[must_use]
pub fn threshold_from_race_pace(race_pace: Pace, distance_km: f64) -> Pace {
    let factor = if distance_km >= 42.0 {
        1.08
    } else if distance_km >= 21.0 {
        1.04
    } else if distance_km >= 10.0 {
        0.98
    } else if distance_km >= 5.0 {
        0.94
    } else {
        0.90
    };
    // Invert: race pace = threshold × factor, so threshold = race pace / factor.
    race_pace.scaled(1.0 / factor)
}

/// Distance used when a caller only has a `VolumeKm`.
#[must_use]
pub fn finish_for_volume(
    threshold: Pace,
    distance: VolumeKm,
) -> Option<runalytics_core::DurationSecs> {
    predicted_finish(threshold, distance.as_f64())
}

#[cfg(test)]
mod tests {
    use super::*;
    use runalytics_core::{ActivitySummary, DurationSecs, HeartRate, Intensity, Timestamp};

    fn date(d: u32) -> Date {
        Date::from_ymd_opt(2026, 10, 1)
            .expect("base")
            .checked_add_days(chrono::Days::new(u64::from(d)))
            .expect("in range")
    }

    fn activity(minutes: u32, km: f64, on: Date) -> Activity {
        Activity {
            id: runalytics_core::ActivityId::new(),
            account: runalytics_core::ProviderAccountId::new(),
            provider_activity_id: "a".into(),
            name: "run".into(),
            started_at: Timestamp::default(),
            local_date: on,
            summary: ActivitySummary {
                distance: VolumeKm(km),
                duration: DurationSecs::from_minutes(minutes),
                avg_pace: None,
                avg_hr: Some(HeartRate::new(150)),
                max_hr: None,
                elevation_gain: None,
                avg_cadence: None,
                training_load: None,
            },
            laps: vec![],
            intensity: Intensity::Aerobic,
            matched_session: None,
            fetched_at: Timestamp::default(),
        }
    }

    #[test]
    fn effort_pace_is_distance_over_time() {
        let e = Effort {
            date: date(0),
            duration_s: 600,
            distance_km: 2.0,
        };
        assert!((e.pace().expect("pace").as_secs_per_km() - 300.0).abs() < 1e-6);
        assert!(
            Effort {
                date: date(0),
                duration_s: 600,
                distance_km: 0.0,
            }
            .pace()
            .is_none()
        );
    }

    #[test]
    fn critical_speed_fit_recovers_a_known_line() {
        // d = 0.005 km/s * (t - 120) -> CS = 18 km/h, W' = 120 s.
        let efforts: Vec<Effort> = [300u32, 480, 600, 900, 1200]
            .iter()
            .map(|&t| Effort {
                date: date(0),
                duration_s: t,
                distance_km: 0.005 * (f64::from(t) - 120.0),
            })
            .collect();
        let pace = best_effort_pace(&efforts, 600).expect("fit");
        // At t=600 the model predicts 0.005*480 = 2.4 km -> 250 s/km.
        assert!(
            (pace.as_secs_per_km() - 250.0).abs() < 2.0,
            "got {}",
            pace.as_secs_per_km()
        );
    }

    #[test]
    fn a_single_effort_falls_back_to_raw_pace() {
        let efforts = vec![Effort {
            date: date(0),
            duration_s: 600,
            distance_km: 2.0,
        }];
        let pace = best_effort_pace(&efforts, 600).expect("fallback");
        assert!((pace.as_secs_per_km() - 300.0).abs() < 1e-6);
    }

    #[test]
    fn efforts_outside_the_window_are_ignored() {
        let efforts = vec![
            Effort {
                date: date(0),
                duration_s: 60, // too short, anaerobic
                distance_km: 0.4,
            },
            Effort {
                date: date(1),
                duration_s: 5400, // too long
                distance_km: 20.0,
            },
        ];
        assert!(best_effort_pace(&efforts, 600).is_none());
    }

    #[test]
    fn identical_durations_do_not_crash_the_fit() {
        let efforts = vec![
            Effort {
                date: date(0),
                duration_s: 600,
                distance_km: 2.0,
            },
            Effort {
                date: date(1),
                duration_s: 600,
                distance_km: 2.2,
            },
        ];
        let pace = best_effort_pace(&efforts, 600).expect("fallback");
        assert!((pace.as_secs_per_km() - 272.7).abs() < 1.0);
    }

    #[test]
    fn efforts_are_extracted_and_short_ones_dropped() {
        let acts = vec![
            activity(10, 2.0, date(0)),
            activity(1, 0.2, date(1)), // under MIN_EFFORT_SECONDS
        ];
        let efforts = efforts_from(&acts);
        assert_eq!(efforts.len(), 1);
    }

    #[test]
    fn race_pace_predicts_a_slower_marathon_than_5k_per_km() {
        let threshold = Pace::new(270.0);
        let five = predicted_race_pace(threshold, 5.0);
        let marathon = predicted_race_pace(threshold, 42.2);
        assert!(marathon.as_secs_per_km() > five.as_secs_per_km());
    }

    #[test]
    fn race_pace_and_threshold_round_trip() {
        let threshold = Pace::new(270.0);
        for km in [5.0, 10.0, 21.1, 42.2] {
            let race = predicted_race_pace(threshold, km);
            let back = threshold_from_race_pace(race, km);
            assert!(
                (back.as_secs_per_km() - 270.0).abs() < 0.01,
                "{km} km round trip gave {}",
                back.as_secs_per_km()
            );
        }
    }

    #[test]
    fn predicted_finish_scales_with_distance() {
        let t = Pace::new(270.0);
        let ten = predicted_finish(t, 10.0).expect("ten");
        let half = predicted_finish(t, 21.1).expect("half");
        assert!(half.as_u32() > ten.as_u32());
        assert!(predicted_finish(t, 0.1).is_none());
    }

    #[test]
    fn vo2max_and_threshold_round_trip() {
        let threshold = Pace::new(240.0);
        let vo2 = vo2max_from_threshold(threshold);
        let back = threshold_from_vo2max(vo2);
        assert!((back.as_secs_per_km() - 240.0).abs() < 0.5);
    }

    /// A round trip is satisfied by *any* unit, including the wrong one, so the
    /// magnitude has to be pinned separately.
    ///
    /// 4:00/km is 250 m/min, and the ACSM equation gives a gross cost of
    /// `3.5 + 0.2 · 250 = 53.5` ml/kg/min. That is the right neighbourhood for a
    /// recreational runner's *threshold* — which sits around 85-90 % of VO2max,
    /// implying a true VO2max near 60 — and it is the number a coach would
    /// recognise. Feeding the equation km/h instead yields 6.5, which the clamp
    /// then reports as 20 for every athlete faster than a walk.
    #[test]
    fn vo2max_magnitudes_are_physiologically_plausible() {
        assert!((vo2max_from_threshold(Pace::new(240.0)) - 53.5).abs() < 0.1);
        // 5:00/km -> 200 m/min -> 43.5
        assert!((vo2max_from_threshold(Pace::new(300.0)) - 43.5).abs() < 0.1);
        // 3:30/km -> 285.7 m/min -> 60.6, elite territory but not absurd
        let elite = vo2max_from_threshold(Pace::new(210.0));
        assert!((59.0..62.0).contains(&elite), "got {elite}");
        // Nothing an athlete can actually run should ever touch the clamp.
        assert!(vo2max_from_threshold(Pace::new(150.0)) < 90.0);
        assert!(vo2max_from_threshold(Pace::new(600.0)) > 20.0);
    }

    #[test]
    fn faster_threshold_is_a_higher_index() {
        assert!(performance_index(Pace::new(200.0)) > performance_index(Pace::new(400.0)));
        assert_eq!(performance_index(Pace::new(100.0)), 100.0);
        assert_eq!(performance_index(Pace::new(900.0)), 0.0);
    }

    #[test]
    fn vo2max_estimate_stays_physiological() {
        assert!(vo2max_from_threshold(Pace::new(120.0)) <= 90.0);
        assert!(vo2max_from_threshold(Pace::new(700.0)) >= 20.0);
    }

    #[test]
    fn percent_change_guards_zero() {
        assert_eq!(percent_change(0.0, 50.0), 0.0);
        assert!((percent_change(50.0, 60.0) - 20.0).abs() < 1e-9);
        assert!(percent_change(50.0, 40.0) < 0.0);
    }

    #[test]
    fn trend_is_positive_when_paces_improve() {
        let points: Vec<(Date, Pace)> = (0..9)
            .map(|i| (date(i), Pace::new(330.0 - f64::from(i) * 5.0)))
            .collect();
        let series = performance_series(&points);
        assert!(performance_trend(&series) > 0.0);
    }

    #[test]
    fn trend_is_flat_for_a_stable_athlete() {
        let points: Vec<(Date, Pace)> = (0..9).map(|i| (date(i), Pace::new(300.0))).collect();
        assert!(performance_trend(&performance_series(&points)).abs() < 1e-9);
    }

    #[test]
    fn too_few_points_give_no_trend() {
        let points = vec![(date(0), Pace::new(300.0)), (date(7), Pace::new(250.0))];
        assert_eq!(performance_trend(&performance_series(&points)), 0.0);
    }

    #[test]
    fn series_is_sorted_by_date() {
        let points = vec![
            (date(14), Pace::new(290.0)),
            (date(0), Pace::new(300.0)),
            (date(7), Pace::new(295.0)),
        ];
        let series = performance_series(&points);
        assert_eq!(series[0].date, date(0));
        assert_eq!(series[2].date, date(14));
    }

    #[test]
    fn latest_race_ignores_the_future() {
        let races = vec![
            RaceResult {
                date: date(0),
                distance_km: 10.0,
                duration_s: 2400,
            },
            RaceResult {
                date: date(30),
                distance_km: 5.0,
                duration_s: 1200,
            },
        ];
        let found = latest_race(&races, date(10)).expect("one");
        assert_eq!(found.distance_km, 10.0);
        assert!(latest_race(&[], date(10)).is_none());
    }

    #[test]
    fn weeks_since_race_counts_correctly() {
        let races = vec![RaceResult {
            date: date(0),
            distance_km: 10.0,
            duration_s: 2400,
        }];
        assert_eq!(weeks_since_race(&races, date(21)), Some(3));
        assert_eq!(weeks_since_race(&races, date(3)), Some(0));
        assert_eq!(weeks_since_race(&[], date(3)), None);
    }

    #[test]
    fn an_old_race_stops_setting_zones() {
        assert!(race_is_current(date(0), date(14)));
        assert!(!race_is_current(date(0), date(200)));
        // A race "in the future" relative to today is not evidence of anything.
        assert!(!race_is_current(date(10), date(0)));
    }

    #[test]
    fn a_recent_race_beats_a_modelled_threshold() {
        let race = RaceResult {
            date: date(0),
            distance_km: 10.0,
            duration_s: 2000, // 200 s/km race pace
        };
        let resolved = resolve_threshold(Some(&race), date(7), Some(Pace::new(400.0)), Some(60.0))
            .expect("threshold");
        // Should come from the race (200/0.98 ≈ 204), not the 400 model.
        assert!(resolved.as_secs_per_km() < 250.0, "got {resolved}");
    }

    #[test]
    fn a_stale_race_does_not_set_zones() {
        let race = RaceResult {
            date: date(0),
            distance_km: 10.0,
            duration_s: 2000,
        };
        let resolved = resolve_threshold(Some(&race), date(300), Some(Pace::new(400.0)), None)
            .expect("threshold");
        assert!((resolved.as_secs_per_km() - 400.0).abs() < 1e-6);
    }

    #[test]
    fn provider_vo2max_is_the_last_resort() {
        let resolved = resolve_threshold(None, date(0), None, Some(50.0)).expect("threshold");
        assert!((resolved.as_secs_per_km() - threshold_from_vo2max(50.0).as_secs_per_km()) < 1e-6);
        assert!(resolve_threshold(None, date(0), None, None).is_none());
    }

    #[test]
    fn race_pace_needs_real_input() {
        assert!(
            RaceResult {
                date: date(0),
                distance_km: 0.1,
                duration_s: 60,
            }
            .pace()
            .is_none()
        );
        assert!(
            RaceResult {
                date: date(0),
                distance_km: 10.0,
                duration_s: 0,
            }
            .pace()
            .is_none()
        );
    }
}
