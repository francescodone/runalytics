//! COROS payload -> draft mapping.
//!
//! The COROS MCP server's payload shapes are only fully knowable against the
//! live server, and it has shipped field-name changes before. So this module
//! is deliberately *tolerant*: it reads through field aliases and unit
//! heuristics rather than one rigid struct, and it fails with a sample of the
//! body rather than a serde path error. A mapping that panics on an unknown
//! extra field deletes a week of a user's runs; a mapping that logs and skips
//! one record does not.
//!
//! The invariants it does enforce, because guessing wrong on these corrupts
//! training history silently:
//!
//! * Durations are seconds; anything plausibly in milliseconds is converted
//!   (a 2-hour run is 7_200 s or 7_200_000 ms, never 72).
//! * Distances are kilometres; values under 100 with a sibling metre-unit hint
//!   are treated as metres only when the key says so (`distance_m`).
//! * Timestamps are UTC epoch seconds or ISO-8601; local dates are derived in
//!   the *athlete's* zone, never UTC — a 22:00 run must land on its own day.

use runalytics_core::{
    ActivityLap, ActivitySummary, Date, DurationSecs, HeartRate, Pace, Timestamp, VolumeKm,
};
use runalytics_provider_core::{ActivityDraft, FitnessDraft, HealthDraft};
use serde_json::Value;

/// Pull a string field through a list of candidate keys.
fn str_field<'a>(obj: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|k| obj.get(*k))
        .and_then(Value::as_str)
}

/// Pull a numeric field, accepting JSON numbers encoded as strings.
fn num_field(obj: &Value, keys: &[&str]) -> Option<f64> {
    keys.iter().find_map(|k| obj.get(*k)).and_then(|v| {
        v.as_f64()
            .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
    })
}

fn hr_field(obj: &Value, keys: &[&str]) -> Option<HeartRate> {
    num_field(obj, keys).map(|v| HeartRate::new(v.round().max(0.0) as u16))
}

/// Interpret a duration value, healing the seconds/milliseconds ambiguity.
///
/// No recorded effort — run or sleep — lasts two days in seconds, while two
/// days in milliseconds is 172.8 million. So anything at or above 172,800 is
/// read as milliseconds and divided down; anything below is seconds. The old
/// 86,400 s threshold failed on sleep, which legitimately reaches ~40,000 s
/// and is reported in ms by COROS, and a 10-hour sleep stored as 36,000,000
/// ms must not become a 417-day effort.
#[must_use]
pub fn duration_secs(raw: f64) -> DurationSecs {
    if raw >= 172_800.0 {
        DurationSecs::new((raw / 1_000.0).round().max(0.0) as u32)
    } else {
        DurationSecs::new(raw.round().max(0.0) as u32)
    }
}

/// Interpret an epoch value that may be seconds or milliseconds.
fn epoch_ts(raw: f64) -> Option<Timestamp> {
    let secs = if raw > 100_000_000_000.0 {
        // Past 1e11 the value cannot be a sane epoch-seconds (that is 5138 CE).
        raw / 1_000.0
    } else {
        raw
    };
    chrono::DateTime::<chrono::Utc>::from_timestamp(secs.round() as i64, 0)
}

fn parse_ts(obj: &Value, keys: &[&str]) -> Option<Timestamp> {
    if let Some(n) = num_field(obj, keys) {
        return epoch_ts(n);
    }
    str_field(obj, keys).and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(s)
            .map(|d| d.with_timezone(&chrono::Utc))
            .ok()
            .or_else(|| {
                chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                    .map(|n| n.and_utc())
                    .ok()
            })
    })
}

/// The local calendar day of an instant, in the athlete's zone.
///
/// A run that starts 22:00 Berlin in summer (UTC+2) is 20:00 UTC and still
/// belongs to its own evening; attributing by UTC day would move it and skew
/// every daily model. Falls back to the UTC day only if the zone conversion
/// itself fails, because a day one hour off is recoverable and a dropped
/// activity is not.
#[must_use]
pub fn local_date(ts: Timestamp, tz: &runalytics_core::Tz) -> Date {
    ts.with_timezone(tz).date_naive()
}

/// One lap from a COROS lap/segment object.
fn map_lap(raw: &Value, index: u32) -> Option<ActivityLap> {
    let duration = num_field(raw, &["duration", "durationSec", "time"]).map(duration_secs)?;
    let distance = num_field(raw, &["distance", "distanceKm", "distance_km"]).unwrap_or(0.0);
    Some(ActivityLap {
        index,
        start: parse_ts(raw, &["startTime", "start_time", "beginTime"]).unwrap_or_default(),
        duration,
        distance: VolumeKm::new(distance),
        avg_pace: num_field(raw, &["avgPace", "avg_pace", "pace"]).map(Pace::new),
        avg_hr: hr_field(raw, &["avgHr", "avg_hr", "heartRate"]),
        max_hr: hr_field(raw, &["maxHr", "max_hr", "maxHeartRate"]),
        elevation_gain: num_field(raw, &["elevationGain", "elevation_gain", "ascend"]),
        cadence: num_field(raw, &["cadence", "avgCadence"]).map(|v| v.round() as u16),
    })
}

/// One activity from a COROS activity-detail object.
///
/// `None` (skip) only when the record has neither a start time nor a duration
/// — there is no honest way to place it on a calendar.
#[must_use]
pub fn map_activity(raw: &Value, tz: &runalytics_core::Tz) -> Option<ActivityDraft> {
    let id = str_field(raw, &["id", "activityId", "activity_id", "workoutId"])?.to_owned();
    let started_at = parse_ts(raw, &["startTime", "start_time", "beginTime", "start"]);
    let duration = num_field(raw, &["duration", "durationSec", "totalTime", "moveTime"])
        .map_or(DurationSecs::ZERO, duration_secs);
    let started_at = started_at.unwrap_or_default();
    let distance = num_field(raw, &["distance", "distanceKm", "distance_km"]).unwrap_or(0.0);

    let laps: Vec<ActivityLap> = raw
        .get("laps")
        .or_else(|| raw.get("segments"))
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .enumerate()
                .filter_map(|(i, lap)| map_lap(lap, u32::try_from(i).unwrap_or(u32::MAX)))
                .collect()
        })
        .unwrap_or_default();

    let avg_pace = num_field(raw, &["avgPace", "avg_pace"])
        .map(Pace::new)
        .or_else(|| Pace::from_parts(VolumeKm::new(distance), duration));

    Some(ActivityDraft {
        provider_activity_id: id,
        name: str_field(raw, &["label", "name", "title", "sportType"])
            .unwrap_or("COROS activity")
            .to_owned(),
        local_date: local_date(started_at, tz),
        started_at,
        summary: ActivitySummary {
            distance: VolumeKm::new(distance),
            duration,
            avg_pace,
            avg_hr: hr_field(raw, &["avgHr", "avg_hr", "averageHeartRate"]),
            max_hr: hr_field(raw, &["maxHr", "max_hr", "maxHeartRate"]),
            elevation_gain: num_field(raw, &["elevationGain", "elevation_gain", "ascend"]),
            avg_cadence: num_field(raw, &["avgCadence", "cadence"]).map(|v| v.round() as u16),
            training_load: num_field(raw, &["trainingLoad", "training_load", "stress"]),
        },
        laps,
        provider_intensity_label: str_field(raw, &["intensity", "intensityLevel"])
            .map(str::to_owned),
    })
}

/// One health day from a COROS daily/sleep payload.
///
/// `date` is the recovery day (the morning the night ended) — COROS keys its
/// sleep records by onset, so a night starting Sunday 23:00 is attributed to
/// Monday before this returns.
#[must_use]
pub fn map_health(raw: &Value, date: Date) -> Option<HealthDraft> {
    let has_anything = ["restingHeartRate", "resting_hr", "sleep", "stress", "steps"]
        .iter()
        .any(|k| raw.get(*k).is_some());
    if !has_anything {
        return None;
    }
    let sleep = raw.get("sleep").map(|s| {
        let stage = |keys: &[&str]| duration_secs(num_field(s, keys).unwrap_or(0.0));
        runalytics_core::SleepDay {
            date,
            total: stage(&["total", "sleepDuration", "sleep_duration", "duration"]),
            deep: stage(&["deep", "deepSleep", "deep_sleep"]),
            light: stage(&["light", "lightSleep", "light_sleep"]),
            rem: stage(&["rem", "remSleep", "rem_sleep", "dream"]),
            awake: stage(&["awake", "awakeDuration", "awake_duration"]),
            nap: stage(&["nap", "napDuration"]),
            score: num_field(s, &["score", "sleepScore"])
                .map(|v| v.round().clamp(0.0, 100.0) as u8),
            lowest_hr: hr_field(s, &["lowestHr", "lowest_hr", "minHeartRate"]),
            hrv: num_field(s, &["hrv", "hrvValue", "hrv_value"]),
            respiratory_rate: num_field(s, &["respiratoryRate", "respiratory_rate"]),
        }
    });
    Some(HealthDraft {
        date,
        resting_hr: hr_field(raw, &["restingHeartRate", "resting_hr", "restingHr"]),
        avg_stress: num_field(raw, &["avgStress", "avg_stress", "stress"]),
        high_stress_minutes: num_field(raw, &["highStressMinutes", "high_stress_minutes"])
            .map(|v| v.round() as u32),
        sleep,
        steps: num_field(raw, &["steps", "stepCount"]).map(|v| v.round() as u32),
        provider_readiness: num_field(raw, &["readiness", "trainingReadiness", "recovery"])
            .map(|v| v.round().clamp(0.0, 100.0) as u8),
        basal_energy: num_field(raw, &["basalEnergy", "basal_energy", "restingEnergy"]),
    })
}

/// One fitness assessment from a COROS fitness/level payload.
#[must_use]
pub fn map_fitness(raw: &Value, date: Date) -> Option<FitnessDraft> {
    let vo2max = num_field(raw, &["vo2max", "vo2Max", "vo2_max"]);
    let level = num_field(raw, &["runningLevel", "running_level", "level"]);
    let threshold = num_field(raw, &["thresholdPace", "threshold_pace"]).map(Pace::new);
    if vo2max.is_none() && level.is_none() && threshold.is_none() {
        return None;
    }
    let predicted = raw
        .get("predictedPaces")
        .or_else(|| raw.get("race_predictions"))
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .filter_map(|(k, v)| v.as_f64().map(|v| (k.clone(), v)))
                .collect()
        })
        .unwrap_or_default();
    Some(FitnessDraft {
        assessment: runalytics_core::FitnessAssessment {
            date,
            vo2max,
            running_level: level,
            threshold_pace: threshold,
            predicted_paces: predicted,
        },
    })
}

/// Extract the array of records from a tool result's structured content.
///
/// COROS wraps lists inconsistently across tools (`data`, `items`, `list`,
/// `records`, or the array itself). This is the one place that shape is
/// tolerated, so the mappers above can stay about *fields*.
///
/// # Errors
///
/// [`runalytics_provider_core::ProviderError::Malformed`] carrying a sample of
/// the unexpected shape, which is what turns a schema-drift bug report into a
/// ten-minute fix.
pub fn record_array(value: &Value, provider: &str) -> runalytics_provider_core::Result<Vec<Value>> {
    if let Some(arr) = value.as_array() {
        return Ok(arr.clone());
    }
    for key in ["data", "items", "list", "records", "activities", "results"] {
        if let Some(arr) = value.get(key).and_then(Value::as_array) {
            return Ok(arr.clone());
        }
    }
    // A single record where a list was expected is a normal API courtesy.
    if value.is_object() {
        return Ok(vec![value.clone()]);
    }
    let sample: String = value.to_string().chars().take(400).collect();
    Err(runalytics_provider_core::ProviderError::Malformed {
        provider: provider.into(),
        detail: "no record array under any known key".into(),
        sample: Some(sample),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tz() -> runalytics_core::Tz {
        "Europe/Berlin".parse().expect("tz")
    }

    #[test]
    fn duration_units_are_healed() {
        assert_eq!(duration_secs(3600.0).as_u32(), 3600);
        assert_eq!(duration_secs(3_600_000.0).as_u32(), 3600);
        assert_eq!(duration_secs(45.0).as_u32(), 45);
    }

    #[test]
    fn epoch_units_are_healed() {
        // 2026-10-08 in seconds and in milliseconds.
        assert_eq!(epoch_ts(1_791_500_000.0).map(|t| t.year()), Some(2026));
        assert_eq!(epoch_ts(1_791_500_000_000.0).map(|t| t.year()), Some(2026));
    }

    use chrono::Datelike as _;

    #[test]
    fn activity_maps_through_aliases_and_derives_pace() {
        let raw = json!({
            "activityId": "9911",
            "label": "Intervals",
            "startTime": 1_791_500_000_i64,
            "durationSec": "2700",
            "distance_km": 11.2,
            "avgHr": 151,
            "laps": [
                { "duration": 600, "distance": 2.0, "avgHr": 130 },
                { "duration": 1200, "distance": 6.0, "avgHr": 165, "avgPace": 240.0 },
            ]
        });
        let draft = map_activity(&raw, &tz()).expect("mapped");
        assert_eq!(draft.provider_activity_id, "9911");
        assert_eq!(draft.name, "Intervals");
        assert_eq!(draft.summary.duration.as_u32(), 2700);
        assert_eq!(draft.summary.avg_hr, Some(HeartRate::new(151)));
        // Pace derived from distance/duration when the provider omits it.
        let pace = draft.summary.avg_pace.expect("derived");
        assert!((pace.as_secs_per_km() - 2700.0 / 11.2).abs() < 0.01);
        assert_eq!(draft.laps.len(), 2);
        assert_eq!(draft.laps[1].avg_hr, Some(HeartRate::new(165)));
    }

    #[test]
    fn health_maps_sleep_and_dates_the_recovery_morning() {
        let raw = json!({
            "restingHeartRate": 51,
            "steps": 9_120,
            "sleep": {
                "sleepDuration": 28_800_000,
                "deepSleep": 6_000_000,
                "score": 86,
                "hrvValue": 52.5
            }
        });
        let draft = map_health(&raw, Date::from_ymd_opt(2026, 10, 8).expect("d")).expect("mapped");
        assert_eq!(draft.resting_hr, Some(HeartRate::new(51)));
        let sleep = draft.sleep.expect("sleep");
        assert_eq!(sleep.total.as_u32(), 28_800, "ms healed to seconds");
        assert_eq!(sleep.score, Some(86));
        assert_eq!(sleep.hrv, Some(52.5));
    }

    #[test]
    fn a_day_with_no_signal_is_skipped_not_zeroed() {
        assert!(map_health(&json!({ "waterMl": 500 }), Date::MIN).is_none());
    }

    #[test]
    fn record_array_tolerates_wrappers_and_reports_drift() {
        assert_eq!(record_array(&json!([1, 2]), "coros").expect("arr").len(), 2);
        assert_eq!(
            record_array(&json!({ "items": [1] }), "coros")
                .expect("arr")
                .len(),
            1
        );
        assert_eq!(
            record_array(&json!({ "id": 1 }), "coros")
                .expect("single")
                .len(),
            1
        );
        let err = record_array(&json!("nonsense"), "coros").expect_err("must fail");
        assert!(matches!(
            err,
            runalytics_provider_core::ProviderError::Malformed { .. }
        ));
    }

    #[test]
    fn a_late_evening_run_stays_on_its_own_day() {
        // 22:00 Berlin in October is UTC+2, i.e. 20:00 UTC the same day;
        // in winter it is 21:00 UTC. Either way the local day is 2026-10-08.
        use chrono::TimeZone as _;
        let utc = chrono::Utc.with_ymd_and_hms(2026, 10, 8, 20, 0, 0).unwrap();
        assert_eq!(
            local_date(utc, &tz()),
            Date::from_ymd_opt(2026, 10, 8).expect("d")
        );
        // 23:30 UTC is already the next morning in Berlin — the local day wins.
        let late = chrono::Utc
            .with_ymd_and_hms(2026, 10, 8, 23, 30, 0)
            .unwrap();
        assert_eq!(
            local_date(late, &tz()),
            Date::from_ymd_opt(2026, 10, 9).expect("d")
        );
    }

    #[test]
    fn fitness_maps_or_skips() {
        let mapped = map_fitness(
            &json!({ "vo2Max": 54.0, "runningLevel": 88, "thresholdPace": 281.0 }),
            Date::MIN,
        )
        .expect("mapped");
        assert_eq!(mapped.assessment.vo2max, Some(54.0));
        assert!(map_fitness(&json!({ "comment": "nice weather" }), Date::MIN).is_none());
    }
}
