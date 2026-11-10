use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Domain {
    SubThreshold,
    Threshold,
    HighThreshold,
    AboveThreshold,
    Uncertain,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Repetition {
    pub duration_s: f64,
    pub recovery_s: f64,
    pub pace_s_km: Option<f64>,
    pub hr: Option<f64>,
    pub power_w: Option<f64>,
    pub rpe: Option<f64>,
    pub lactate_mmol: Option<f64>,
    pub lactate_delay_s: Option<f64>,
    pub talk_test_comfortable: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Estimate {
    pub value: f64,
    pub confidence: f64,
    pub relative_uncertainty: f64,
    pub source: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ThresholdPoint {
    pub pace_s_km: Option<Estimate>,
    pub hr: Option<Estimate>,
    pub power_w: Option<Estimate>,
    pub lactate_mmol: Option<Estimate>,
    pub lactate_delay_s: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionAnalysis {
    pub intended_session_type: Domain,
    pub observed_session_type: Domain,
    pub threshold_intensity_estimate: Option<f64>,
    pub threshold_control: Option<f64>,
    pub threshold_load: f64,
    pub work_minutes: f64,
    pub confidence: f64,
    pub overshot: bool,
    pub lactate_mean_mmol: Option<f64>,
    pub lactate_delta_mmol: Option<f64>,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzedSession {
    pub date: NaiveDate,
    pub analysis: SessionAnalysis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FitnessTrend {
    Improving,
    Stable,
    Declining,
    Uncertain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Tolerance {
    Low,
    Moderate,
    High,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RatedTolerance {
    pub level: Tolerance,
    pub confidence: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Capacity {
    pub training_age_years: Option<f64>,
    pub weeks_consistent: Option<f64>,
    pub weekly_km: Option<f64>,
    pub historical_weekly_km: Option<f64>,
    pub weekly_sessions: Option<f64>,
    pub threshold_months: Option<f64>,
    pub easy_time_fraction: Option<f64>,
    pub sleep_hours: Option<f64>,
    pub fatigue_high: Option<bool>,
    pub injury_active: Option<bool>,
    pub injury_last_year: Option<bool>,
    pub recovery_good: Option<bool>,
    pub recovery_time_available: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Conditions {
    pub temperature_c: Option<f64>,
    pub elevation_gain_m_per_km: Option<f64>,
    pub altitude_m: Option<f64>,
    pub fatigued: Option<bool>,
    pub tapered: Option<bool>,
    pub sensor_suspect: bool,
}

impl Conditions {
    fn comparable(&self, other: &Self) -> bool {
        let close = |first: Option<f64>, second: Option<f64>, limit: f64| matches!((first, second), (Some(first), Some(second)) if first.is_finite() && second.is_finite() && (first - second).abs() <= limit);
        !self.sensor_suspect
            && !other.sensor_suspect
            && self.fatigued == Some(false)
            && other.fatigued == Some(false)
            && self.tapered == Some(false)
            && other.tapered == Some(false)
            && close(self.temperature_c, other.temperature_c, 3.0)
            && close(
                self.elevation_gain_m_per_km,
                other.elevation_gain_m_per_km,
                3.0,
            )
            && close(self.altitude_m, other.altitude_m, 200.0)
    }

    fn confounded(&self) -> bool {
        self.sensor_suspect
            || self.fatigued == Some(true)
            || self.tapered == Some(true)
            || self
                .temperature_c
                .is_some_and(|value| !value.is_finite() || value > 25.0)
            || self
                .elevation_gain_m_per_km
                .is_some_and(|value| !value.is_finite() || value > 10.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEvidence {
    pub date: NaiveDate,
    pub intended: Domain,
    pub repetitions: Vec<Repetition>,
    #[serde(default)]
    pub conditions: Conditions,
    pub planned_work_s: Option<f64>,
    pub recovery_hours: Option<f64>,
    pub following_performance_declined: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MeasurementSource {
    LactateTest,
    Watch,
    FieldTest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThresholdObservation {
    pub date: NaiveDate,
    pub source: MeasurementSource,
    pub lt1: ThresholdPoint,
    pub lt2: ThresholdPoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Performance {
    pub date: NaiveDate,
    pub duration_s: f64,
    pub distance_m: f64,
    pub power_w: Option<f64>,
    pub maximal: bool,
    #[serde(default)]
    pub conditions: Conditions,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Evidence {
    pub observations: Vec<ThresholdObservation>,
    pub sessions: Vec<SessionEvidence>,
    pub performances: Vec<Performance>,
    pub capacity: Capacity,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Efficiency {
    pub threshold_pace_at_standardized_hr: Option<Estimate>,
    pub threshold_power_at_standardized_hr: Option<Estimate>,
    pub pace_at_standardized_lactate: Option<Estimate>,
    pub power_at_standardized_lactate: Option<Estimate>,
    pub hr_at_standardized_pace: Option<Estimate>,
    pub lactate_at_standardized_pace: Option<Estimate>,
    pub threshold_duration_tolerance: Option<Estimate>,
    pub threshold_session_stability: Option<Estimate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AthleteState {
    pub version: u32,
    pub as_of: NaiveDate,
    pub lt1_estimate: ThresholdPoint,
    pub lt2_estimate: ThresholdPoint,
    pub critical_speed_m_s: Option<Estimate>,
    pub critical_power_w: Option<Estimate>,
    pub threshold_confidence: f64,
    pub lactate_profile_confidence: f64,
    pub threshold_control: Option<f64>,
    pub threshold_volume_7d: f64,
    pub threshold_volume_28d: f64,
    pub threshold_frequency: usize,
    pub threshold_load: f64,
    pub threshold_load_trend: Option<f64>,
    pub threshold_tolerance: RatedTolerance,
    pub threshold_recovery: Option<f64>,
    pub continuous_threshold_duration_lower_bound_min: Option<Estimate>,
    pub double_threshold_tolerance: RatedTolerance,
    pub efficiency: Efficiency,
    pub fitness_trend: FitnessTrend,
    pub repeated_overshoots: usize,
    pub quality_minutes_cap: f64,
    pub allow_second_quality: bool,
    pub hold_quality: bool,
    pub sessions: Vec<AnalyzedSession>,
    pub reasons: Vec<String>,
}

fn estimate(value: f64, confidence: f64, source: &str) -> Estimate {
    Estimate {
        value,
        confidence,
        relative_uncertainty: (1.0 - confidence) * 0.20,
        source: source.into(),
    }
}

fn age(as_of: NaiveDate, date: NaiveDate) -> Option<i64> {
    let days = (as_of - date).num_days();
    (days >= 0).then_some(days)
}

fn decay(point: &ThresholdPoint, days: i64, source: MeasurementSource) -> ThresholdPoint {
    let cap: f64 = match source {
        MeasurementSource::LactateTest => 0.9,
        MeasurementSource::Watch => 0.55,
        MeasurementSource::FieldTest => 0.65,
    };
    let adjust = |anchor: &Option<Estimate>| {
        anchor
            .as_ref()
            .filter(|anchor| usable(anchor))
            .map(|anchor| {
                let mut anchor = anchor.clone();
                anchor.confidence =
                    anchor.confidence.min(cap) * 2.0_f64.powf(-(days as f64) / 56.0);
                anchor.relative_uncertainty =
                    (anchor.relative_uncertainty + days as f64 / 1000.0).min(0.25);
                anchor
            })
    };
    ThresholdPoint {
        pace_s_km: adjust(&point.pace_s_km),
        hr: adjust(&point.hr),
        power_w: adjust(&point.power_w),
        lactate_mmol: adjust(&point.lactate_mmol),
        lactate_delay_s: point.lactate_delay_s,
    }
}

fn capacity_rating(
    capacity: &Capacity,
    control: Option<f64>,
    count: usize,
    doubles: bool,
) -> RatedTolerance {
    if capacity.injury_active == Some(true)
        || capacity.fatigue_high == Some(true)
        || capacity.recovery_good == Some(false)
        || capacity.sleep_hours.is_some_and(|hours| hours < 7.0)
        || control.is_some_and(|control| control < 65.0)
    {
        return RatedTolerance {
            level: Tolerance::Low,
            confidence: 0.8,
        };
    }
    let values = [
        capacity.training_age_years,
        capacity.weeks_consistent,
        capacity.weekly_km,
        capacity.historical_weekly_km,
        capacity.weekly_sessions,
        capacity.threshold_months,
        capacity.easy_time_fraction,
        capacity.sleep_hours,
    ];
    if values
        .iter()
        .any(|value| !value.is_some_and(|value| value.is_finite() && value >= 0.0))
        || capacity.fatigue_high.is_none()
        || capacity.injury_active.is_none()
        || capacity.recovery_good.is_none()
        || (doubles
            && (capacity.injury_last_year.is_none() || capacity.recovery_time_available.is_none()))
    {
        return RatedTolerance {
            level: Tolerance::Unknown,
            confidence: 0.0,
        };
    }
    let adequate = capacity.training_age_years.unwrap() >= if doubles { 3.0 } else { 1.0 }
        && capacity.weeks_consistent.unwrap() >= if doubles { 24.0 } else { 8.0 }
        && capacity.weekly_km.unwrap() >= if doubles { 80.0 } else { 35.0 }
        && capacity.historical_weekly_km.unwrap() >= if doubles { 80.0 } else { 30.0 }
        && capacity.weekly_km.unwrap() <= capacity.historical_weekly_km.unwrap() * 1.1
        && capacity.weekly_sessions.unwrap() >= if doubles { 8.0 } else { 4.0 }
        && capacity.threshold_months.unwrap() >= if doubles { 12.0 } else { 3.0 }
        && (0.8..=1.0).contains(&capacity.easy_time_fraction.unwrap())
        && capacity.recovery_good == Some(true)
        && control.is_some_and(|control| control >= 85.0)
        && count >= if doubles { 6 } else { 3 }
        && (!doubles
            || (capacity.injury_last_year == Some(false)
                && capacity.recovery_time_available == Some(true)));
    RatedTolerance {
        level: if adequate {
            Tolerance::High
        } else {
            Tolerance::Moderate
        },
        confidence: if adequate { 0.7 } else { 0.6 },
    }
}

fn efficiency(
    session: &SessionEvidence,
    analysis: &SessionAnalysis,
    anchor: &ThresholdPoint,
) -> Efficiency {
    let mut output = Efficiency::default();
    if session.conditions.confounded()
        || analysis.confidence < 0.4
        || analysis.observed_session_type != Domain::Threshold
        || analysis.overshot
    {
        return output;
    }
    let reps = &session.repetitions;
    let near = |value: Option<f64>, reference: &Option<Estimate>, fraction: f64| matches!((positive(value), reference.as_ref()), (Some(value), Some(reference)) if usable(reference) && (value / reference.value - 1.0).abs() <= fraction);
    let timed = |rep: &Repetition| matches!((rep.lactate_delay_s, anchor.lactate_delay_s), (Some(first), Some(second)) if first.is_finite() && second.is_finite() && first >= 0.0 && second >= 0.0 && (first - second).abs() <= 15.0);
    let metric = |value: Option<f64>| {
        value.map(|value| estimate(value, analysis.confidence, "matched threshold session"))
    };
    output.threshold_pace_at_standardized_hr = metric(mean(
        reps.iter()
            .filter(|rep| rep.duration_s >= 180.0 && near(rep.hr, &anchor.hr, 0.02))
            .filter_map(|rep| positive(rep.pace_s_km)),
    ));
    output.threshold_power_at_standardized_hr = metric(mean(
        reps.iter()
            .filter(|rep| rep.duration_s >= 180.0 && near(rep.hr, &anchor.hr, 0.02))
            .filter_map(|rep| positive(rep.power_w)),
    ));
    output.pace_at_standardized_lactate = metric(mean(
        reps.iter()
            .filter(|rep| timed(rep) && near(rep.lactate_mmol, &anchor.lactate_mmol, 0.1))
            .filter_map(|rep| positive(rep.pace_s_km)),
    ));
    output.power_at_standardized_lactate = metric(mean(
        reps.iter()
            .filter(|rep| timed(rep) && near(rep.lactate_mmol, &anchor.lactate_mmol, 0.1))
            .filter_map(|rep| positive(rep.power_w)),
    ));
    output.hr_at_standardized_pace = metric(mean(
        reps.iter()
            .filter(|rep| rep.duration_s >= 180.0 && near(rep.pace_s_km, &anchor.pace_s_km, 0.02))
            .filter_map(|rep| positive(rep.hr)),
    ));
    output.lactate_at_standardized_pace = metric(mean(
        reps.iter()
            .filter(|rep| timed(rep) && near(rep.pace_s_km, &anchor.pace_s_km, 0.02))
            .filter_map(|rep| positive(rep.lactate_mmol)),
    ));
    output.threshold_duration_tolerance = metric(Some(analysis.work_minutes));
    output.threshold_session_stability = metric(analysis.threshold_control);
    output
}

fn trend_pair(first: &Efficiency, last: &Efficiency) -> Option<f64> {
    let pairs = [
        (
            &first.threshold_pace_at_standardized_hr,
            &last.threshold_pace_at_standardized_hr,
            -1.0,
        ),
        (
            &first.threshold_power_at_standardized_hr,
            &last.threshold_power_at_standardized_hr,
            1.0,
        ),
        (
            &first.pace_at_standardized_lactate,
            &last.pace_at_standardized_lactate,
            -1.0,
        ),
        (
            &first.power_at_standardized_lactate,
            &last.power_at_standardized_lactate,
            1.0,
        ),
        (
            &first.hr_at_standardized_pace,
            &last.hr_at_standardized_pace,
            -1.0,
        ),
        (
            &first.lactate_at_standardized_pace,
            &last.lactate_at_standardized_pace,
            -1.0,
        ),
    ];
    mean(pairs.into_iter().filter_map(|(first, last, direction)| {
        let (first, last) = (first.as_ref()?, last.as_ref()?);
        Some((last.value / first.value - 1.0) * direction)
    }))
}

pub fn athlete_state(evidence: &Evidence, as_of: NaiveDate, fallback_pace: f64) -> AthleteState {
    let mut observations: Vec<_> = evidence
        .observations
        .iter()
        .filter_map(|observation| {
            let days = age(as_of, observation.date)?;
            (days <= 180).then_some((observation, days))
        })
        .collect();
    observations.sort_by_key(|(_, days)| *days);
    let mut lt1 = ThresholdPoint::default();
    let mut lt2 = ThresholdPoint::default();
    for (observation, days) in observations.iter().rev() {
        let merge = |target: &mut ThresholdPoint, point: &ThresholdPoint| {
            let point = decay(point, *days, observation.source);
            if point.lactate_mmol.as_ref().is_some_and(|incoming| {
                target
                    .lactate_mmol
                    .as_ref()
                    .is_none_or(|current| incoming.confidence >= current.confidence)
            }) {
                target.lactate_delay_s = point.lactate_delay_s;
            }
            for (target, incoming) in [
                (&mut target.pace_s_km, point.pace_s_km),
                (&mut target.hr, point.hr),
                (&mut target.power_w, point.power_w),
                (&mut target.lactate_mmol, point.lactate_mmol),
            ] {
                if let Some(incoming) = incoming {
                    if target
                        .as_ref()
                        .is_none_or(|current| incoming.confidence >= current.confidence)
                    {
                        *target = Some(incoming);
                    }
                }
            }
        };
        merge(&mut lt1, &observation.lt1);
        merge(&mut lt2, &observation.lt2);
    }
    let performances: Vec<_> = evidence
        .performances
        .iter()
        .filter(|performance| {
            age(as_of, performance.date).is_some_and(|days| days <= 56)
                && performance.maximal
                && !performance.conditions.confounded()
                && performance.duration_s.is_finite()
                && performance.distance_m.is_finite()
                && performance.duration_s >= 180.0
                && performance.distance_m > 0.0
        })
        .collect();
    if lt2.pace_s_km.is_none() {
        if let Some(race) = performances
            .iter()
            .filter(|race| (5000.0..=21100.0).contains(&race.distance_m))
            .max_by_key(|race| race.date)
        {
            let distance_hour = race.distance_m * (3600.0 / race.duration_s).powf(1.0 / 1.06);
            lt2.pace_s_km = Some(estimate(
                3_600_000.0 / distance_hour,
                0.4,
                "race one-hour proxy; not measured LT2",
            ));
        } else if positive(Some(fallback_pace)).is_some() {
            lt2.pace_s_km = Some(estimate(
                fallback_pace,
                0.3,
                "requested pace; unverified proxy",
            ));
        }
    }
    let mut critical_speed = None;
    let mut critical_power = None;
    for (index, first) in performances.iter().enumerate() {
        for second in performances.iter().skip(index + 1) {
            let (short, long) = if first.duration_s < second.duration_s {
                (*first, *second)
            } else {
                (*second, *first)
            };
            if long.duration_s > 1200.0
                || long.duration_s / short.duration_s < 2.0
                || (long.date - short.date).num_days().abs() > 21
                || !short.conditions.comparable(&long.conditions)
            {
                continue;
            }
            let speed = (long.distance_m - short.distance_m) / (long.duration_s - short.duration_s);
            let reserve = short.distance_m - speed * short.duration_s;
            if speed > 0.0 && reserve > 0.0 && speed < long.distance_m / long.duration_s {
                critical_speed = Some(estimate(
                    speed,
                    0.5,
                    "two-performance critical-speed fit; distinct from LT2",
                ));
            }
            if let (Some(short_power), Some(long_power)) =
                (positive(short.power_w), positive(long.power_w))
            {
                let power = (long_power * long.duration_s - short_power * short.duration_s)
                    / (long.duration_s - short.duration_s);
                if power > 0.0 && power < long_power && short_power > long_power {
                    critical_power = Some(estimate(
                        power,
                        0.5,
                        "two-performance critical-power fit; distinct from LT2",
                    ));
                }
            }
        }
    }
    let mut sessions: Vec<_> = evidence
        .sessions
        .iter()
        .filter(|session| age(as_of, session.date).is_some_and(|days| days < 56))
        .collect();
    sessions.sort_by_key(|session| session.date);
    sessions.dedup_by(|first, second| {
        first.date == second.date && first.repetitions == second.repetitions
    });
    if let Some(pace) = lt2.pace_s_km.as_ref().filter(|anchor| usable(anchor)) {
        let steady: Vec<_> = sessions
            .iter()
            .filter(|session| {
                !session.conditions.confounded()
                    && session.intended == Domain::Threshold
                    && age(as_of, session.date).is_some_and(|days| days < 28)
                    && session
                        .repetitions
                        .iter()
                        .map(|rep| rep.duration_s)
                        .sum::<f64>()
                        >= 900.0
                    && session.repetitions.iter().all(|rep| {
                        rep.duration_s >= 180.0
                            && positive(rep.pace_s_km)
                                .is_some_and(|value| (0.98..=1.04).contains(&(value / pace.value)))
                            && rep.rpe.is_some_and(|rpe| (5.0..=7.0).contains(&rpe))
                    })
            })
            .collect();
        let days: std::collections::BTreeSet<_> =
            steady.iter().map(|session| session.date).collect();
        if days.len() >= 3 {
            if lt2.hr.is_none() {
                lt2.hr = mean(
                    steady
                        .iter()
                        .flat_map(|session| &session.repetitions)
                        .filter_map(|rep| positive(rep.hr)),
                )
                .map(|value| estimate(value, 0.4, "repeatable pace/RPE-conditioned HR proxy"));
            }
            if lt2.power_w.is_none() {
                lt2.power_w = mean(
                    steady
                        .iter()
                        .flat_map(|session| &session.repetitions)
                        .filter_map(|rep| positive(rep.power_w)),
                )
                .map(|value| estimate(value, 0.4, "repeatable pace/RPE-conditioned power proxy"));
            }
        }
    }
    let analyzed: Vec<_> = sessions
        .iter()
        .map(|session| {
            let mut analysis = analyze_session(
                session.intended,
                if session.conditions.sensor_suspect {
                    &[]
                } else {
                    &session.repetitions
                },
                &lt2,
                session.conditions.confounded(),
            );
            if session.planned_work_s.is_some_and(|planned| {
                planned.is_finite() && planned > 0.0 && analysis.work_minutes * 60.0 < planned * 0.9
            }) || session.recovery_hours.is_some_and(|hours| hours > 48.0)
                || session.following_performance_declined == Some(true)
            {
                analysis.threshold_control = analysis
                    .threshold_control
                    .map(|score| (score - 25.0).max(0.0));
                analysis.overshot = analysis.threshold_control.is_some_and(|score| score < 65.0);
                analysis
                    .reasons
                    .push("incomplete work or delayed recovery".into());
            }
            analysis
        })
        .collect();
    let threshold_family = |domain: Domain| {
        matches!(
            domain,
            Domain::Threshold | Domain::HighThreshold | Domain::AboveThreshold
        )
    };
    let recent: Vec<_> = sessions
        .iter()
        .zip(&analyzed)
        .filter(|(session, analysis)| {
            age(as_of, session.date).unwrap() < 28
                && (threshold_family(session.intended)
                    || threshold_family(analysis.observed_session_type))
        })
        .collect();
    let overshoots = recent
        .iter()
        .filter(|(_, analysis)| analysis.overshot)
        .count();
    let control = mean(
        recent
            .iter()
            .filter_map(|(_, analysis)| analysis.threshold_control),
    )
    .map(|score| (score - overshoots.saturating_sub(1) as f64 * 10.0).max(0.0));
    let volume = |window: i64| {
        recent
            .iter()
            .filter(|(session, analysis)| {
                age(as_of, session.date).unwrap() < window
                    && threshold_family(analysis.observed_session_type)
            })
            .map(|(_, analysis)| analysis.work_minutes)
            .sum::<f64>()
    };
    let load = |from: i64, to: i64| {
        recent
            .iter()
            .filter(|(session, _)| {
                let days = age(as_of, session.date).unwrap();
                days >= from && days < to
            })
            .map(|(_, analysis)| analysis.threshold_load)
            .sum::<f64>()
    };
    let recovery = mean(
        recent
            .iter()
            .filter_map(|(session, _)| positive(session.recovery_hours)),
    );
    let days_observed = recent
        .iter()
        .filter(|(_, analysis)| analysis.confidence >= 0.4)
        .map(|(session, _)| session.date)
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let tolerance = capacity_rating(&evidence.capacity, control, days_observed, false);
    let double_tolerance = capacity_rating(&evidence.capacity, control, days_observed, true);
    let efficiency_history: Vec<_> = sessions
        .iter()
        .zip(&analyzed)
        .map(|(session, analysis)| efficiency(session, analysis, &lt2))
        .collect();
    let mut changes = Vec::new();
    let mut used_baselines = std::collections::BTreeSet::new();
    for (last_index, last) in sessions.iter().enumerate() {
        if age(as_of, last.date).unwrap() >= 14 {
            continue;
        }
        let matched = sessions.iter().enumerate().find(|(first_index, first)| {
            let gap = (last.date - first.date).num_days();
            *first_index < last_index
                && !used_baselines.contains(&first.date)
                && (14..=42).contains(&gap)
                && last.conditions.comparable(&first.conditions)
                && first.repetitions.len() == last.repetitions.len()
                && first
                    .repetitions
                    .iter()
                    .zip(&last.repetitions)
                    .all(|(first, last)| {
                        (first.duration_s - last.duration_s).abs() <= 30.0
                            && (first.recovery_s - last.recovery_s).abs() <= 15.0
                    })
        });
        if let Some((first_index, first)) = matched {
            if let Some(change) = trend_pair(
                &efficiency_history[first_index],
                &efficiency_history[last_index],
            ) {
                used_baselines.insert(first.date);
                changes.push((last.date, change));
            }
        }
    }
    changes.sort_by_key(|(date, _)| *date);
    changes.dedup_by_key(|(date, _)| *date);
    let trend = if changes.len() < 2 {
        FitnessTrend::Uncertain
    } else {
        let change = mean(changes.iter().map(|(_, change)| *change)).unwrap();
        if changes.iter().any(|(_, value)| *value > 0.02)
            && changes.iter().any(|(_, value)| *value < -0.02)
        {
            FitnessTrend::Uncertain
        } else if change > 0.02 {
            FitnessTrend::Improving
        } else if change < -0.02 {
            FitnessTrend::Declining
        } else {
            FitnessTrend::Stable
        }
    };
    let hold = evidence.capacity.injury_active == Some(true)
        || evidence.capacity.fatigue_high == Some(true)
        || evidence.capacity.recovery_good == Some(false)
        || evidence
            .capacity
            .sleep_hours
            .is_some_and(|hours| hours < 6.0);
    let controlled_max = recent
        .iter()
        .filter(|(_, analysis)| {
            analysis.confidence >= 0.4
                && analysis
                    .threshold_control
                    .is_some_and(|score| score >= 85.0)
                && analysis.observed_session_type == Domain::Threshold
        })
        .map(|(_, analysis)| analysis.work_minutes)
        .fold(0.0, f64::max);
    let density_spike = load(7, 28) > 0.0 && load(0, 7) > load(7, 28) / 3.0 * 1.2;
    let cap = if overshoots >= 2 || density_spike || recovery.is_some_and(|hours| hours > 48.0) {
        12.0
    } else if tolerance.level == Tolerance::High {
        (controlled_max * 1.05).clamp(12.0, 30.0)
    } else {
        15.0
    };
    let confidence = [&lt2.pace_s_km, &lt2.hr, &lt2.power_w]
        .into_iter()
        .filter_map(|anchor| anchor.as_ref().map(|anchor| anchor.confidence))
        .fold(0.0, f64::max);
    let lactate_confidence = lt2
        .lactate_mmol
        .as_ref()
        .map_or(0.0, |anchor| anchor.confidence);
    let mut reasons = vec![
        "Controlled work and an easy aerobic majority; doubles are never automatically scheduled"
            .into(),
    ];
    if lactate_confidence == 0.0 {
        reasons.push("Wearable proxy is not a blood-lactate measurement; LT1/HR/power stay unknown without evidence".into());
    }
    if overshoots >= 2 {
        reasons.push(
            "Repeated threshold overshooting: reduce work, do not reward faster repetitions".into(),
        );
    }
    if hold {
        reasons.push(
            "Recovery or injury signals require easy-only training pending reassessment".into(),
        );
    }
    AthleteState {
        version: 2,
        as_of,
        lt1_estimate: lt1,
        lt2_estimate: lt2,
        critical_speed_m_s: critical_speed,
        critical_power_w: critical_power,
        threshold_confidence: confidence,
        lactate_profile_confidence: lactate_confidence,
        threshold_control: control,
        threshold_volume_7d: volume(7),
        threshold_volume_28d: volume(28),
        threshold_frequency: recent
            .iter()
            .filter(|(session, analysis)| {
                age(as_of, session.date).unwrap() < 7
                    && threshold_family(analysis.observed_session_type)
            })
            .count(),
        threshold_load: load(0, 7),
        threshold_load_trend: (load(7, 28) > 0.0).then(|| load(0, 7) / (load(7, 28) / 3.0) - 1.0),
        threshold_tolerance: tolerance.clone(),
        threshold_recovery: recovery,
        continuous_threshold_duration_lower_bound_min: recent
            .iter()
            .filter(|(_, analysis)| {
                analysis.confidence >= 0.4
                    && analysis.observed_session_type == Domain::Threshold
                    && analysis
                        .threshold_control
                        .is_some_and(|control| control >= 85.0)
            })
            .flat_map(|(session, analysis)| {
                session.repetitions.iter().filter_map(move |rep| {
                    positive(Some(rep.duration_s))
                        .map(|seconds| (seconds / 60.0, analysis.confidence))
                })
            })
            .max_by(|first, second| first.0.total_cmp(&second.0))
            .map(|(minutes, confidence)| {
                estimate(
                    minutes,
                    confidence,
                    "longest controlled repetition; lower bound, not time to exhaustion",
                )
            }),
        double_threshold_tolerance: double_tolerance,
        efficiency: efficiency_history.last().cloned().unwrap_or_default(),
        fitness_trend: trend,
        repeated_overshoots: overshoots,
        quality_minutes_cap: if hold { 0.0 } else { cap },
        allow_second_quality: tolerance.level == Tolerance::High
            && overshoots == 0
            && !density_spike
            && !hold
            && recovery.is_some_and(|hours| hours <= 36.0),
        hold_quality: hold,
        sessions: sessions
            .iter()
            .zip(&analyzed)
            .map(|(session, analysis)| AnalyzedSession {
                date: session.date,
                analysis: analysis.clone(),
            })
            .collect(),
        reasons,
    }
}

pub fn wearable_evidence(
    store: &crate::store::Store,
    as_of: NaiveDate,
) -> crate::error::Result<Evidence> {
    let mut evidence = Evidence::default();
    if let Some(fitness) = store.fitness_on_or_before(as_of)? {
        if let Some(pace) = positive(fitness.threshold_sec_km) {
            evidence.observations.push(ThresholdObservation {
                date: fitness.date,
                source: MeasurementSource::Watch,
                lt1: ThresholdPoint::default(),
                lt2: ThresholdPoint {
                    pace_s_km: Some(estimate(pace, 0.55, "wearable threshold pace")),
                    ..Default::default()
                },
            });
        }
    }
    let midnight = |date: NaiveDate| date.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
    let runs = store.runs_between(
        midnight(as_of - chrono::Duration::days(56)),
        midnight(as_of + chrono::Duration::days(1)),
    )?;
    let current = store.current_plan()?;
    let completions = store.calendar_completions()?;
    let reference = evidence
        .observations
        .last()
        .and_then(|observation| observation.lt2.pace_s_km.as_ref())
        .map(|anchor| anchor.value)
        .or_else(|| current.as_ref().map(|(plan, _)| plan.threshold_sec_km));
    for run in &runs {
        let points = store.track_points(run.id, crate::laps::TRACK_POINTS)?;
        let matched = current.as_ref().and_then(|(plan, workouts)| {
            let workout_id = &completions
                .iter()
                .find(|(_, activity_id, _)| *activity_id == run.id)?
                .0;
            Some((
                plan,
                workouts.iter().find(|workout| workout.id == *workout_id)?,
            ))
        });
        let mut repetitions = Vec::new();
        let mut intended = Domain::Uncertain;
        let mut planned = None;
        if let Some((plan, workout)) = matched.filter(|(_, workout)| {
            matches!(
                workout.kind,
                crate::types::WorkoutKind::Tempo | crate::types::WorkoutKind::Intervals
            )
        }) {
            intended = if workout.kind == crate::types::WorkoutKind::Tempo {
                Domain::Threshold
            } else {
                Domain::AboveThreshold
            };
            let steps = super::session::details_for_plan(workout, plan);
            planned = Some(
                steps
                    .steps
                    .iter()
                    .filter(|step| !step.open_ended)
                    .map(|step| {
                        step.distance_m * step.repeats as f64 / 1000.0
                            * step.pace_hi_s_km.unwrap_or(300) as f64
                    })
                    .sum(),
            );
            let original_notes = completions.iter()
                .find(|(_, activity_id, _)| *activity_id == run.id)
                .map(|entry| entry.2.as_str())
                .unwrap_or_default();
            for lap in crate::laps::insight_laps(&points, run, plan, workout, original_notes) {
                repetitions.push(Repetition {
                    duration_s: lap.duration_s,
                    pace_s_km: lap.avg_pace_s_km,
                    hr: lap.avg_hr.map(|hr| hr as f64),
                    ..Default::default()
                });
            }
        } else if let Some(reference) = reference {
            let (_, splits) = crate::laps::build(&points, run, None, 0.0);
            for split in splits {
                if split.avg_pace_s_km.is_some_and(|pace| {
                    (120.0..=1200.0).contains(&pace) && reference / pace >= 0.94
                }) {
                    repetitions.push(Repetition {
                        duration_s: split.duration_s,
                        pace_s_km: split.avg_pace_s_km,
                        hr: split.avg_hr.map(|hr| hr as f64),
                        ..Default::default()
                    });
                }
            }
        }
        if !repetitions.is_empty() {
            evidence.sessions.push(SessionEvidence {
                date: run.start_time.date_naive(),
                intended,
                repetitions,
                conditions: Conditions {
                    elevation_gain_m_per_km: run
                        .elevation_gain_m
                        .filter(|_| run.distance_m > 0.0)
                        .map(|gain| gain / (run.distance_m / 1000.0)),
                    ..Default::default()
                },
                planned_work_s: planned,
                recovery_hours: None,
                following_performance_declined: None,
            });
        }
    }
    let recent_km = runs
        .iter()
        .filter(|run| age(as_of, run.start_time.date_naive()).is_some_and(|days| days < 28))
        .map(|run| run.distance_m / 1000.0)
        .sum::<f64>()
        / 4.0;
    let previous_km = runs
        .iter()
        .filter(|run| {
            age(as_of, run.start_time.date_naive()).is_some_and(|days| (28..56).contains(&days))
        })
        .map(|run| run.distance_m / 1000.0)
        .sum::<f64>()
        / 4.0;
    evidence.capacity.weekly_km = Some(recent_km);
    evidence.capacity.historical_weekly_km = Some(previous_km);
    evidence.capacity.weekly_sessions = Some(
        runs.iter()
            .filter(|run| age(as_of, run.start_time.date_naive()).is_some_and(|days| days < 28))
            .count() as f64
            / 4.0,
    );
    evidence.capacity.injury_active = store.latest_pain()?.map(|_| true);
    let health = store.health_between(as_of - chrono::Duration::days(7), as_of)?;
    if health.len() >= 3 {
        evidence.capacity.sleep_hours = mean(health.iter().filter_map(|day| {
            day.sleep_min
                .filter(|minutes| (60..=900).contains(minutes))
                .map(|minutes| minutes as f64 / 60.0)
        }));
    }
    let baseline_health = store.health_between(
        as_of - chrono::Duration::days(35),
        as_of - chrono::Duration::days(8),
    )?;
    let recent_hrv = mean(health.iter().filter_map(|day| positive(day.hrv_avg)));
    let baseline_hrv = mean(
        baseline_health
            .iter()
            .filter_map(|day| positive(day.hrv_avg)),
    );
    let recent_hr = mean(
        health
            .iter()
            .filter_map(|day| day.resting_hr.map(|hr| hr as f64))
            .filter(|hr| *hr > 0.0),
    );
    let baseline_hr = mean(
        baseline_health
            .iter()
            .filter_map(|day| day.resting_hr.map(|hr| hr as f64))
            .filter(|hr| *hr > 0.0),
    );
    if health.len() >= 3 && baseline_health.len() >= 7 {
        if matches!((recent_hrv, baseline_hrv), (Some(recent), Some(baseline)) if recent < baseline * 0.8)
            || matches!((recent_hr, baseline_hr), (Some(recent), Some(baseline)) if recent > baseline + 7.0)
        {
            evidence.capacity.fatigue_high = Some(true);
        }
    }
    Ok(evidence)
}

fn positive(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && *value > 0.0)
}

fn usable(anchor: &Estimate) -> bool {
    positive(Some(anchor.value)).is_some()
        && anchor.confidence.is_finite()
        && anchor.confidence > 0.0
        && anchor.relative_uncertainty.is_finite()
        && (0.0..=0.25).contains(&anchor.relative_uncertainty)
}

fn mean(values: impl Iterator<Item = f64>) -> Option<f64> {
    let values: Vec<_> = values.filter(|value| value.is_finite()).collect();
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

pub fn analyze_session(
    intended: Domain,
    reps: &[Repetition],
    lt2: &ThresholdPoint,
    confounded: bool,
) -> SessionAnalysis {
    let reps: Vec<_> = reps
        .iter()
        .filter(|rep| rep.duration_s.is_finite() && rep.duration_s >= 30.0)
        .collect();
    let mut intensities = Vec::new();
    let mut sources = Vec::new();
    let mut modalities = [false; 4];
    for rep in &reps {
        let mut signals = Vec::new();
        if let (Some(pace), Some(anchor)) = (positive(rep.pace_s_km), &lt2.pace_s_km) {
            if usable(anchor) {
                signals.push(anchor.value / pace);
                sources.push(anchor.confidence.min(0.45) * (1.0 - anchor.relative_uncertainty));
                modalities[0] = true;
            }
        }
        if rep.duration_s >= 180.0 {
            if let (Some(hr), Some(anchor)) = (positive(rep.hr), &lt2.hr) {
                if usable(anchor) {
                    signals.push(hr / anchor.value);
                    sources.push(
                        anchor.confidence.clamp(0.0, 0.65) * (1.0 - anchor.relative_uncertainty),
                    );
                    modalities[1] = true;
                }
            }
        }
        if let (Some(power), Some(anchor)) = (positive(rep.power_w), &lt2.power_w) {
            if usable(anchor) {
                signals.push(power / anchor.value);
                sources
                    .push(anchor.confidence.clamp(0.0, 0.60) * (1.0 - anchor.relative_uncertainty));
                modalities[2] = true;
            }
        }
        if let (Some(lactate), Some(anchor), Some(delay)) = (
            positive(rep.lactate_mmol),
            &lt2.lactate_mmol,
            rep.lactate_delay_s,
        ) {
            if usable(anchor)
                && delay.is_finite()
                && delay >= 0.0
                && lt2.lactate_delay_s.is_some_and(|reference| {
                    reference.is_finite() && reference >= 0.0 && (reference - delay).abs() <= 15.0
                })
            {
                signals.push(1.0 + (lactate / anchor.value - 1.0) * 0.15);
                sources.push(anchor.confidence.min(0.85) * (1.0 - anchor.relative_uncertainty));
                modalities[3] = true;
            }
        }
        if let Some(intensity) = signals
            .into_iter()
            .filter(|value| value.is_finite())
            .reduce(f64::max)
        {
            intensities.push(intensity);
        }
    }
    let intensity = mean(intensities.iter().copied());
    let mut reasons = Vec::new();
    let mut penalty: f64 = 0.0;
    if let Some(intensity) = intensity {
        penalty += ((intensity - 1.0).max(0.0) * 500.0).min(60.0);
    }
    let rpe = mean(
        reps.iter()
            .filter_map(|rep| positive(rep.rpe))
            .filter(|rpe| *rpe <= 10.0),
    );
    if rpe.is_some_and(|rpe| rpe >= 8.0) {
        penalty += 25.0;
        reasons.push("high perceived exertion".into());
    }
    if reps
        .iter()
        .any(|rep| rep.talk_test_comfortable == Some(false))
    {
        penalty += 10.0;
        reasons
            .push("breathing/talk-test suggests limited control; not a lactate measurement".into());
    }
    if let (Some(first), Some(last)) = (reps.first(), reps.last()) {
        if let (Some(start), Some(end)) = (positive(first.pace_s_km), positive(last.pace_s_km)) {
            if end / start > 1.05 {
                penalty += 20.0;
                reasons.push("workload deterioration".into());
            }
            if start / end > 1.05 {
                penalty += 15.0;
                reasons.push("unnecessary pace escalation".into());
            }
        }
        if let (Some(start), Some(end)) = (positive(first.hr), positive(last.hr)) {
            let stable_workload = matches!((positive(first.pace_s_km), positive(last.pace_s_km)), (Some(first), Some(last)) if (last / first - 1.0).abs() <= 0.02)
                || matches!((positive(first.power_w), positive(last.power_w)), (Some(first), Some(last)) if (last / first - 1.0).abs() <= 0.02);
            if stable_workload && end / start > 1.05 {
                penalty += 15.0;
                reasons
                    .push("heart-rate progression; fatigue or environment may contribute".into());
            }
        }
        if let (Some(start), Some(end), Some(first_delay), Some(last_delay)) = (
            positive(first.lactate_mmol),
            positive(last.lactate_mmol),
            first.lactate_delay_s,
            last.lactate_delay_s,
        ) {
            if first_delay.is_finite()
                && last_delay.is_finite()
                && first_delay >= 0.0
                && last_delay >= 0.0
                && (first_delay - last_delay).abs() <= 15.0
                && end - start > 1.0
                && end / start > 1.25
            {
                penalty += 30.0;
                reasons.push("lactate accumulation at comparable sampling timing".into());
            }
        }
        if last.recovery_s > first.recovery_s * 1.5 && last.recovery_s - first.recovery_s > 30.0 {
            penalty += 15.0;
            reasons.push("increasing recovery requirements".into());
        }
    }
    let confidence = if sources.is_empty() {
        0.0
    } else {
        let strongest = sources.iter().copied().fold(0.0, f64::max);
        (strongest
            + if modalities.iter().filter(|present| **present).count() >= 2 {
                0.05
            } else {
                0.0
            })
        .min(if modalities[3] { 0.90 } else { 0.65 })
            * if confounded { 0.5 } else { 1.0 }
    };
    let control = intensity.map(|_| (100.0 - penalty).clamp(0.0, 100.0));
    let overshot = control.is_some_and(|control| control < 65.0);
    let observed = match intensity {
        None => Domain::Uncertain,
        Some(value) if value > 1.08 || (overshot && rpe.is_some_and(|rpe| rpe >= 9.0)) => {
            Domain::AboveThreshold
        }
        Some(value) if value > 1.025 || overshot => Domain::HighThreshold,
        Some(value) if value < 0.94 => Domain::SubThreshold,
        Some(_) => Domain::Threshold,
    };
    let work_minutes = reps.iter().map(|rep| rep.duration_s / 60.0).sum::<f64>();
    let lactate_samples: Vec<_> = reps
        .iter()
        .filter_map(|rep| Some((positive(rep.lactate_mmol)?, rep.lactate_delay_s?)))
        .filter(|(_, delay)| delay.is_finite() && *delay >= 0.0)
        .collect();
    let lactate_delta = lactate_samples
        .first()
        .zip(lactate_samples.last())
        .filter(|(first, last)| lactate_samples.len() >= 2 && (first.1 - last.1).abs() <= 15.0)
        .map(|(first, last)| last.0 - first.0);
    SessionAnalysis {
        intended_session_type: intended,
        observed_session_type: observed,
        threshold_intensity_estimate: intensity,
        threshold_control: control,
        threshold_load: if intensity.is_some() {
            work_minutes * intensity.unwrap().clamp(0.5, 1.5).powi(3) * 2.0
        } else {
            0.0
        },
        work_minutes,
        confidence,
        overshot,
        lactate_mean_mmol: mean(lactate_samples.iter().map(|(value, _)| *value)),
        lactate_delta_mmol: lactate_delta,
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 1).unwrap()
    }

    fn conditions() -> Conditions {
        Conditions {
            temperature_c: Some(15.0),
            elevation_gain_m_per_km: Some(0.0),
            altitude_m: Some(100.0),
            fatigued: Some(false),
            tapered: Some(false),
            sensor_suspect: false,
        }
    }

    fn session(days: i64, pace: f64) -> SessionEvidence {
        SessionEvidence {
            date: today() - chrono::Duration::days(days),
            intended: Domain::Threshold,
            repetitions: vec![
                Repetition {
                    duration_s: 360.0,
                    recovery_s: 60.0,
                    pace_s_km: Some(pace),
                    hr: Some(170.0),
                    power_w: Some(300.0),
                    rpe: Some(6.0),
                    ..Default::default()
                };
                5
            ],
            conditions: conditions(),
            planned_work_s: Some(1800.0),
            recovery_hours: Some(24.0),
            following_performance_declined: Some(false),
        }
    }

    fn evidence() -> Evidence {
        Evidence {
            observations: vec![ThresholdObservation {
                date: today(),
                source: MeasurementSource::LactateTest,
                lt1: ThresholdPoint {
                    hr: Some(anchor(145.0)),
                    ..Default::default()
                },
                lt2: ThresholdPoint {
                    pace_s_km: Some(anchor(300.0)),
                    hr: Some(anchor(170.0)),
                    power_w: Some(anchor(300.0)),
                    ..Default::default()
                },
            }],
            sessions: [1, 4, 8, 12, 16, 20]
                .into_iter()
                .map(|days| session(days, 303.0))
                .collect(),
            capacity: Capacity {
                training_age_years: Some(5.0),
                weeks_consistent: Some(40.0),
                weekly_km: Some(90.0),
                historical_weekly_km: Some(90.0),
                weekly_sessions: Some(9.0),
                threshold_months: Some(24.0),
                easy_time_fraction: Some(0.85),
                sleep_hours: Some(8.0),
                fatigue_high: Some(false),
                injury_active: Some(false),
                injury_last_year: Some(false),
                recovery_good: Some(true),
                recovery_time_available: Some(true),
            },
            ..Default::default()
        }
    }

    #[test]
    fn unknown_evidence_is_not_high_capacity_or_measured_threshold() {
        let state = athlete_state(&Evidence::default(), today(), 300.0);
        assert_eq!(state.double_threshold_tolerance.level, Tolerance::Unknown);
        assert_eq!(state.threshold_tolerance.level, Tolerance::Unknown);
        assert_eq!(state.fitness_trend, FitnessTrend::Uncertain);
        assert!(state.lt1_estimate.hr.is_none() && state.lt2_estimate.hr.is_none());
        assert_eq!(state.threshold_confidence, 0.3);
        assert_eq!(state.lactate_profile_confidence, 0.0);
        assert!(!state.allow_second_quality);
        assert_eq!(state.quality_minutes_cap, 15.0);
    }

    #[test]
    fn stale_and_future_lactate_tests_cannot_create_fresh_confidence() {
        let mut input = evidence();
        input.observations[0].lt2.lactate_mmol = Some(anchor(3.2));
        input.observations[0].lt2.lactate_delay_s = Some(60.0);
        let fresh = athlete_state(&input, today(), 300.0);
        input.observations[0].date = today() - chrono::Duration::days(112);
        let stale = athlete_state(&input, today(), 300.0);
        assert!(stale.lactate_profile_confidence < fresh.lactate_profile_confidence / 3.0);
        input.observations[0].date = today() + chrono::Duration::days(1);
        assert_eq!(
            athlete_state(&input, today(), 300.0).lactate_profile_confidence,
            0.0
        );
    }

    #[test]
    fn lactate_values_are_athlete_specific_and_sampling_timing_matters() {
        let reps = vec![
            Repetition {
                duration_s: 360.0,
                lactate_mmol: Some(4.0),
                lactate_delay_s: Some(60.0),
                ..Default::default()
            };
            5
        ];
        let mut point = ThresholdPoint {
            lactate_mmol: Some(anchor(4.0)),
            lactate_delay_s: Some(60.0),
            ..Default::default()
        };
        assert_eq!(
            analyze_session(Domain::Threshold, &reps, &point, false).observed_session_type,
            Domain::Threshold
        );
        point.lactate_mmol = Some(anchor(2.0));
        assert_eq!(
            analyze_session(Domain::Threshold, &reps, &point, false).observed_session_type,
            Domain::AboveThreshold
        );
        point.lactate_delay_s = Some(180.0);
        assert_eq!(
            analyze_session(Domain::Threshold, &reps, &point, false).observed_session_type,
            Domain::Uncertain
        );
    }

    #[test]
    fn interval_structures_share_work_volume_but_not_hr_latency_or_control() {
        let point = evidence().observations[0].lt2.clone();
        for (count, duration) in [(10, 180.0), (5, 360.0), (3, 600.0)] {
            let reps = vec![
                Repetition {
                    duration_s: duration,
                    recovery_s: 60.0,
                    pace_s_km: Some(303.0),
                    hr: Some(170.0),
                    ..Default::default()
                };
                count
            ];
            let analysis = analyze_session(Domain::Threshold, &reps, &point, false);
            assert_eq!(analysis.work_minutes, 30.0);
            assert_eq!(analysis.observed_session_type, Domain::Threshold);
        }
        let short = [Repetition {
            duration_s: 60.0,
            hr: Some(190.0),
            ..Default::default()
        }];
        assert_eq!(
            analyze_session(Domain::Threshold, &short, &point, false).observed_session_type,
            Domain::Uncertain
        );
    }

    #[test]
    fn repeated_overshooting_lowers_control_and_prescription_not_rewards_pace() {
        let mut input = evidence();
        let controlled = athlete_state(&input, today(), 300.0);
        for session in input.sessions.iter_mut().take(2) {
            for rep in &mut session.repetitions {
                rep.pace_s_km = Some(270.0);
                rep.hr = Some(190.0);
                rep.rpe = Some(9.0);
            }
        }
        let state = athlete_state(&input, today(), 300.0);
        assert_eq!(state.repeated_overshoots, 2);
        assert!(state.threshold_control < controlled.threshold_control);
        assert_eq!(state.quality_minutes_cap, 12.0);
        assert!(!state.allow_second_quality);
        assert_eq!(
            state
                .sessions
                .last()
                .unwrap()
                .analysis
                .intended_session_type,
            Domain::Threshold
        );
        assert_eq!(
            state
                .sessions
                .last()
                .unwrap()
                .analysis
                .observed_session_type,
            Domain::AboveThreshold
        );
    }

    #[test]
    fn doubles_require_history_recovery_and_unique_days_not_fast_times() {
        let mut input = evidence();
        assert_eq!(
            athlete_state(&input, today(), 300.0)
                .double_threshold_tolerance
                .level,
            Tolerance::High
        );
        input.capacity.recovery_time_available = None;
        assert_eq!(
            athlete_state(&input, today(), 300.0)
                .double_threshold_tolerance
                .level,
            Tolerance::Unknown
        );
        input.capacity.sleep_hours = Some(5.0);
        let state = athlete_state(&input, today(), 300.0);
        assert_eq!(state.double_threshold_tolerance.level, Tolerance::Low);
        assert!(state.hold_quality);
        assert_eq!(state.quality_minutes_cap, 0.0);
        input = evidence();
        input.sessions = vec![session(1, 303.0); 8];
        assert_ne!(
            athlete_state(&input, today(), 300.0)
                .double_threshold_tolerance
                .level,
            Tolerance::High
        );
    }

    #[test]
    fn threshold_density_and_recovery_are_not_total_running_mileage() {
        let mut input = evidence();
        let state = athlete_state(&input, today(), 300.0);
        assert_eq!(state.threshold_volume_7d, 60.0);
        assert_eq!(state.threshold_volume_28d, 180.0);
        assert_eq!(state.threshold_frequency, 2);
        assert!(state.threshold_load > state.threshold_volume_7d);
        input.sessions[0].following_performance_declined = Some(true);
        input.sessions[0].planned_work_s = Some(3600.0);
        assert!(athlete_state(&input, today(), 300.0).threshold_control < state.threshold_control);
    }

    #[test]
    fn adaptation_requires_repeated_environment_matched_sessions() {
        let mut input = evidence();
        input.sessions = vec![
            session(35, 315.0),
            session(28, 315.0),
            session(7, 300.0),
            session(1, 300.0),
        ];
        assert_eq!(
            athlete_state(&input, today(), 300.0).fitness_trend,
            FitnessTrend::Improving
        );
        for session in &mut input.sessions {
            session.conditions.temperature_c = None;
        }
        assert_eq!(
            athlete_state(&input, today(), 300.0).fitness_trend,
            FitnessTrend::Uncertain
        );
        for session in &mut input.sessions {
            session.conditions = conditions();
            session.conditions.tapered = Some(true);
        }
        assert_eq!(
            athlete_state(&input, today(), 300.0).fitness_trend,
            FitnessTrend::Uncertain
        );
    }

    #[test]
    fn critical_speed_is_distinct_and_requires_comparable_maximal_efforts() {
        let mut input = evidence();
        input.performances = vec![
            Performance {
                date: today(),
                duration_s: 180.0,
                distance_m: 1000.0,
                power_w: Some(400.0),
                maximal: true,
                conditions: conditions(),
            },
            Performance {
                date: today() - chrono::Duration::days(7),
                duration_s: 720.0,
                distance_m: 3400.0,
                power_w: Some(330.0),
                maximal: true,
                conditions: conditions(),
            },
        ];
        let state = athlete_state(&input, today(), 300.0);
        assert!((state.critical_speed_m_s.unwrap().value - 4.444444).abs() < 0.001);
        assert!(state.critical_power_w.is_some());
        assert_eq!(state.lt2_estimate.pace_s_km.unwrap().value, 300.0);
        input.performances[0].conditions.sensor_suspect = true;
        assert!(athlete_state(&input, today(), 300.0)
            .critical_speed_m_s
            .is_none());
    }

    #[test]
    fn repetition_count_does_not_inflate_proxy_confidence() {
        let lt2 = ThresholdPoint {
            pace_s_km: Some(anchor(300.0)),
            ..Default::default()
        };
        let rep = Repetition {
            duration_s: 180.0,
            pace_s_km: Some(300.0),
            ..Default::default()
        };
        let single = analyze_session(Domain::Threshold, &[rep.clone()], &lt2, false);
        let many = analyze_session(Domain::Threshold, &vec![rep; 10], &lt2, false);
        assert_eq!(single.confidence, many.confidence);
        assert!(many.confidence <= 0.45);
        assert!(analyze_session(Domain::Threshold, &[], &lt2, false).confidence == 0.0);
    }

    fn anchor(value: f64) -> Estimate {
        Estimate {
            value,
            confidence: 0.7,
            relative_uncertainty: 0.05,
            source: "test".into(),
        }
    }

    #[test]
    fn threshold_control_distinguishes_controlled_overshot_and_unknown() {
        let lt2 = ThresholdPoint {
            pace_s_km: Some(anchor(300.0)),
            hr: Some(anchor(170.0)),
            ..Default::default()
        };
        let reps = vec![
            Repetition {
                duration_s: 360.0,
                pace_s_km: Some(303.0),
                hr: Some(167.0),
                recovery_s: 60.0,
                ..Default::default()
            };
            5
        ];
        let controlled = analyze_session(Domain::Threshold, &reps, &lt2, false);
        assert_eq!(controlled.observed_session_type, Domain::Threshold);
        assert_eq!(controlled.threshold_control, Some(100.0));
        assert!((controlled.work_minutes - 30.0).abs() < 0.01);
        assert!(controlled.threshold_load > 50.0);
        let hard = vec![
            Repetition {
                pace_s_km: Some(270.0),
                hr: Some(185.0),
                rpe: Some(9.0),
                ..reps[0].clone()
            };
            5
        ];
        let overshot = analyze_session(Domain::Threshold, &hard, &lt2, false);
        assert_eq!(overshot.intended_session_type, Domain::Threshold);
        assert_eq!(overshot.observed_session_type, Domain::AboveThreshold);
        assert!(overshot.overshot);
        let unknown = analyze_session(Domain::Threshold, &[], &lt2, false);
        assert_eq!(unknown.observed_session_type, Domain::Uncertain);
        assert_eq!(unknown.threshold_control, None);
        assert_eq!(unknown.confidence, 0.0);
    }

    #[test]
    fn invalid_anchors_and_unmatched_lactate_timing_are_not_evidence() {
        let lt2 = ThresholdPoint {
            hr: Some(anchor(0.0)),
            power_w: Some(anchor(f64::NAN)),
            lactate_mmol: Some(anchor(2.0)),
            lactate_delay_s: Some(60.0),
            ..Default::default()
        };
        let reps = [Repetition {
            duration_s: 360.0,
            hr: Some(170.0),
            power_w: Some(300.0),
            lactate_mmol: Some(4.0),
            lactate_delay_s: Some(180.0),
            ..Default::default()
        }];
        let result = analyze_session(Domain::Threshold, &reps, &lt2, false);
        assert_eq!(result.threshold_intensity_estimate, None);
        assert_eq!(result.threshold_load, 0.0);
        assert_eq!(result.confidence, 0.0);
    }
}
