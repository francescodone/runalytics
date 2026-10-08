//! Translating sidecar JSON into provider-core drafts.
//!
//! Community Garmin MCP servers wrap payloads inconsistently — bare arrays,
//! `{data: [...]}`, `{result: {...}}` — and rename fields between versions.
//! Every read here is alias-tolerant: try the known spellings in order, and
//! drop a record only when it has no usable identity at all. A silently
//! dropped field is recoverable; a fabricated one poisons the scoring.
//!
//! Garmin reports durations in seconds and distances in metres, but the
//! sleep tools historically report *milliseconds*. The same >= 48h heuristic
//! the COROS adapter uses heals that without trusting a field name.

use chrono::{DateTime, NaiveDateTime, TimeZone};
use runalytics_core::{
    ActivityLap, ActivitySummary, Date, DurationSecs, FitnessAssessment, HeartRate, Pace, SleepDay,
    Timestamp, Tz, VolumeKm,
};
use runalytics_provider_core::{ActivityDraft, FitnessDraft, HealthDraft, ProviderError};
use serde_json::Value;

/// First non-empty string among `keys` in `obj`.
pub fn str_field<'a>(obj: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .filter_map(|k| obj.get(*k))
        .find_map(Value::as_str)
        .filter(|s| !s.is_empty())
}

/// First numeric value among `keys`, as f64.
pub fn num_field(obj: &Value, keys: &[&str]) -> Option<f64> {
    keys.iter().filter_map(|k| obj.get(*k)).find_map(|v| {
        v.as_f64()
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
    })
}

/// Heal a duration that may be reported in milliseconds: anything >= 48h in
/// seconds is physically implausible for one effort, so treat it as ms.
/// (Sleep windows in ms legitimately exceed 86 400, which is why the old
/// one-day threshold was wrong.)
#[must_use]
pub fn duration_secs(raw: f64) -> DurationSecs {
    if raw >= 172_800.0 {
        DurationSecs((raw / 1000.0).round() as u32)
    } else {
        DurationSecs(raw.round() as u32)
    }
}

/// Parse a Garmin timestamp: ISO-8601 with offset, or a naive local string
/// (`2026-10-01T07:30:12` / `2026-10-01 07:30:12`) interpreted in the
/// athlete's zone — Garmin's `*Local` fields are exactly that.
pub fn parse_ts(raw: &str, tz: &Tz) -> Option<Timestamp> {
    if let Ok(ok) = DateTime::parse_from_rfc3339(raw) {
        return Some(ok.with_timezone(&chrono::Utc));
    }
    let naive = NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S"))
        .or_else(|_| NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M"))
        .ok()?;
    tz.from_local_datetime(&naive).single().map(|z| z.to_utc())
}

/// The local calendar day of an instant, in the athlete's zone.
#[must_use]
pub fn local_date(ts: Timestamp, tz: &Tz) -> Date {
    ts.with_timezone(tz).date_naive()
}

/// Unwrap the record array out of the wrapper shapes sidecars emit.
pub fn record_array(value: &Value, provider: &str) -> Result<Vec<Value>, ProviderError> {
    let arr = match value {
        Value::Array(a) => Some(a),
        Value::Object(obj) => ["data", "activities", "results", "result", "records"]
            .iter()
            .filter_map(|k| obj.get(*k))
            .find_map(Value::as_array),
        _ => None,
    };
    arr.cloned().ok_or_else(|| ProviderError::Malformed {
        provider: provider.to_owned(),
        detail: "expected a record array (bare or under data/activities/results)".to_owned(),
        sample: Some(value.to_string().chars().take(400).collect()),
    })
}

/// One activity record -> draft. `None` drops the record (no id or no start).
pub fn map_activity(raw: &Value, tz: &Tz) -> Option<ActivityDraft> {
    let id = str_field(raw, &["activityId", "activity_id", "id", "workoutId"])
        .map(str::to_owned)
        .or_else(|| num_field(raw, &["activityId", "activity_id"]).map(|n| n.to_string()))?;
    let started_at = str_field(
        raw,
        &["startTimeLocal", "startTime", "start_time", "beginTime"],
    )
    .and_then(|s| parse_ts(s, tz))
    // Some servers report only epoch seconds for the UTC start.
    .or_else(|| {
        num_field(raw, &["startTimeGMT", "startTime", "epochStart"]).map(|e| {
            let secs = if e > 1e11 { e / 1000.0 } else { e };
            chrono::Utc.timestamp_opt(secs as i64, 0).single()
        })?
    })?;

    let distance_m = num_field(raw, &["distance", "distanceMeters"]).unwrap_or(0.0);
    let duration =
        num_field(raw, &["duration", "movingDuration", "durationSeconds"]).map(duration_secs);
    let avg_pace = match (distance_m, duration) {
        (d, Some(dur)) if d > 0.0 && dur.as_u32() > 0 => {
            Pace::from_parts(VolumeKm(d / 1000.0), dur)
        }
        _ => num_field(raw, &["averagePace", "avgPace"]).map(Pace::new),
    };

    let laps = raw
        .get("laps")
        .and_then(Value::as_array)
        .map(|laps| {
            laps.iter()
                .enumerate()
                .filter_map(|(index, lap)| {
                    let duration = num_field(lap, &["duration", "elapsedDuration"])?;
                    Some(ActivityLap {
                        index: u32::try_from(index).unwrap_or(u32::MAX),
                        start: str_field(lap, &["startTimeLocal", "startGMT"])
                            .and_then(|s| parse_ts(s, tz))
                            .unwrap_or(started_at),
                        duration: duration_secs(duration),
                        distance: VolumeKm(
                            num_field(lap, &["distance", "distanceMeters"]).unwrap_or(0.0) / 1000.0,
                        ),
                        avg_pace: num_field(lap, &["averagePace", "runTime"])
                            .filter(|p| *p > 60.0)
                            .map(Pace::new),
                        avg_hr: num_field(lap, &["averageHR", "averageHeartRate"])
                            .map(|h| HeartRate::new(h as u16)),
                        max_hr: num_field(lap, &["maxHR", "maxHeartRate"])
                            .map(|h| HeartRate::new(h as u16)),
                        elevation_gain: num_field(lap, &["elevationGain", "elevGain"]),
                        cadence: num_field(lap, &["averageRunCadence", "averageCadence"])
                            .map(|c| c as u16),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Some(ActivityDraft {
        provider_activity_id: id,
        name: str_field(raw, &["activityName", "activity_name", "name", "title"])
            .unwrap_or("Garmin activity")
            .to_owned(),
        started_at,
        local_date: local_date(started_at, tz),
        summary: ActivitySummary {
            distance: VolumeKm(distance_m / 1000.0),
            duration: duration.unwrap_or(DurationSecs::ZERO),
            avg_pace,
            avg_hr: num_field(raw, &["averageHeartRate", "averageHR", "avg_hr"])
                .map(|h| HeartRate::new(h as u16)),
            max_hr: num_field(raw, &["maxHeartRate", "maxHR"]).map(|h| HeartRate::new(h as u16)),
            elevation_gain: num_field(raw, &["elevationGain", "elevGain"]),
            avg_cadence: num_field(
                raw,
                &[
                    "averageRunningCadenceStepsPerMinute",
                    "averageRunCadence",
                    "averageCadence",
                ],
            )
            .map(|c| c as u16),
            training_load: num_field(raw, &["trainingEffect", "aerobicTrainingEffect"]),
        },
        laps,
        provider_intensity_label: str_field(raw, &["intensity", "intensityMinutes"])
            .map(str::to_owned),
    })
}

/// A sleep record -> the sleep portion of a health draft for `date`.
///
/// Sidecar sleep payloads key the night by onset but the app attributes it to
/// the morning it *ends*: a night ending Monday belongs to Monday. If the
/// record carries an end timestamp, that wins over the caller's fallback.
pub fn map_sleep(raw: &Value, fallback: Date, tz: &Tz) -> Option<HealthDraft> {
    let end_date = str_field(raw, &["sleepEndLocal", "wakeTime", "endDateTimeLocal"])
        .and_then(|s| parse_ts(s, tz))
        .map(|ts| local_date(ts, tz))
        .or_else(|| {
            str_field(raw, &["date", "calendarDate", "sleepDate"])
                .and_then(|s| Date::parse_from_str(s, "%Y-%m-%d").ok())
        })
        .unwrap_or(fallback);

    let seconds = |keys: &[&str]| num_field(raw, keys).map(duration_secs);
    let total = seconds(&[
        "sleepTimeSeconds",
        "durationInSeconds",
        "totalSleepTime",
        "sleepDuration",
    ])?;
    if total.as_u32() == 0 {
        return None;
    }

    Some(HealthDraft {
        date: end_date,
        resting_hr: None,
        avg_stress: None,
        high_stress_minutes: None,
        steps: None,
        provider_readiness: None,
        basal_energy: None,
        sleep: Some(SleepDay {
            date: end_date,
            total,
            deep: seconds(&["deepSleepSeconds", "deepSleepDuration"]).unwrap_or_default(),
            light: seconds(&["lightSleepSeconds", "lightSleepDuration"]).unwrap_or_default(),
            rem: seconds(&["REMsleepSeconds", "remSleepSeconds", "REMsleepDuration"])
                .unwrap_or_default(),
            awake: seconds(&["awakeSleepSeconds", "awakeDuration"]).unwrap_or_default(),
            nap: seconds(&["napSleepDuration"]).unwrap_or_default(),
            score: num_field(raw, &["sleepScore", "sleep_score"])
                .map(|s| s.clamp(0.0, 100.0) as u8),
            lowest_hr: num_field(raw, &["lowestHeartRate", "lowestHR"])
                .map(|h| HeartRate::new(h as u16)),
            hrv: num_field(raw, &["hrvAverage", "averageHRV"]),
            respiratory_rate: num_field(raw, &["averageRespiratoryRate", "respiratoryRate"]),
        }),
    })
}

/// A daily heart record -> resting HR for `date`.
pub fn map_heart(raw: &Value, date: Date) -> Option<HealthDraft> {
    let resting = num_field(raw, &["restingHeartRate", "restingHR", "rhr"])
        .map(|h| HeartRate::new(h as u16))?;
    Some(HealthDraft {
        date,
        resting_hr: Some(resting),
        avg_stress: None,
        high_stress_minutes: None,
        steps: None,
        provider_readiness: None,
        basal_energy: None,
        sleep: None,
    })
}

/// A daily stress record -> avg stress + high-stress minutes for `date`.
pub fn map_stress(raw: &Value, date: Date) -> Option<HealthDraft> {
    let avg = num_field(
        raw,
        &["averageStressLevel", "overallStressLevel", "avgStress"],
    );
    let high = num_field(raw, &["highStressDuration", "stressHighDuration"]).map(duration_secs);
    if avg.is_none() && high.is_none() {
        return None;
    }
    Some(HealthDraft {
        date,
        resting_hr: None,
        avg_stress: avg,
        high_stress_minutes: high.map(DurationSecs::as_minutes),
        steps: None,
        provider_readiness: None,
        basal_energy: None,
        sleep: None,
    })
}

/// A steps record -> step count for `date`.
pub fn map_steps(raw: &Value, date: Date) -> Option<HealthDraft> {
    let steps = num_field(raw, &["totalSteps", "steps", "stepCount"]).map(|s| s as u32)?;
    Some(HealthDraft {
        date,
        resting_hr: None,
        avg_stress: None,
        high_stress_minutes: None,
        steps: Some(steps),
        provider_readiness: None,
        basal_energy: None,
        sleep: None,
    })
}

/// A VO2max record -> fitness draft for `date`.
pub fn map_fitness(raw: &Value, date: Date) -> Option<FitnessDraft> {
    let vo2 = num_field(raw, &["vo2Max", "vo2max", "vo2MaxPreciseValue"]);
    let threshold = num_field(raw, &["thresholdPace", "functionalThresholdPace"])
        .filter(|p| *p > 60.0)
        .map(Pace::new);
    if vo2.is_none() && threshold.is_none() {
        return None;
    }
    Some(FitnessDraft {
        assessment: FitnessAssessment {
            date,
            vo2max: vo2,
            running_level: num_field(raw, &["runningLevel", "performanceIndex"]),
            threshold_pace: threshold,
            predicted_paces: Vec::new(),
        },
    })
}

/// Pull the date out of a record (`date`, `calendarDate`, `activitySummaryDate`),
/// falling back to the single-day window the request was made with.
pub fn record_date(raw: &Value, fallback: Date) -> Date {
    str_field(
        raw,
        &["date", "calendarDate", "activitySummaryDate", "statDate"],
    )
    .and_then(|s| {
        Date::parse_from_str(s, "%Y-%m-%d").ok().or_else(|| {
            // Timestamps like "2026-10-03T00:00:00.000" — take the date
            // prefix by chars so a multibyte boundary cannot panic.
            let prefix: String = s.chars().take(10).collect();
            Date::parse_from_str(&prefix, "%Y-%m-%d").ok()
        })
    })
    .unwrap_or(fallback)
}

/// A `YYYY-MM-DD` argument pair as sidecars expect it.
#[must_use]
pub fn date_args(from: Date, to: Date) -> serde_json::Map<String, Value> {
    let mut args = serde_json::Map::new();
    let f = from.format("%Y-%m-%d").to_string();
    let t = to.format("%Y-%m-%d").to_string();
    args.insert("start_date".into(), Value::String(f.clone()));
    args.insert("end_date".into(), Value::String(t));
    args.insert("startDate".into(), Value::String(f));
    args.insert(
        "endDate".into(),
        Value::String(to.format("%Y-%m-%d").to_string()),
    );
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tz() -> Tz {
        "Europe/Berlin".parse().expect("tz")
    }

    fn date(y: i32, m: u32, d: u32) -> Date {
        Date::from_ymd_opt(y, m, d).expect("date")
    }

    #[test]
    fn activity_maps_with_local_start_and_derived_pace() {
        let raw = json!({
            "activityId": 12345,
            "activityName": "Morning Loop",
            "startTimeLocal": "2026-10-01T07:30:00",
            "distance": 12000.0,
            "duration": 3600.0,
            "averageHeartRate": 152,
            "elevationGain": 87.0,
        });
        let draft = map_activity(&raw, &tz()).expect("maps");
        assert_eq!(draft.provider_activity_id, "12345");
        assert_eq!(draft.name, "Morning Loop");
        assert_eq!(draft.local_date, date(2026, 10, 1));
        assert_eq!(draft.summary.distance.as_f64(), 12.0);
        assert_eq!(draft.summary.duration, DurationSecs(3600));
        assert_eq!(
            draft.summary.avg_pace.expect("pace").as_secs_per_km(),
            300.0
        );
    }

    #[test]
    fn activity_without_id_or_start_is_dropped() {
        assert!(map_activity(&json!({"activityName": "x"}), &tz()).is_none());
    }

    #[test]
    fn sleep_ms_durations_are_healed() {
        let raw = json!({
            "sleepEndLocal": "2026-10-05T07:00:00",
            "sleepTimeSeconds": 28_800_000.0,
            "deepSleepSeconds": 6_400_000.0,
            "sleepScore": 84,
        });
        let draft = map_sleep(&raw, date(2026, 10, 4), &tz()).expect("maps");
        let sleep = draft.sleep.as_ref().expect("sleep");
        // 8h in ms -> 8h in s, and the night belongs to the morning it ends.
        assert_eq!(sleep.total, DurationSecs(28_800));
        assert_eq!(draft.date, date(2026, 10, 5));
        assert_eq!(sleep.score, Some(84));
    }

    #[test]
    fn sleep_without_total_is_dropped() {
        assert!(map_sleep(&json!({"sleepScore": 5}), date(2026, 10, 4), &tz()).is_none());
    }

    #[test]
    fn heart_stress_steps_map_to_their_day() {
        let h = map_heart(&json!({"restingHeartRate": 48}), date(2026, 10, 2)).expect("hr");
        assert_eq!(h.resting_hr, Some(HeartRate::new(48)));
        let s = map_stress(
            &json!({"averageStressLevel": 32.0, "highStressDuration": 3600}),
            date(2026, 10, 2),
        )
        .expect("stress");
        assert_eq!(s.avg_stress, Some(32.0));
        assert_eq!(s.high_stress_minutes, Some(60));
        let st = map_steps(&json!({"totalSteps": 9211}), date(2026, 10, 2)).expect("steps");
        assert_eq!(st.steps, Some(9211));
    }

    #[test]
    fn fitness_maps_vo2_and_threshold() {
        let f = map_fitness(
            &json!({"vo2Max": 54.5, "thresholdPace": 245.0}),
            date(2026, 10, 2),
        )
        .expect("fitness");
        assert_eq!(f.assessment.vo2max, Some(54.5));
        assert_eq!(
            f.assessment.threshold_pace.expect("tp").as_secs_per_km(),
            245.0
        );
        assert!(map_fitness(&json!({"foo": 1}), date(2026, 10, 2)).is_none());
    }

    #[test]
    fn record_array_unwraps_wrappers() {
        let v = json!({"data": [{"a": 1}]});
        assert_eq!(record_array(&v, "garmin").expect("ok").len(), 1);
        let bad = json!({"nothing": true});
        assert!(record_array(&bad, "garmin").is_err());
    }

    #[test]
    fn record_date_reads_aliases_and_falls_back() {
        assert_eq!(
            record_date(&json!({"calendarDate": "2026-10-03"}), date(2026, 1, 1)),
            date(2026, 10, 3)
        );
        assert_eq!(record_date(&json!({}), date(2026, 1, 1)), date(2026, 1, 1));
    }

    #[test]
    fn duration_healing_matches_coros_rule() {
        assert_eq!(duration_secs(3_600.0), DurationSecs(3600));
        assert_eq!(duration_secs(3_600_000.0), DurationSecs(3600));
        // A two-day sleep in ms is still ms; a 47h effort in s stays seconds.
        assert_eq!(duration_secs(171_000.0), DurationSecs(171_000));
    }
}
