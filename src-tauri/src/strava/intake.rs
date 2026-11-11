use chrono::{DateTime, FixedOffset, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use rusqlite::{params, OptionalExtension};

use crate::error::{AppError, Result};
use crate::store::Store;
use super::Client;

#[derive(Debug, Clone, Copy)]
pub(crate) enum IdentityDecision {
    Pair(i64),
    Separate,
}

#[derive(Deserialize)]
pub(crate) struct RemoteWorkout {
    id: i64,
    athlete: RemoteAthlete,
    sport_type: String,
    start_date: DateTime<Utc>,
    start_date_local: NaiveDateTime,
    utc_offset: i32,
    elapsed_time: f64,
    moving_time: f64,
    distance: f64,
    total_elevation_gain: Option<f64>,
    device_name: Option<String>,
}

#[derive(Deserialize)]
struct RemoteAthlete {
    id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TrainingWorkout {
    pub athlete_id: i64,
    pub remote_id: i64,
    pub sport: String,
    pub original_sport: String,
    pub start_time: DateTime<Utc>,
    pub local_start: NaiveDateTime,
    pub utc_offset_s: i32,
    pub elapsed_s: f64,
    pub moving_s: f64,
    pub distance_m: f64,
    pub elevation_gain_m: Option<f64>,
    pub device_name: Option<String>,
}

fn invalid() -> AppError {
    AppError::Other("Strava returned invalid workout evidence.".into())
}

fn nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

impl RemoteWorkout {
    pub(crate) fn training(self, expected_athlete: i64) -> Result<TrainingWorkout> {
        if self.id <= 0
            || expected_athlete <= 0
            || self.athlete.id != expected_athlete
            || self.sport_type.trim().is_empty()
            || !nonnegative(self.elapsed_time)
            || !nonnegative(self.moving_time)
            || self.moving_time > self.elapsed_time
            || !nonnegative(self.distance)
            || self.total_elevation_gain.is_some_and(|value| !nonnegative(value))
        {
            return Err(invalid());
        }
        let offset = FixedOffset::east_opt(self.utc_offset).ok_or_else(invalid)?;
        if self.start_date.with_timezone(&offset).naive_local() != self.start_date_local {
            return Err(invalid());
        }
        Ok(TrainingWorkout {
            athlete_id: expected_athlete,
            remote_id: self.id,
            original_sport: self.sport_type.clone(),
            sport: match self.sport_type.as_str() {
                "Run" | "TrailRun" | "VirtualRun" => "run".into(),
                _ => self.sport_type,
            },
            start_time: self.start_date,
            local_start: self.start_date_local,
            utc_offset_s: self.utc_offset,
            elapsed_s: self.elapsed_time,
            moving_s: self.moving_time,
            distance_m: self.distance,
            elevation_gain_m: self.total_elevation_gain,
            device_name: self.device_name,
        })
    }
}

impl TrainingWorkout {
    pub(crate) fn source_key(&self) -> String {
        format!("{}:{}", self.athlete_id, self.remote_id)
    }

    pub(crate) fn pace_s_km(&self) -> Option<f64> {
        (self.distance_m > 0.0 && self.moving_s > 0.0)
            .then(|| self.moving_s * 1000.0 / self.distance_m)
    }
}

pub(crate) const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS strava_intake_checkpoint (
    athlete_id INTEGER PRIMARY KEY CHECK(athlete_id > 0),
    upper_bound INTEGER NOT NULL,
    next_page INTEGER NOT NULL CHECK(next_page > 0),
    complete INTEGER NOT NULL CHECK(complete IN (0,1)),
    updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS strava_recent_checkpoint (
    athlete_id INTEGER PRIMARY KEY CHECK(athlete_id > 0),
    lower_bound INTEGER NOT NULL,
    upper_bound INTEGER NOT NULL CHECK(upper_bound > lower_bound),
    next_page INTEGER NOT NULL CHECK(next_page > 0),
    complete INTEGER NOT NULL CHECK(complete IN (0,1)),
    updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS strava_workout_evidence (
    activity_id INTEGER NOT NULL UNIQUE REFERENCES activities(id) ON DELETE CASCADE,
    athlete_id INTEGER NOT NULL CHECK(athlete_id > 0),
    remote_id INTEGER NOT NULL CHECK(remote_id > 0),
    elapsed_s REAL NOT NULL CHECK(elapsed_s >= 0),
    moving_s REAL NOT NULL CHECK(moving_s >= 0 AND moving_s <= elapsed_s),
    original_sport TEXT NOT NULL,
    local_start TEXT NOT NULL,
    utc_offset_s INTEGER NOT NULL,
    device_name TEXT,
    created_at INTEGER NOT NULL,
    fetched_at INTEGER NOT NULL,
    retention_expires_at INTEGER NOT NULL CHECK(retention_expires_at > fetched_at),
    PRIMARY KEY(athlete_id,remote_id)
);
CREATE TABLE IF NOT EXISTS activity_source_aliases (
    activity_id INTEGER NOT NULL REFERENCES activities(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    account_id TEXT NOT NULL,
    record_id TEXT NOT NULL,
    PRIMARY KEY(provider,account_id,record_id)
);
CREATE TABLE IF NOT EXISTS strava_identity_review (
    athlete_id INTEGER NOT NULL CHECK(athlete_id > 0),
    remote_id INTEGER NOT NULL CHECK(remote_id > 0),
    workout_json TEXT NOT NULL,
    candidate_ids_json TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    fetched_at INTEGER NOT NULL,
    retention_expires_at INTEGER NOT NULL CHECK(retention_expires_at > fetched_at),
    PRIMARY KEY(athlete_id,remote_id)
);
";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ImportPage {
    pub athlete_id: i64,
    pub upper_bound: i64,
    pub number: u32,
    pub complete: bool,
    pub fetched_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ImportCheckpoint {
    pub upper_bound: i64,
    pub next_page: u32,
    pub complete: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ImportOutcome {
    pub new_records: usize,
    pub pages: usize,
    pub complete: bool,
    pub retry_at: Option<i64>,
    pub failure_status: Option<u16>,
}

pub(super) async fn bootstrap(
    client: &mut Client<'_>,
    store: &Store,
    athlete_id: i64,
    now: i64,
) -> Result<ImportOutcome> {
    if athlete_id <= 0 || now <= 180 * 86400 {
        return Err(invalid());
    }
    let mut checkpoint = store.training_checkpoint(athlete_id)?.unwrap_or(ImportCheckpoint {
        upper_bound: now,
        next_page: 1,
        complete: false,
    });
    let mut outcome = ImportOutcome {
        new_records: 0,
        pages: 0,
        complete: checkpoint.complete,
        retry_at: None,
        failure_status: None,
    };
    let lower_bound = checkpoint.upper_bound.checked_sub(180 * 86400).ok_or_else(invalid)?;
    while !checkpoint.complete && outcome.pages < 10 {
        let endpoint = format!(
            "/athlete/activities?after={lower_bound}&before={}&per_page=100&page={}",
            checkpoint.upper_bound, checkpoint.next_page,
        );
        let remote: Vec<RemoteWorkout> = match client.request(reqwest::Method::GET, &endpoint, None).await {
            Ok(records) => records,
            Err(error) => {
                outcome.retry_at = Some(error.retry_at);
                outcome.failure_status = Some(error.status);
                return Ok(outcome);
            }
        };
        if remote.len() > 100 {
            return Err(invalid());
        }
        let complete = remote.len() < 100;
        let workouts = remote.into_iter().map(|record| record.training(athlete_id))
            .collect::<Result<Vec<_>>>()?;
        if workouts.iter().any(|workout| workout.start_time.timestamp() <= lower_bound) {
            return Err(invalid());
        }
        outcome.new_records += store.cache_training_page(ImportPage {
            athlete_id,
            upper_bound: checkpoint.upper_bound,
            number: checkpoint.next_page,
            complete,
            fetched_at: now,
        }, &workouts)?;
        outcome.pages += 1;
        checkpoint.next_page += 1;
        checkpoint.complete = complete;
        outcome.complete = complete;
    }
    Ok(outcome)
}

pub(super) async fn incremental(
    client: &mut Client<'_>,
    store: &Store,
    athlete_id: i64,
    now: i64,
) -> Result<ImportOutcome> {
    let bootstrap = store.training_checkpoint(athlete_id)?
        .filter(|checkpoint| checkpoint.complete)
        .ok_or_else(invalid)?;
    let recent = store.recent_training_checkpoint(athlete_id)?;
    let previous_upper = recent.as_ref().map_or(bootstrap.upper_bound, |(_, checkpoint)| checkpoint.upper_bound);
    let (lower_bound, mut checkpoint) = match recent {
        Some((lower, checkpoint)) if !checkpoint.complete => (lower, checkpoint),
        _ => (previous_upper.checked_sub(7 * 86400).ok_or_else(invalid)?, ImportCheckpoint {
            upper_bound: now,
            next_page: 1,
            complete: now <= previous_upper,
        }),
    };
    let mut outcome = ImportOutcome {
        new_records: 0, pages: 0, complete: checkpoint.complete,
        retry_at: None, failure_status: None,
    };
    if now < checkpoint.upper_bound || lower_bound >= checkpoint.upper_bound {
        return Err(invalid());
    }
    while !checkpoint.complete && outcome.pages < 10 {
        let endpoint = format!(
            "/athlete/activities?after={lower_bound}&before={}&per_page=100&page={}",
            checkpoint.upper_bound, checkpoint.next_page,
        );
        let remote: Vec<RemoteWorkout> = match client.request(reqwest::Method::GET, &endpoint, None).await {
            Ok(records) => records,
            Err(error) => {
                outcome.retry_at = Some(error.retry_at);
                outcome.failure_status = Some(error.status);
                return Ok(outcome);
            }
        };
        if remote.len() > 100 {
            return Err(invalid());
        }
        let complete = remote.len() < 100;
        let workouts = remote.into_iter().map(|record| record.training(athlete_id))
            .collect::<Result<Vec<_>>>()?;
        if workouts.iter().any(|workout| workout.start_time.timestamp() <= lower_bound) {
            return Err(invalid());
        }
        outcome.new_records += store.cache_training_window(ImportPage {
            athlete_id, upper_bound: checkpoint.upper_bound, number: checkpoint.next_page,
            complete, fetched_at: now,
        }, &workouts, Some(lower_bound))?;
        outcome.pages += 1;
        checkpoint.next_page += 1;
        checkpoint.complete = complete;
        outcome.complete = complete;
    }
    Ok(outcome)
}

impl Store {
    pub(crate) fn resolve_training_identity(&self, athlete_id: i64, remote_id: i64, decision: IdentityDecision, now: i64) -> Result<i64> {
        let mut connection = self.conn.lock().unwrap();
        let transaction = connection.transaction()?;
        let (workout_json, candidates_json, fetched_at, expires_at): (String,String,i64,i64) = transaction.query_row(
            "SELECT workout_json,candidate_ids_json,fetched_at,retention_expires_at FROM strava_identity_review
             WHERE athlete_id=?1 AND remote_id=?2", params![athlete_id,remote_id],
            |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?)),
        )?;
        if now < fetched_at || now >= expires_at {
            return Err(invalid());
        }
        let workout: TrainingWorkout = serde_json::from_str(&workout_json)?;
        let candidates: Vec<i64> = serde_json::from_str(&candidates_json)?;
        if workout.athlete_id != athlete_id || workout.remote_id != remote_id
            || matches!(decision, IdentityDecision::Pair(activity_id) if !candidates.contains(&activity_id)) {
            return Err(invalid());
        }
        Self::cache_training_workout(&transaction, &workout, fetched_at, Some(decision))?;
        let activity_id = transaction.query_row(
            "SELECT activity_id FROM strava_workout_evidence WHERE athlete_id=?1 AND remote_id=?2",
            params![athlete_id,remote_id], |row| row.get(0),
        )?;
        transaction.commit()?;
        Ok(activity_id)
    }

    fn recent_training_checkpoint(&self, athlete_id: i64) -> Result<Option<(i64, ImportCheckpoint)>> {
        Ok(self.conn.lock().unwrap().query_row(
            "SELECT lower_bound,upper_bound,next_page,complete FROM strava_recent_checkpoint WHERE athlete_id=?1",
            [athlete_id], |row| Ok((row.get(0)?, ImportCheckpoint {
                upper_bound: row.get(1)?, next_page: row.get(2)?, complete: row.get(3)?,
            })),
        ).optional()?)
    }

    pub(crate) fn training_checkpoint(&self, athlete_id: i64) -> Result<Option<ImportCheckpoint>> {
        Ok(self.conn.lock().unwrap().query_row(
            "SELECT upper_bound,next_page,complete FROM strava_intake_checkpoint WHERE athlete_id=?1",
            [athlete_id], |row| Ok(ImportCheckpoint {
                upper_bound: row.get(0)?, next_page: row.get(1)?, complete: row.get(2)?,
            }),
        ).optional()?)
    }

    pub(crate) fn cache_training_page(&self, page: ImportPage, workouts: &[TrainingWorkout]) -> Result<usize> {
        self.cache_training_window(page, workouts, None)
    }

    fn cache_training_window(&self, page: ImportPage, workouts: &[TrainingWorkout], recent_lower: Option<i64>) -> Result<usize> {
        if page.athlete_id <= 0 || page.number == 0 || page.fetched_at < page.upper_bound {
            return Err(invalid());
        }
        let mut connection = self.conn.lock().unwrap();
        let transaction = connection.transaction()?;
        transaction.execute(
            "DELETE FROM strava_identity_review WHERE retention_expires_at<=?1",
            [page.fetched_at],
        )?;
        let table = if recent_lower.is_some() { "strava_recent_checkpoint" } else { "strava_intake_checkpoint" };
        if let Some(lower_bound) = recent_lower {
            if lower_bound >= page.upper_bound {
                return Err(invalid());
            }
            transaction.execute(
                "DELETE FROM strava_recent_checkpoint WHERE athlete_id=?1 AND complete=1 AND upper_bound<?2 AND ?3=1",
                params![page.athlete_id, page.upper_bound, page.number],
            )?;
        }
        let checkpoint: Option<(i64, u32, bool)> = transaction.query_row(
            &format!("SELECT upper_bound,next_page,complete FROM {table} WHERE athlete_id=?1"),
            [page.athlete_id], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?)),
        ).optional()?;
        if let Some((upper_bound, next_page, complete)) = checkpoint {
            if upper_bound != page.upper_bound || page.number > next_page || complete && page.number >= next_page {
                return Err(invalid());
            }
        } else if page.number != 1 {
            return Err(invalid());
        }
        let mut new_records = 0;
        for workout in workouts {
            if workout.athlete_id != page.athlete_id || workout.start_time.timestamp() >= page.upper_bound
                || recent_lower.is_some_and(|lower| workout.start_time.timestamp() <= lower) {
                return Err(invalid());
            }
            new_records += Self::cache_training_workout(&transaction, workout, page.fetched_at, None)?;
        }
        if checkpoint.is_none_or(|(_, next_page, _)| page.number == next_page) {
            if let Some(lower_bound) = recent_lower {
                transaction.execute(
                    "INSERT INTO strava_recent_checkpoint(athlete_id,lower_bound,upper_bound,next_page,complete,updated_at)
                     VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(athlete_id) DO UPDATE SET next_page=excluded.next_page,
                        complete=excluded.complete,updated_at=excluded.updated_at",
                    params![page.athlete_id,lower_bound,page.upper_bound,i64::from(page.number)+1,page.complete,page.fetched_at],
                )?;
            } else {
                transaction.execute(
                    "INSERT INTO strava_intake_checkpoint(athlete_id,upper_bound,next_page,complete,updated_at)
                 VALUES(?1,?2,?3,?4,?5) ON CONFLICT(athlete_id) DO UPDATE SET next_page=excluded.next_page,
                    complete=excluded.complete,updated_at=excluded.updated_at",
                    params![page.athlete_id,page.upper_bound,i64::from(page.number)+1,page.complete,page.fetched_at],
                )?;
            }
        }
        transaction.commit()?;
        Ok(new_records)
    }

    fn cache_training_workout(transaction: &rusqlite::Transaction<'_>, workout: &TrainingWorkout, fetched_at: i64, decision: Option<IdentityDecision>) -> Result<usize> {
            let latest: Option<i64> = transaction.query_row(
                "SELECT MAX(fetched_at) FROM (
                    SELECT fetched_at FROM strava_workout_evidence WHERE athlete_id=?1 AND remote_id=?2
                    UNION ALL SELECT fetched_at FROM strava_identity_review WHERE athlete_id=?1 AND remote_id=?2
                 )",
                params![workout.athlete_id,workout.remote_id], |row| row.get(0),
            )?;
            if latest.is_some_and(|previous| previous > fetched_at) {
                return Ok(0);
            }
            let source_key = workout.source_key();
            let existing: Option<i64> = transaction.query_row(
                "SELECT id FROM activities WHERE provider='strava' AND provider_record_id=?1",
                [&source_key], |row| row.get(0),
            ).optional()?;
            let verified_legacy: Option<(i64, String, String, i64, f64, f64)> = if let Some(IdentityDecision::Pair(activity_id)) = decision {
                transaction.query_row(
                    "SELECT id,provider,provider_record_id,start_ts,duration_s,distance_m FROM activities
                     WHERE id=?1 AND provider<>'strava' AND lower(sport) IN ('run','running','trailrun','virtualrun')",
                    [activity_id],
                    |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?)),
                ).optional()?.ok_or_else(invalid).map(Some)?
            } else { transaction.query_row(
                "SELECT a.id,a.provider,a.provider_record_id,a.start_ts,a.duration_s,a.distance_m
                 FROM strava_exports e JOIN activities a ON e.source_key=a.provider || ':' || a.provider_record_id
                 WHERE e.athlete_id=?1 AND e.target_id=?2 AND e.status IN ('updated','already_present')",
                params![workout.athlete_id,workout.remote_id],
                |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?)),
            ).optional()? };
            if let Some((activity_id, provider, record_id, start, duration, distance)) = verified_legacy {
                let remote = super::RemoteActivity {
                    id: workout.remote_id, start_date: workout.start_time,
                    elapsed_time: workout.elapsed_s, moving_time: Some(workout.moving_s),
                    distance: workout.distance_m, sport_type: workout.original_sport.clone(),
                    description: None, athlete: Some(super::RemoteAthlete { id: workout.athlete_id }),
                };
                let bound_elsewhere: bool = transaction.query_row(
                    "SELECT EXISTS(SELECT 1 FROM strava_workout_evidence WHERE activity_id=?1 AND (athlete_id<>?2 OR remote_id<>?3))",
                    params![activity_id,workout.athlete_id,workout.remote_id], |row| row.get(0),
                )?;
                if existing.is_some_and(|existing_id| existing_id != activity_id)
                    || bound_elsewhere || !super::matches_activity(start, duration, distance, &remote) {
                    return Err(invalid());
                }
                transaction.execute(
                    "INSERT INTO activity_source_aliases(activity_id,provider,account_id,record_id) VALUES(?1,?2,'legacy',?3)
                     ON CONFLICT(provider,account_id,record_id) DO NOTHING",
                    params![activity_id,provider,record_id],
                )?;
                let alias_id: i64 = transaction.query_row(
                    "SELECT activity_id FROM activity_source_aliases WHERE provider=?1 AND account_id='legacy' AND record_id=?2",
                    params![provider,record_id], |row| row.get(0),
                )?;
                if alias_id != activity_id {
                    return Err(invalid());
                }
                transaction.execute(
                    "UPDATE activities SET provider='strava',provider_record_id=?2 WHERE id=?1",
                    params![activity_id,source_key],
                )?;
            }
            let existing: Option<i64> = transaction.query_row(
                "SELECT id FROM activities WHERE provider='strava' AND provider_record_id=?1",
                [&source_key], |row| row.get(0),
            ).optional()?;
            if existing.is_none() && decision.is_none() {
                let remote = super::RemoteActivity {
                    id: workout.remote_id, start_date: workout.start_time,
                    elapsed_time: workout.elapsed_s, moving_time: Some(workout.moving_s),
                    distance: workout.distance_m, sport_type: workout.original_sport.clone(),
                    description: None, athlete: None,
                };
                let mut statement = transaction.prepare(
                    "SELECT id,start_ts,duration_s,distance_m FROM activities
                     WHERE provider<>'strava' AND lower(sport) IN ('run','running','trailrun','virtualrun')
                     AND start_ts BETWEEN ?1-120 AND ?1+120 ORDER BY id",
                )?;
                let rows = statement.query_map([workout.start_time.timestamp()], |row| {
                    Ok((row.get::<_,i64>(0)?,row.get::<_,i64>(1)?,row.get::<_,f64>(2)?,row.get::<_,f64>(3)?))
                })?;
                let mut candidates = Vec::new();
                for candidate in rows {
                    let (activity_id,start,duration,distance) = candidate?;
                    if super::matches_activity(start,duration,distance,&remote) {
                        candidates.push(activity_id);
                    }
                }
                if !candidates.is_empty() {
                    transaction.execute(
                        "INSERT INTO strava_identity_review(athlete_id,remote_id,workout_json,candidate_ids_json,created_at,fetched_at,retention_expires_at)
                         VALUES(?1,?2,?3,?4,?5,?5,?6) ON CONFLICT(athlete_id,remote_id) DO UPDATE SET
                         workout_json=excluded.workout_json,candidate_ids_json=excluded.candidate_ids_json,
                         fetched_at=excluded.fetched_at,retention_expires_at=excluded.retention_expires_at",
                        params![workout.athlete_id,workout.remote_id,serde_json::to_string(workout)?,serde_json::to_string(&candidates)?,
                                fetched_at,fetched_at.checked_add(7 * 86400).ok_or_else(invalid)?],
                    )?;
                            return Ok(0);
                }
            }
            transaction.execute("DELETE FROM strava_identity_review WHERE athlete_id=?1 AND remote_id=?2", params![workout.athlete_id,workout.remote_id])?;
            transaction.execute(
                "INSERT INTO activities(provider,provider_record_id,sport,start_ts,duration_s,distance_m,avg_pace_s_km,elevation_gain_m)
                 VALUES('strava',?1,?2,?3,?4,?5,?6,?7)
                 ON CONFLICT(provider,provider_record_id) DO UPDATE SET sport=excluded.sport,start_ts=excluded.start_ts,
                    duration_s=excluded.duration_s,distance_m=excluded.distance_m,avg_pace_s_km=excluded.avg_pace_s_km,
                    elevation_gain_m=excluded.elevation_gain_m,avg_hr=NULL,max_hr=NULL",
                params![source_key,workout.sport,workout.start_time.timestamp(),workout.elapsed_s,
                    workout.distance_m,workout.pace_s_km(),workout.elevation_gain_m],
            )?;
            let activity_id = existing.unwrap_or_else(|| transaction.last_insert_rowid());
            transaction.execute(
                "INSERT INTO activity_source_aliases(activity_id,provider,account_id,record_id) VALUES(?1,'strava',?2,?3)
                 ON CONFLICT(provider,account_id,record_id) DO NOTHING",
                params![activity_id,workout.athlete_id.to_string(),workout.remote_id.to_string()],
            )?;
            let alias_id: i64 = transaction.query_row(
                "SELECT activity_id FROM activity_source_aliases WHERE provider='strava' AND account_id=?1 AND record_id=?2",
                params![workout.athlete_id.to_string(),workout.remote_id.to_string()], |row| row.get(0),
            )?;
            if alias_id != activity_id {
                return Err(invalid());
            }
            let expires_at = fetched_at.checked_add(7 * 86400).ok_or_else(invalid)?;
            transaction.execute(
                "INSERT INTO strava_workout_evidence(activity_id,athlete_id,remote_id,elapsed_s,moving_s,original_sport,
                    local_start,utc_offset_s,device_name,created_at,fetched_at,retention_expires_at)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?10,?11)
                 ON CONFLICT(athlete_id,remote_id) DO UPDATE SET elapsed_s=excluded.elapsed_s,moving_s=excluded.moving_s,
                    original_sport=excluded.original_sport,local_start=excluded.local_start,utc_offset_s=excluded.utc_offset_s,
                    device_name=excluded.device_name,fetched_at=excluded.fetched_at,retention_expires_at=excluded.retention_expires_at",
                params![activity_id,workout.athlete_id,workout.remote_id,workout.elapsed_s,workout.moving_s,
                    workout.original_sport,workout.local_start.to_string(),workout.utc_offset_s,workout.device_name,
                    fetched_at,expires_at],
            )?;
            Ok(usize::from(existing.is_none()))
    }
}

#[derive(Deserialize)]
pub(super) struct Stream<T> {
    data: Vec<T>,
}

#[derive(Deserialize)]
pub(super) struct RemoteStreams {
    time: Stream<i64>,
    distance: Stream<f64>,
    moving: Option<Stream<bool>>,
    latlng: Option<Stream<[f64; 2]>>,
    altitude: Option<Stream<f64>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TrainingPoint {
    pub elapsed_s: i64,
    pub distance_m: f64,
    pub moving: Option<bool>,
    pub position: Option<[f64; 2]>,
    pub altitude_m: Option<f64>,
}

impl RemoteStreams {
    pub(super) fn training(self, elapsed_s: f64, include_gps: bool) -> Result<Vec<TrainingPoint>> {
        let length = self.time.data.len();
        if !nonnegative(elapsed_s)
            || length == 0
            || self.distance.data.len() != length
            || self.moving.as_ref().is_some_and(|stream| stream.data.len() != length)
            || self.latlng.as_ref().is_some_and(|stream| stream.data.len() != length)
            || self.altitude.as_ref().is_some_and(|stream| stream.data.len() != length)
        {
            return Err(invalid());
        }
        let mut points = Vec::with_capacity(length);
        let mut previous_time = None;
        let mut previous_distance = 0.0;
        for (index, elapsed) in self.time.data.iter().copied().enumerate() {
            let distance = self.distance.data[index];
            if elapsed < 0
                || elapsed as f64 > elapsed_s
                || previous_time.is_some_and(|previous| elapsed <= previous)
                || !nonnegative(distance)
                || distance < previous_distance
            {
                return Err(invalid());
            }
            let position = if include_gps {
                self.latlng.as_ref().map(|stream| stream.data[index])
            } else {
                None
            };
            if position.is_some_and(|[latitude, longitude]| {
                !latitude.is_finite() || !longitude.is_finite()
                    || !(-90.0..=90.0).contains(&latitude)
                    || !(-180.0..=180.0).contains(&longitude)
            }) {
                return Err(invalid());
            }
            let altitude = self.altitude.as_ref().map(|stream| stream.data[index]);
            if altitude.is_some_and(|value| !value.is_finite()) {
                return Err(invalid());
            }
            points.push(TrainingPoint {
                elapsed_s: elapsed,
                distance_m: distance,
                moving: self.moving.as_ref().map(|stream| stream.data[index]),
                position,
                altitude_m: altitude,
            });
            previous_time = Some(elapsed);
            previous_distance = distance;
        }
        Ok(points)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn recording() -> Value {
        json!({
            "id": 1234, "athlete": { "id": 42 }, "sport_type": "Run",
            "start_date": "2026-11-01T06:30:00Z",
            "start_date_local": "2026-11-01T01:30:00", "utc_offset": -18000,
            "elapsed_time": 2100, "moving_time": 1800, "distance": 6000,
            "total_elevation_gain": 120, "device_name": "Garmin Forerunner",
            "average_heartrate": 150, "max_heartrate": 180,
            "calories": 400, "average_watts": 200, "average_cadence": 80
        })
    }

    fn workout(value: Value) -> Result<TrainingWorkout> {
        serde_json::from_value::<RemoteWorkout>(value).unwrap().training(42)
    }

    #[test]
    fn training_mapping_preserves_units_identity_and_local_date_without_health() {
        let result = workout(recording()).unwrap();
        assert_eq!(result.source_key(), "42:1234");
        assert_eq!(result.elapsed_s, 2100.0);
        assert_eq!(result.moving_s, 1800.0);
        assert_eq!(result.distance_m, 6000.0);
        assert_eq!(result.pace_s_km(), Some(300.0));
        assert_eq!(result.elevation_gain_m, Some(120.0));
        assert_eq!(result.local_start.to_string(), "2026-11-01 01:30:00");
        assert_eq!(result.device_name.as_deref(), Some("Garmin Forerunner"));
        let output = serde_json::to_value(result).unwrap();
        assert_eq!(output.as_object().unwrap().len(), 12);
        for field in ["average_heartrate", "max_heartrate", "calories", "average_watts", "average_cadence"] {
            assert!(output.get(field).is_none());
        }
    }

    #[test]
    fn rejects_wrong_owner_impossible_times_and_negative_measurements() {
        for (field, value) in [
            ("athlete", json!({"id": 99})), ("id", json!(0)),
            ("moving_time", json!(2101)), ("distance", json!(-1)),
            ("elapsed_time", json!(-1)), ("total_elevation_gain", json!(-1)),
            ("utc_offset", json!(999999)), ("sport_type", json!("")),
            ("start_date_local", json!("2026-11-01T02:30:00")),
        ] {
            let mut input = recording();
            input[field] = value;
            assert!(workout(input).is_err(), "accepted invalid {field}");
        }
    }

    #[test]
    fn sport_and_gpsless_workouts_do_not_invent_pace_or_coordinates() {
        for sport in ["Run", "TrailRun", "VirtualRun", "Ride", "WeightTraining"] {
            let mut input = recording();
            input["sport_type"] = json!(sport);
            input["distance"] = json!(0);
            let result = workout(input).unwrap();
            assert_eq!(result.sport, if ["Run", "TrailRun", "VirtualRun"].contains(&sport) { "run" } else { sport });
            assert_eq!(result.pace_s_km(), None);
        }
        let streams: RemoteStreams = serde_json::from_value(json!({
            "time": {"data": [0, 300]}, "distance": {"data": [0, 1000]}
        })).unwrap();
        let points = streams.training(300.0, true).unwrap();
        assert!(points.iter().all(|point| point.position.is_none() && point.moving.is_none()));
    }

    #[test]
    fn streams_preserve_pauses_and_trimmed_boundaries_and_obey_gps_consent() {
        let input = json!({
            "time": {"data": [30, 90, 150]}, "distance": {"data": [100, 100, 300]},
            "moving": {"data": [true, false, true]},
            "latlng": {"data": [[45, 9], [45, 9], [45.001, 9.001]]},
            "altitude": {"data": [10, 10, 12]}, "heartrate": {"data": [150, 140, 160]}
        });
        let points = serde_json::from_value::<RemoteStreams>(input.clone()).unwrap().training(180.0, false).unwrap();
        assert_eq!(points[0].elapsed_s, 30);
        assert_eq!(points[1].moving, Some(false));
        assert_eq!(points[1].distance_m, 100.0);
        assert!(points.iter().all(|point| point.position.is_none()));
        let points = serde_json::from_value::<RemoteStreams>(input).unwrap().training(180.0, true).unwrap();
        assert_eq!(points[2].position, Some([45.001, 9.001]));
        assert_eq!(points[2].altitude_m, Some(12.0));
        assert_eq!(serde_json::to_value(&points[0]).unwrap().as_object().unwrap().len(), 5);
    }

    #[test]
    fn invalid_streams_never_produce_partial_evidence() {
        for input in [
            json!({"time":{"data":[]}, "distance":{"data":[]}}),
            json!({"time":{"data":[0,1]}, "distance":{"data":[0]}}),
            json!({"time":{"data":[0,0]}, "distance":{"data":[0,1]}}),
            json!({"time":{"data":[0,301]}, "distance":{"data":[0,1]}}),
            json!({"time":{"data":[0,1]}, "distance":{"data":[2,1]}}),
            json!({"time":{"data":[0,1]}, "distance":{"data":[0,1]}, "latlng":{"data":[[91,0],[0,0]]}}),
            json!({"time":{"data":[0,1]}, "distance":{"data":[0,1]}, "moving":{"data":[true]}}),
        ] {
            let streams: RemoteStreams = serde_json::from_value(input).unwrap();
            assert!(streams.training(300.0, true).is_err());
        }
        let mut invalid_number = recording();
        invalid_number["distance"] = Value::Null;
        assert!(serde_json::from_value::<RemoteWorkout>(invalid_number).is_err());
    }

    fn page(number: u32) -> ImportPage {
        ImportPage { athlete_id: 42, upper_bound: 1800000000, number, complete: false, fetched_at: 1800000000 }
    }

    #[test]
    fn page_retries_preserve_ids_and_checkpoint_and_update_measurements() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        assert_eq!(store.cache_training_page(page(1), &[workout(recording()).unwrap()]).unwrap(), 1);
        let original = store.list_activities(10).unwrap()[0].clone();
        let mut changed = recording();
        changed["moving_time"] = json!(1500);
        assert_eq!(store.cache_training_page(page(1), &[workout(changed).unwrap()]).unwrap(), 0);
        let updated = store.list_activities(10).unwrap()[0].clone();
        assert_eq!(updated.id, original.id);
        assert_eq!(updated.provider, crate::types::ProviderKind::Strava);
        assert_eq!(updated.avg_pace_s_km, Some(250.0));
        assert_eq!(updated.duration_s, 2100.0);
        assert_eq!(updated.avg_hr, None);
        assert_eq!(updated.max_hr, None);
        let connection = store.conn.lock().unwrap();
        let (next, created, expires): (u32, i64, i64) = connection.query_row(
            "SELECT next_page,created_at,retention_expires_at FROM strava_intake_checkpoint JOIN strava_workout_evidence USING(athlete_id)",
            [], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?)),
        ).unwrap();
        assert_eq!(next, 2);
        assert_eq!(created, 1800000000);
        assert_eq!(expires, 1800604800);
    }

    #[test]
    fn older_page_replay_cannot_overwrite_newer_record_or_review() {
        let newer = ImportPage { fetched_at: page(1).fetched_at + 100, ..page(1) };
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let mut updated = recording();
        updated["moving_time"] = json!(1500);
        store.cache_training_page(newer, &[workout(updated).unwrap()]).unwrap();
        store.cache_training_page(page(1), &[workout(recording()).unwrap()]).unwrap();
        assert_eq!(store.list_activities(10).unwrap()[0].avg_pace_s_km, Some(250.0));
        let connection = store.conn.lock().unwrap();
        let times: (i64,i64) = connection.query_row(
            "SELECT fetched_at,retention_expires_at FROM strava_workout_evidence", [],
            |row| Ok((row.get(0)?,row.get(1)?)),
        ).unwrap();
        assert_eq!(times, (newer.fetched_at,newer.fetched_at+7*86400));
        drop(connection);

        let review_store = Store::open(std::path::Path::new(":memory:")).unwrap();
        seed_verified_recording(&review_store);
        review_store.conn.lock().unwrap().execute("DELETE FROM strava_exports", []).unwrap();
        let mut updated = recording();
        updated["moving_time"] = json!(1500);
        review_store.cache_training_page(newer, &[workout(updated).unwrap()]).unwrap();
        review_store.cache_training_page(page(1), &[workout(recording()).unwrap()]).unwrap();
        let connection = review_store.conn.lock().unwrap();
        let (payload,fetched_at): (String,i64) = connection.query_row(
            "SELECT workout_json,fetched_at FROM strava_identity_review", [],
            |row| Ok((row.get(0)?,row.get(1)?)),
        ).unwrap();
        let stored: TrainingWorkout = serde_json::from_str(&payload).unwrap();
        assert_eq!(stored.moving_s,1500.0);
        assert_eq!(fetched_at,newer.fetched_at);
    }

    #[test]
    fn failed_page_rolls_back_records_and_progress() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let first = workout(recording()).unwrap();
        let mut wrong_owner = workout(recording()).unwrap();
        wrong_owner.athlete_id = 99;
        assert!(store.cache_training_page(page(1), &[first, wrong_owner]).is_err());
        assert!(store.list_activities(10).unwrap().is_empty());
        assert_eq!(store.conn.lock().unwrap().query_row("SELECT count(*) FROM strava_intake_checkpoint", [], |row| row.get::<_,i64>(0)).unwrap(), 0);
        assert!(store.cache_training_page(page(2), &[]).is_err());
        store.cache_training_page(page(1), &[]).unwrap();
        assert!(store.cache_training_page(page(3), &[]).is_err());
    }

    #[test]
    fn source_identity_is_scoped_to_the_athlete_and_unknown_sources_fail() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        store.cache_training_page(page(1), &[workout(recording()).unwrap()]).unwrap();
        let mut input = recording();
        input["athlete"] = json!({"id":99});
        let other = serde_json::from_value::<RemoteWorkout>(input).unwrap().training(99).unwrap();
        store.cache_training_page(ImportPage { athlete_id:99, ..page(1) }, &[other]).unwrap();
        assert_eq!(store.list_activities(10).unwrap().len(), 2);
        assert!(crate::store::provider_from("unknown").is_err());
        assert_eq!(crate::store::provider_from("strava").unwrap(), crate::types::ProviderKind::Strava);
    }

    async fn fake_pages(responses: Vec<(u16, Value)>) -> (Client<'static>, tokio::task::JoinHandle<Vec<String>>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut requests = Vec::new();
            for (status, body) in responses {
                let (mut connection, _) = listener.accept().await.unwrap();
                let mut bytes = Vec::new();
                loop {
                    let mut buffer = [0; 1024];
                    let count = connection.read(&mut buffer).await.unwrap();
                    assert!(count > 0);
                    bytes.extend_from_slice(&buffer[..count]);
                    if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                requests.push(String::from_utf8(bytes).unwrap().lines().next().unwrap().to_string());
                let body = body.to_string();
                connection.write_all(format!(
                    "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len(),
                ).as_bytes()).await.unwrap();
            }
            requests
        });
        (Client {
            http: super::super::http().unwrap(), base_url: format!("http://{address}"),
            token: "synthetic".into(), remaining: 40, defer_until: 0, quota: None,
        }, server)
    }

    fn full_page(first_id: i64) -> Value {
        Value::Array((0..100).map(|index| {
            let mut input = recording();
            input["id"] = json!(first_id + index);
            input
        }).collect())
    }

    #[tokio::test]
    async fn bootstrap_resumes_a_failed_page_with_the_original_window() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let now = 1794000000;
        let (mut client, server) = fake_pages(vec![(200, full_page(1)), (503, json!({}))]).await;
        let first = bootstrap(&mut client, &store, 42, now).await.unwrap();
        assert_eq!((first.pages, first.new_records, first.complete, first.failure_status), (1, 100, false, Some(503)));
        assert!(first.retry_at.is_some());
        let requests = server.await.unwrap();
        assert!(requests[0].contains("page=1"));
        assert!(requests[1].contains("page=2"));
        assert_eq!(store.training_checkpoint(42).unwrap().unwrap().next_page, 2);

        let (mut client, server) = fake_pages(vec![(200, json!([recording()]))]).await;
        let resumed = bootstrap(&mut client, &store, 42, now + 600).await.unwrap();
        assert_eq!((resumed.pages, resumed.new_records, resumed.complete), (1, 1, true));
        let requests = server.await.unwrap();
        assert!(requests[0].contains(&format!("before={now}")));
        assert!(requests[0].contains("page=2"));
        assert_eq!(store.list_activities(200).unwrap().len(), 101);
        client.remaining = 0;
        let complete = bootstrap(&mut client, &store, 42, now + 900).await.unwrap();
        assert_eq!((complete.pages, complete.complete, complete.failure_status), (0, true, None));
    }

    #[tokio::test]
    async fn bootstrap_obeys_page_and_request_budgets_without_claiming_completion() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let (mut client, server) = fake_pages((0..10).map(|index| (200, full_page(index * 100 + 1))).collect()).await;
        let outcome = bootstrap(&mut client, &store, 42, 1794000000).await.unwrap();
        assert_eq!((outcome.pages, outcome.new_records, outcome.complete), (10, 1000, false));
        assert_eq!(server.await.unwrap().len(), 10);
        assert_eq!(client.remaining, 30);
        assert_eq!(store.training_checkpoint(42).unwrap().unwrap().next_page, 11);

        client.remaining = 0;
        let paused = bootstrap(&mut client, &store, 42, 1794000100).await.unwrap();
        assert_eq!((paused.pages, paused.complete, paused.failure_status), (0, false, Some(429)));
        assert_eq!(store.training_checkpoint(42).unwrap().unwrap().next_page, 11);
    }

    #[tokio::test]
    async fn bootstrap_rejects_a_whole_invalid_page_without_advancing() {
        for invalid_record in [
            { let mut input = recording(); input["athlete"] = json!({"id":99}); input },
            { let mut input = recording(); input["start_date"] = json!("2025-01-01T06:30:00Z"); input["start_date_local"] = json!("2025-01-01T01:30:00"); input },
        ] {
            let store = Store::open(std::path::Path::new(":memory:")).unwrap();
            let (mut client, server) = fake_pages(vec![(200, json!([recording(), invalid_record]))]).await;
            assert!(bootstrap(&mut client, &store, 42, 1794000000).await.is_err());
            assert_eq!(server.await.unwrap().len(), 1);
            assert!(store.training_checkpoint(42).unwrap().is_none());
            assert!(store.list_activities(10).unwrap().is_empty());
        }
    }

    #[tokio::test]
    async fn incremental_imports_new_and_delayed_runs_without_duplicating_history() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let now = 1794000000;
        store.cache_training_page(ImportPage { upper_bound: now, fetched_at: now, complete: true, ..page(1) }, &[]).unwrap();
        let (mut client, server) = fake_pages(vec![(200, json!([recording()]))]).await;
        let imported = incremental(&mut client, &store, 42, now + 86400).await.unwrap();
        assert_eq!((imported.new_records, imported.pages, imported.complete), (1, 1, true));
        let first_id = store.list_activities(10).unwrap()[0].id;
        let requests = server.await.unwrap();
        assert!(requests[0].contains(&format!("after={}", now - 7 * 86400)));

        let (mut client, server) = fake_pages(vec![(200, json!([recording()]))]).await;
        let repeated = incremental(&mut client, &store, 42, now + 2 * 86400).await.unwrap();
        assert_eq!((repeated.new_records, repeated.complete), (0, true));
        assert_eq!(store.list_activities(10).unwrap()[0].id, first_id);
        assert!(server.await.unwrap()[0].contains(&format!("after={}", now - 6 * 86400)));
        assert_eq!(store.training_checkpoint(42).unwrap().unwrap().upper_bound, now);
    }

    #[tokio::test]
    async fn incremental_resumes_fixed_window_and_covers_long_offline_gap() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let now = 1794000000;
        store.cache_training_page(ImportPage { upper_bound: now, fetched_at: now, complete: true, ..page(1) }, &[]).unwrap();
        let upper = now + 30 * 86400;
        let (mut client, server) = fake_pages(vec![(200, full_page(1)), (503, json!({}))]).await;
        let first = incremental(&mut client, &store, 42, upper).await.unwrap();
        assert_eq!((first.pages, first.complete, first.failure_status), (1, false, Some(503)));
        assert!(server.await.unwrap()[0].contains(&format!("after={}", now - 7 * 86400)));

        let (mut client, server) = fake_pages(vec![(200, json!([]))]).await;
        let resumed = incremental(&mut client, &store, 42, upper + 86400).await.unwrap();
        assert!(resumed.complete);
        let requests = server.await.unwrap();
        assert!(requests[0].contains(&format!("before={upper}")));
        assert!(requests[0].contains("page=2"));
        assert_eq!(store.recent_training_checkpoint(42).unwrap().unwrap().1.upper_bound, upper);
    }

    #[tokio::test]
    async fn incremental_requires_bootstrap_and_preserves_watermark_on_invalid_page() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let (mut client, server) = fake_pages(vec![]).await;
        assert!(incremental(&mut client, &store, 42, 1794000000).await.is_err());
        assert!(server.await.unwrap().is_empty());
        let now = 1794000000;
        store.cache_training_page(ImportPage { upper_bound: now, fetched_at: now, complete: true, ..page(1) }, &[]).unwrap();
        let mut wrong_owner = recording();
        wrong_owner["athlete"] = json!({"id":99});
        let (mut client, server) = fake_pages(vec![(200, json!([recording(), wrong_owner]))]).await;
        assert!(incremental(&mut client, &store, 42, now + 86400).await.is_err());
        assert_eq!(server.await.unwrap().len(), 1);
        assert!(store.recent_training_checkpoint(42).unwrap().is_none());
        assert!(store.list_activities(10).unwrap().is_empty());
        assert_eq!(store.training_checkpoint(42).unwrap().unwrap().upper_bound, now);
    }

    fn seed_verified_recording(store: &Store) -> i64 {
        let connection = store.conn.lock().unwrap();
        connection.execute(
            "INSERT INTO activities(provider,provider_record_id,sport,start_ts,duration_s,distance_m)
             VALUES('coros','watch-run','run',?1,2100,6000)",
            [workout(recording()).unwrap().start_time.timestamp()],
        ).unwrap();
        let activity_id = connection.last_insert_rowid();
        connection.execute("INSERT INTO strava_accounts(athlete_id,client_id,enabled) VALUES(42,'synthetic',0)", []).unwrap();
        connection.execute(
            "INSERT INTO strava_exports(athlete_id,source_key,workout_id,start_ts,duration_s,distance_m,payload,marker,target_id,status,created_at,updated_at)
             VALUES(42,'coros:watch-run','session',1,2100,6000,'','marker',1234,'updated',1,1)", [],
        ).unwrap();
        connection.execute(
            "INSERT INTO calendar_completions(workout_id,activity_id,original_notes) VALUES('session',?1,'preserved prescription')",
            [activity_id],
        ).unwrap();
        activity_id
    }

    #[test]
    fn verified_legacy_pair_preserves_local_id_and_completed_prescription() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let original_id = seed_verified_recording(&store);
        assert_eq!(store.cache_training_page(page(1), &[workout(recording()).unwrap()]).unwrap(), 0);
        let activities = store.list_activities(10).unwrap();
        assert_eq!(activities.len(), 1);
        assert_eq!(activities[0].id, original_id);
        assert_eq!(activities[0].provider, crate::types::ProviderKind::Strava);
        assert_eq!(activities[0].provider_record_id, "42:1234");
        let connection = store.conn.lock().unwrap();
        let saved: (i64,String) = connection.query_row(
            "SELECT activity_id,original_notes FROM calendar_completions WHERE workout_id='session'", [],
            |row| Ok((row.get(0)?,row.get(1)?)),
        ).unwrap();
        assert_eq!(saved, (original_id,"preserved prescription".into()));
        assert_eq!(connection.query_row(
            "SELECT activity_id FROM activity_source_aliases WHERE provider='coros' AND record_id='watch-run'", [],
            |row| row.get::<_,i64>(0),
        ).unwrap(), original_id);
    }

    #[test]
    fn mismatched_verified_pair_rolls_back_without_relabeling_history() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let original_id = seed_verified_recording(&store);
        let mut changed = recording();
        changed["distance"] = json!(10000);
        assert!(store.cache_training_page(page(1), &[workout(changed).unwrap()]).is_err());
        let activities = store.list_activities(10).unwrap();
        assert_eq!(activities.len(), 1);
        assert_eq!(activities[0].id, original_id);
        assert_eq!(activities[0].provider, crate::types::ProviderKind::Coros);
        assert!(store.training_checkpoint(42).unwrap().is_none());
    }

    #[test]
    fn adopted_identity_keeps_legacy_completion_lookup_and_writer_ledger() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let original_id = seed_verified_recording(&store);
        store.cache_training_page(page(1), &[workout(recording()).unwrap()]).unwrap();
        assert_eq!(store.activity_id_by_record("coros", "watch-run").unwrap(), Some(original_id));
        assert_eq!(store.activity_id_by_record("strava", "42:1234").unwrap(), Some(original_id));
        super::super::housekeeping(&store, 1800000000).unwrap();
        super::super::enqueue(&store, 42, 0, 1800000000).unwrap();
        let connection = store.conn.lock().unwrap();
        let ledger: (i64,String,i64) = connection.query_row(
            "SELECT count(*),source_key,target_id FROM strava_exports", [],
            |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?)),
        ).unwrap();
        assert_eq!(ledger, (1,"coros:watch-run".into(),1234));
    }

    #[test]
    fn imported_writer_uses_exact_target_and_never_queues_another_athlete() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        store.cache_training_page(page(1), &[workout(recording()).unwrap()]).unwrap();
        let activity_id = store.list_activities(10).unwrap()[0].id;
        {
            let connection = store.conn.lock().unwrap();
            connection.execute("INSERT INTO strava_accounts(athlete_id,client_id,enabled) VALUES(42,'synthetic',1),(99,'synthetic',1)", []).unwrap();
            connection.execute(
                "INSERT INTO calendar_completions(workout_id,activity_id,original_notes) VALUES('session',?1,'prescribed session')",
                [activity_id],
            ).unwrap();
        }
        super::super::enqueue(&store, 42, 0, 1800000000).unwrap();
        super::super::enqueue(&store, 99, 0, 1800000000).unwrap();
        let connection = store.conn.lock().unwrap();
        let ledger: (i64,i64,String,i64) = connection.query_row(
            "SELECT count(*),athlete_id,source_key,target_id FROM strava_exports", [],
            |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?)),
        ).unwrap();
        assert_eq!(ledger, (1,42,"strava:42:1234".into(),1234));
    }

    #[test]
    fn imported_writer_does_not_queue_stale_or_future_evidence() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        store.cache_training_page(page(1), &[workout(recording()).unwrap()]).unwrap();
        let activity_id = store.list_activities(10).unwrap()[0].id;
        {
            let connection = store.conn.lock().unwrap();
            connection.execute("INSERT INTO strava_accounts(athlete_id,client_id,enabled) VALUES(42,'synthetic',1)", []).unwrap();
            connection.execute(
                "INSERT INTO calendar_completions(workout_id,activity_id,original_notes) VALUES('session',?1,'prescribed session')",
                [activity_id],
            ).unwrap();
        }
        let fetched_at = page(1).fetched_at;
        for now in [fetched_at - 1, fetched_at + 7 * 86400] {
            super::super::enqueue(&store, 42, 0, now).unwrap();
            let count: i64 = store.conn.lock().unwrap().query_row("SELECT count(*) FROM strava_exports", [], |row| row.get(0)).unwrap();
            assert_eq!(count, 0);
        }
        super::super::enqueue(&store, 42, 0, fetched_at + 7 * 86400 - 1).unwrap();
        let target: i64 = store.conn.lock().unwrap().query_row("SELECT target_id FROM strava_exports", [], |row| row.get(0)).unwrap();
        assert_eq!(target, 1234);
    }

    #[test]
    fn unverified_duplicates_are_reviewed_without_double_counting_or_merging() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let original_id = seed_verified_recording(&store);
        {
            let connection = store.conn.lock().unwrap();
            connection.execute("DELETE FROM strava_exports", []).unwrap();
            connection.execute(
                "INSERT INTO activities(provider,provider_record_id,sport,start_ts,duration_s,distance_m)
                 SELECT 'garmin','similar-run',sport,start_ts+60,duration_s,distance_m FROM activities WHERE id=?1",
                [original_id],
            ).unwrap();
        }
        let imported = workout(recording()).unwrap();
        assert_eq!(store.cache_training_page(page(1), &[imported]).unwrap(), 0);
        assert_eq!(store.list_activities(10).unwrap().len(), 2);
        let connection = store.conn.lock().unwrap();
        let pending: (String,String) = connection.query_row(
            "SELECT workout_json,candidate_ids_json FROM strava_identity_review WHERE athlete_id=42 AND remote_id=1234",
            [], |row| Ok((row.get(0)?,row.get(1)?)),
        ).unwrap();
        let candidates: Vec<i64> = serde_json::from_str(&pending.1).unwrap();
        assert_eq!(candidates.len(), 2);
        assert!(candidates.contains(&original_id));
        assert!(!pending.0.contains("heartrate"));
        let completion: i64 = connection.query_row("SELECT activity_id FROM calendar_completions", [], |row| row.get(0)).unwrap();
        assert_eq!(completion, original_id);
        drop(connection);
        store.cache_training_page(page(1), &[workout(recording()).unwrap()]).unwrap();
        let count: i64 = store.conn.lock().unwrap().query_row("SELECT count(*) FROM strava_identity_review", [], |row| row.get(0)).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn nearby_nonrunning_and_out_of_tolerance_legacy_records_do_not_block_import() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let original_id = seed_verified_recording(&store);
        {
            let connection = store.conn.lock().unwrap();
            connection.execute("DELETE FROM strava_exports", []).unwrap();
            connection.execute("UPDATE activities SET sport='cycling' WHERE id=?1", [original_id]).unwrap();
            connection.execute(
                "INSERT INTO activities(provider,provider_record_id,sport,start_ts,duration_s,distance_m)
                 SELECT 'coros','too-far','run',start_ts+121,duration_s,distance_m FROM activities WHERE id=?1", [original_id],
            ).unwrap();
        }
        assert_eq!(store.cache_training_page(page(1), &[workout(recording()).unwrap()]).unwrap(), 1);
        assert_eq!(store.list_activities(10).unwrap().len(), 3);
    }

    #[test]
    fn confirmed_pair_retains_completion_and_expiry_and_rejects_stale_decisions() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let original_id = seed_verified_recording(&store);
        store.conn.lock().unwrap().execute("DELETE FROM strava_exports", []).unwrap();
        store.cache_training_page(page(1), &[workout(recording()).unwrap()]).unwrap();
        let now = page(1).fetched_at;
        assert!(store.resolve_training_identity(99,1234,IdentityDecision::Pair(original_id),now).is_err());
        assert!(store.resolve_training_identity(42,1234,IdentityDecision::Pair(original_id + 1),now).is_err());
        assert!(store.resolve_training_identity(42,1234,IdentityDecision::Pair(original_id),now+7*86400).is_err());
        assert_eq!(store.resolve_training_identity(42,1234,IdentityDecision::Pair(original_id),now+10).unwrap(), original_id);
        assert!(store.resolve_training_identity(42,1234,IdentityDecision::Pair(original_id),now+10).is_err());
        assert_eq!(store.list_activities(10).unwrap().len(), 1);
        assert_eq!(store.calendar_completions().unwrap()[0], ("session".into(),original_id,"preserved prescription".into()));
        let expiry: i64 = store.conn.lock().unwrap().query_row("SELECT retention_expires_at FROM strava_workout_evidence", [], |row| row.get(0)).unwrap();
        assert_eq!(expiry, now+7*86400);
    }

    #[test]
    fn explicit_separate_recording_is_idempotent_and_changed_pair_rolls_back() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let original_id = seed_verified_recording(&store);
        store.conn.lock().unwrap().execute("DELETE FROM strava_exports", []).unwrap();
        store.cache_training_page(page(1), &[workout(recording()).unwrap()]).unwrap();
        let now = page(1).fetched_at;
        store.conn.lock().unwrap().execute("UPDATE activities SET distance_m=10000 WHERE id=?1", [original_id]).unwrap();
        assert!(store.resolve_training_identity(42,1234,IdentityDecision::Pair(original_id),now).is_err());
        assert_eq!(store.list_activities(10).unwrap()[0].provider, crate::types::ProviderKind::Coros);
        let distinct_id = store.resolve_training_identity(42,1234,IdentityDecision::Separate,now).unwrap();
        assert_ne!(distinct_id, original_id);
        assert_eq!(store.cache_training_page(page(1), &[workout(recording()).unwrap()]).unwrap(), 0);
        assert_eq!(store.list_activities(10).unwrap().len(), 2);
        assert_eq!(store.calendar_completions().unwrap()[0].1, original_id);
    }

    #[test]
    fn a_later_import_prunes_expired_reviews_without_erasing_legacy_history() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let original_id = seed_verified_recording(&store);
        store.conn.lock().unwrap().execute("DELETE FROM strava_exports", []).unwrap();
        store.cache_training_page(page(1), &[workout(recording()).unwrap()]).unwrap();
        let expiry = page(1).fetched_at + 7 * 86400;
        store.cache_training_page(ImportPage { number:2,fetched_at:expiry,..page(1) }, &[]).unwrap();
        let connection = store.conn.lock().unwrap();
        let remaining: i64 = connection.query_row("SELECT count(*) FROM strava_identity_review", [], |row| row.get(0)).unwrap();
        assert_eq!(remaining,0);
        let completion: i64 = connection.query_row("SELECT activity_id FROM calendar_completions", [], |row| row.get(0)).unwrap();
        assert_eq!(completion,original_id);
    }
}