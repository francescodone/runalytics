//! Typed repositories over the schema.
//!
//! Every repository is a zero-sized namespace of associated functions taking
//! `&Db`. That shape keeps call sites readable (`PlanRepo::load(&db, id)?`),
//! avoids a lifetime on the object, and lets a repository participate in a
//! caller's transaction when one is already open.
//!
//! Two rules apply throughout:
//!
//! * **Writes that touch more than one row go through `Db::transaction`.** A
//!   plan without its sessions, or an activity without its laps, is a corrupt
//!   read for everything downstream.
//! * **Reads return domain types or nothing.** No partial row structs leak out
//!   of this module; the UI and the MCP server only ever see `runalytics-core`.

use rusqlite::{Connection, OptionalExtension, params};

use crate::db::Db;
use crate::error::{Result, StoreError};
use crate::models::{
    exists, now, parse_date, parse_time, parse_timestamp, read_bool, write_bool, write_date,
    write_json, write_optional_u16, write_time, write_timestamp,
};
use runalytics_core::{
    Activity, ActivityId, ActivityLap, ActivitySummary, Anchor, AnchorResolution, AthleteSnapshot,
    Date, GoalKind, HealthDay, HealthDayId, HeartRate, Intensity, Pace, Plan, PlanId, PlanStatus,
    PlanWeek, PlannedSession, PlannedSessionId, Provider, ProviderAccountId, SessionKind,
    StructuredWorkout, Timestamp, VolumeKm,
};

// ---------------------------------------------------------------------------
// provider accounts
// ---------------------------------------------------------------------------

/// A connected provider account as the app needs to see it.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderAccount {
    pub id: ProviderAccountId,
    pub provider: Provider,
    pub account_label: String,
    pub external_user_id: Option<String>,
    pub region: Option<String>,
    pub writable: bool,
    pub connected: bool,
    pub created_at: Timestamp,
    pub last_sync_at: Option<Timestamp>,
}

/// Resumable sync position for one data class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncCursor {
    pub data_class: String,
    pub date: Date,
}

/// Connected wearable accounts and their sync cursors.
pub struct ProviderAccountRepo;

impl ProviderAccountRepo {
    /// Insert or refresh an account, keyed on `(provider, account_label)`.
    ///
    /// The id is preserved on conflict so calendar and score rows referencing
    /// an account survive a reconnect.
    pub fn upsert(
        db: &Db,
        provider: Provider,
        account_label: &str,
        region: Option<&str>,
    ) -> Result<ProviderAccountId> {
        let conn = db.conn();
        let existing: Option<String> = conn
            .query_row(
                "SELECT id FROM provider_account WHERE provider = ?1 AND account_label = ?2",
                params![provider.as_str(), account_label],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(id) = existing {
            conn.execute(
                "UPDATE provider_account
                 SET region = COALESCE(?2, region), writable = ?3, connected = 1
                 WHERE id = ?1",
                params![id, region, write_bool(provider.supports_write())],
            )?;
            return id
                .parse()
                .map_err(|e: uuid::Error| StoreError::InvalidValue {
                    field: "account_id",
                    value: format!("{id} ({e})"),
                });
        }

        let id = ProviderAccountId::new();
        conn.execute(
            "INSERT INTO provider_account
                (id, provider, account_label, region, writable, connected, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6)",
            params![
                id.to_string(),
                provider.as_str(),
                account_label,
                region,
                write_bool(provider.supports_write()),
                write_timestamp(now())
            ],
        )?;
        Ok(id)
    }

    pub fn list(db: &Db) -> Result<Vec<ProviderAccount>> {
        let conn = db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, provider, account_label, external_user_id, region, writable,
                    connected, created_at, last_sync_at
             FROM provider_account ORDER BY provider",
        )?;
        let rows = stmt.query_map([], row_to_account)?;
        rows.map(|r| r.map_err(StoreError::from)).collect()
    }

    pub fn get(db: &Db, id: ProviderAccountId) -> Result<ProviderAccount> {
        let conn = db.conn();
        conn.query_row(
            "SELECT id, provider, account_label, external_user_id, region, writable,
                    connected, created_at, last_sync_at
             FROM provider_account WHERE id = ?1",
            params![id.to_string()],
            row_to_account,
        )
        .optional()?
        .ok_or_else(|| StoreError::NotFound {
            entity: "provider_account",
            id: id.to_string(),
        })
    }

    /// The first connected account for a provider, used when the app assumes a
    /// single device per provider.
    pub fn find_by_provider(db: &Db, provider: Provider) -> Result<Option<ProviderAccount>> {
        let conn = db.conn();
        conn.query_row(
            "SELECT id, provider, account_label, external_user_id, region, writable,
                    connected, created_at, last_sync_at
             FROM provider_account WHERE provider = ?1 AND connected = 1
             ORDER BY last_sync_at DESC LIMIT 1",
            params![provider.as_str()],
            row_to_account,
        )
        .optional()
        .map_err(StoreError::from)
    }

    pub fn mark_connected(db: &Db, id: ProviderAccountId, connected: bool) -> Result<()> {
        db.conn().execute(
            "UPDATE provider_account SET connected = ?2 WHERE id = ?1",
            params![id.to_string(), write_bool(connected)],
        )?;
        Ok(())
    }

    pub fn record_sync(db: &Db, id: ProviderAccountId, at: Timestamp) -> Result<()> {
        db.conn().execute(
            "UPDATE provider_account SET last_sync_at = ?2 WHERE id = ?1",
            params![id.to_string(), write_timestamp(at)],
        )?;
        Ok(())
    }

    /// Read the sync cursor for a data class.
    ///
    /// Callers subtract a small overlap window before pulling: a day's health
    /// data is finalised hours after midnight, so resuming at exactly the last
    /// cursor date would permanently miss late-arriving sleep records.
    pub fn cursor(db: &Db, id: ProviderAccountId, data_class: &str) -> Result<Option<Date>> {
        let conn = db.conn();
        let text: Option<String> = conn
            .query_row(
                "SELECT cursor_date FROM sync_cursor WHERE account_id = ?1 AND data_class = ?2",
                params![id.to_string(), data_class],
                |row| row.get(0),
            )
            .optional()?;
        text.as_deref().map(parse_date).transpose()
    }

    pub fn set_cursor(db: &Db, id: ProviderAccountId, data_class: &str, date: Date) -> Result<()> {
        db.conn().execute(
            "INSERT INTO sync_cursor (account_id, data_class, cursor_date, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(account_id, data_class)
             DO UPDATE SET cursor_date = excluded.cursor_date, updated_at = excluded.updated_at",
            params![
                id.to_string(),
                data_class,
                write_date(date),
                write_timestamp(now())
            ],
        )?;
        Ok(())
    }

    pub fn cursors(db: &Db, id: ProviderAccountId) -> Result<Vec<SyncCursor>> {
        let conn = db.conn();
        let mut stmt = conn.prepare(
            "SELECT data_class, cursor_date FROM sync_cursor WHERE account_id = ?1
             ORDER BY data_class",
        )?;
        let rows = stmt.query_map(params![id.to_string()], |row| {
            let class: String = row.get(0)?;
            let date: String = row.get(1)?;
            Ok((class, date))
        })?;
        rows.map(|r| {
            let (class, date) = r?;
            Ok(SyncCursor {
                data_class: class,
                date: parse_date(&date)?,
            })
        })
        .collect()
    }
}

fn row_to_account(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProviderAccount> {
    let id_text: String = row.get("id")?;
    let provider_text: String = row.get("provider")?;
    let created: String = row.get("created_at")?;
    let last_sync: Option<String> = row.get("last_sync_at")?;
    // Errors inside `query_map` must be `rusqlite::Error`, so domain parse
    // failures are carried as a custom error and unwrapped by the caller.
    Ok(ProviderAccount {
        id: id_text.parse().unwrap_or_default(),
        provider: provider_text.parse().unwrap_or(Provider::Coros),
        account_label: row.get("account_label")?,
        external_user_id: row.get("external_user_id")?,
        region: row.get("region")?,
        writable: read_bool(row, "writable"),
        connected: read_bool(row, "connected"),
        created_at: parse_timestamp_loose(&created),
        last_sync_at: last_sync.as_deref().map(parse_timestamp_loose),
    })
}

/// Parse for display paths, where a malformed timestamp must not blank a whole
/// list. Repository write paths always use `parse_timestamp` and do fail loudly.
fn parse_timestamp_loose(text: &str) -> Timestamp {
    parse_timestamp(text).unwrap_or_else(|_| Timestamp::default())
}

// ---------------------------------------------------------------------------
// activities
// ---------------------------------------------------------------------------

/// Imported activities and their laps.
pub struct ActivityRepo;

impl ActivityRepo {
    /// Insert-or-replace an activity together with its laps.
    ///
    /// Returns `true` when the row was newly created, which the sync path
    /// reports as "new records" so the user sees progress on a first sync.
    pub fn upsert(db: &Db, activity: &Activity) -> Result<bool> {
        db.transaction(|tx| {
            // `changes` cannot answer "is this new?": an ON CONFLICT DO UPDATE
            // that changes a value also reports 1. Ask the table directly.
            let existed: bool = tx
                .query_row(
                    "SELECT 1 FROM activity WHERE account_id = ?1 AND provider_activity_id = ?2",
                    params![activity.account.to_string(), activity.provider_activity_id],
                    |_| Ok(()),
                )
                .optional()?
                .is_some();

            tx.execute(
                "INSERT INTO activity (
                    id, account_id, provider_activity_id, name, started_at, local_date,
                    distance_km, duration_s, avg_pace, avg_hr, max_hr, elevation_gain,
                    avg_cadence, training_load, intensity, matched_session_id, fetched_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)
                 ON CONFLICT(account_id, provider_activity_id) DO UPDATE SET
                    name = excluded.name,
                    started_at = excluded.started_at,
                    local_date = excluded.local_date,
                    distance_km = excluded.distance_km,
                    duration_s = excluded.duration_s,
                    avg_pace = excluded.avg_pace,
                    avg_hr = excluded.avg_hr,
                    max_hr = excluded.max_hr,
                    elevation_gain = excluded.elevation_gain,
                    avg_cadence = excluded.avg_cadence,
                    training_load = excluded.training_load,
                    intensity = excluded.intensity,
                    fetched_at = excluded.fetched_at",
                params![
                    activity.id.to_string(),
                    activity.account.to_string(),
                    activity.provider_activity_id,
                    activity.name,
                    write_timestamp(activity.started_at),
                    write_date(activity.local_date),
                    activity.summary.distance.as_f64(),
                    i64::from(activity.summary.duration.as_u32()),
                    activity
                        .summary
                        .avg_pace
                        .map(runalytics_core::Pace::as_secs_per_km),
                    write_optional_u16(activity.summary.avg_hr.map(HeartRate::as_u16)),
                    write_optional_u16(activity.summary.max_hr.map(HeartRate::as_u16)),
                    activity.summary.elevation_gain,
                    activity.summary.avg_cadence.map(i64::from),
                    activity.summary.training_load,
                    activity.intensity.as_str(),
                    activity.matched_session.map(|s| s.to_string()),
                    write_timestamp(activity.fetched_at)
                ],
            )?;

            // Laps are replaced wholesale: providers never append a lap to an
            // activity they have already finished reporting.
            tx.execute(
                "DELETE FROM activity_lap WHERE activity_id = ?1",
                params![activity.id.to_string()],
            )?;
            for lap in &activity.laps {
                tx.execute(
                    "INSERT INTO activity_lap
                        (activity_id, idx, start_at, duration_s, distance_km, avg_pace,
                         avg_hr, max_hr, elevation_gain, cadence)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                    params![
                        activity.id.to_string(),
                        i64::from(lap.index),
                        write_timestamp(lap.start),
                        i64::from(lap.duration.as_u32()),
                        lap.distance.as_f64(),
                        lap.avg_pace.map(runalytics_core::Pace::as_secs_per_km),
                        write_optional_u16(lap.avg_hr.map(HeartRate::as_u16)),
                        write_optional_u16(lap.max_hr.map(HeartRate::as_u16)),
                        lap.elevation_gain,
                        lap.cadence.map(i64::from)
                    ],
                )?;
            }
            Ok(!existed)
        })
    }

    pub fn get(db: &Db, id: ActivityId) -> Result<Activity> {
        let conn = db.conn();
        let mut activity = conn
            .query_row(
                "SELECT * FROM activity WHERE id = ?1",
                params![id.to_string()],
                row_to_activity,
            )
            .optional()?
            .ok_or_else(|| StoreError::NotFound {
                entity: "activity",
                id: id.to_string(),
            })?;
        activity.laps = load_laps(&conn, id)?;
        Ok(activity)
    }

    /// Activities in an inclusive local-date range, oldest first.
    pub fn list_range(db: &Db, from: Date, to: Date) -> Result<Vec<Activity>> {
        // Collect the ids under the lock, then release it: `Self::get` takes the
        // same lock, and the connection guard is not reentrant.
        let ids: Vec<String> = {
            let conn = db.conn();
            let mut stmt = conn.prepare(
                "SELECT id FROM activity WHERE local_date BETWEEN ?1 AND ?2
                 ORDER BY started_at",
            )?;
            let rows = stmt.query_map(params![write_date(from), write_date(to)], |row| {
                row.get::<_, String>("id")
            })?;
            rows.map(|r| r.map_err(StoreError::from))
                .collect::<Result<_>>()?
        };

        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            let parsed: ActivityId =
                id.parse()
                    .map_err(|e: uuid::Error| StoreError::InvalidValue {
                        field: "activity_id",
                        value: format!("{id} ({e})"),
                    })?;
            out.push(Self::get(db, parsed)?);
        }
        Ok(out)
    }

    pub fn latest_date(db: &Db) -> Result<Option<Date>> {
        let conn = db.conn();
        let text: Option<String> = conn
            .query_row("SELECT MAX(local_date) FROM activity", [], |row| row.get(0))
            .optional()?
            .flatten();
        text.as_deref().map(parse_date).transpose()
    }

    /// Attach an activity to the planned session it satisfied.
    pub fn match_session(
        db: &Db,
        activity_id: ActivityId,
        session_id: Option<PlannedSessionId>,
    ) -> Result<()> {
        db.conn().execute(
            "UPDATE activity SET matched_session_id = ?2 WHERE id = ?1",
            params![activity_id.to_string(), session_id.map(|s| s.to_string())],
        )?;
        Ok(())
    }

    /// Activities on a day that are not yet attributed to a session, used by
    /// the auto-match pass in `runalytics-scoring`.
    pub fn unmatched_on(db: &Db, date: Date) -> Result<Vec<Activity>> {
        // Same lock discipline as `list_range`: read ids, unlock, then load.
        let ids: Vec<String> = {
            let conn = db.conn();
            let mut stmt = conn.prepare(
                "SELECT id FROM activity WHERE local_date = ?1 AND matched_session_id IS NULL
                 ORDER BY started_at",
            )?;
            let rows = stmt.query_map(params![write_date(date)], |row| row.get(0))?;
            rows.map(|r| r.map_err(StoreError::from))
                .collect::<Result<_>>()?
        };
        ids.iter()
            .map(|id| {
                let parsed: ActivityId =
                    id.parse()
                        .map_err(|e: uuid::Error| StoreError::InvalidValue {
                            field: "activity_id",
                            value: format!("{id} ({e})"),
                        })?;
                Self::get(db, parsed)
            })
            .collect()
    }
}

fn load_laps(conn: &Connection, activity_id: ActivityId) -> Result<Vec<ActivityLap>> {
    let mut stmt = conn.prepare(
        "SELECT idx, start_at, duration_s, distance_km, avg_pace, avg_hr, max_hr,
                elevation_gain, cadence
         FROM activity_lap WHERE activity_id = ?1 ORDER BY idx",
    )?;
    let rows = stmt.query_map(params![activity_id.to_string()], |row| {
        let idx: i64 = row.get("idx")?;
        let start: String = row.get("start_at")?;
        let duration: i64 = row.get("duration_s")?;
        let distance: f64 = row.get("distance_km")?;
        let avg_pace: Option<f64> = row.get("avg_pace")?;
        Ok(ActivityLap {
            index: idx as u32,
            start: parse_timestamp_loose(&start),
            duration: runalytics_core::DurationSecs(duration as u32),
            distance: VolumeKm(distance),
            avg_pace: avg_pace.map(Pace::new),
            avg_hr: crate::models::read_optional_u16(row, "avg_hr").map(HeartRate::new),
            max_hr: crate::models::read_optional_u16(row, "max_hr").map(HeartRate::new),
            elevation_gain: row.get("elevation_gain")?,
            cadence: crate::models::read_optional_u16(row, "cadence"),
        })
    })?;
    rows.map(|r| r.map_err(StoreError::from)).collect()
}

fn row_to_activity(row: &rusqlite::Row<'_>) -> rusqlite::Result<Activity> {
    let id: String = row.get("id")?;
    let account: String = row.get("account_id")?;
    let started: String = row.get("started_at")?;
    let fetched: String = row.get("fetched_at")?;
    let local_date: String = row.get("local_date")?;
    let intensity: String = row.get("intensity")?;
    let matched: Option<String> = row.get("matched_session_id")?;
    let avg_pace: Option<f64> = row.get("avg_pace")?;

    Ok(Activity {
        id: id.parse().unwrap_or_default(),
        account: account.parse().unwrap_or_default(),
        provider_activity_id: row.get("provider_activity_id")?,
        name: row.get("name")?,
        started_at: parse_timestamp_loose(&started),
        local_date: parse_date(&local_date).unwrap_or(Date::MIN),
        summary: ActivitySummary {
            distance: VolumeKm(row.get("distance_km")?),
            duration: runalytics_core::DurationSecs(row.get::<_, i64>("duration_s")? as u32),
            avg_pace: avg_pace.map(Pace::new),
            avg_hr: crate::models::read_optional_u16(row, "avg_hr").map(HeartRate::new),
            max_hr: crate::models::read_optional_u16(row, "max_hr").map(HeartRate::new),
            elevation_gain: row.get("elevation_gain")?,
            avg_cadence: crate::models::read_optional_u16(row, "avg_cadence"),
            training_load: row.get("training_load")?,
        },
        laps: Vec::new(),
        intensity: intensity.parse().unwrap_or(Intensity::Unknown),
        matched_session: matched.and_then(|s| s.parse().ok()),
        fetched_at: parse_timestamp_loose(&fetched),
    })
}

// ---------------------------------------------------------------------------
// health
// ---------------------------------------------------------------------------

/// Daily health and sleep records.
pub struct HealthRepo;

impl HealthRepo {
    pub fn upsert(db: &Db, day: &HealthDay) -> Result<()> {
        let sleep_json = match &day.sleep {
            None => None,
            Some(sleep) => Some(write_json(sleep)?),
        };
        db.conn().execute(
            "INSERT INTO health_day
                (id, account_id, date, resting_hr, avg_stress, high_stress_min, steps,
                 provider_readiness, basal_energy, sleep_json, has_recovery, fetched_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
             ON CONFLICT(account_id, date) DO UPDATE SET
                resting_hr = excluded.resting_hr,
                avg_stress = excluded.avg_stress,
                high_stress_min = excluded.high_stress_min,
                steps = excluded.steps,
                provider_readiness = excluded.provider_readiness,
                basal_energy = excluded.basal_energy,
                sleep_json = excluded.sleep_json,
                has_recovery = excluded.has_recovery,
                fetched_at = excluded.fetched_at",
            params![
                day.id.to_string(),
                day.account.to_string(),
                write_date(day.date),
                write_optional_u16(day.resting_hr.map(HeartRate::as_u16)),
                day.avg_stress,
                day.high_stress_minutes.map(i64::from),
                day.steps.map(i64::from),
                day.provider_readiness.map(i64::from),
                day.basal_energy,
                sleep_json,
                write_bool(day.has_recovery_signal()),
                write_timestamp(now())
            ],
        )?;
        Ok(())
    }

    pub fn get(db: &Db, date: Date) -> Result<Option<HealthDay>> {
        let conn = db.conn();
        conn.query_row(
            "SELECT * FROM health_day WHERE date = ?1 ORDER BY fetched_at DESC LIMIT 1",
            params![write_date(date)],
            row_to_health,
        )
        .optional()
        .map_err(StoreError::from)
    }

    pub fn list_range(db: &Db, from: Date, to: Date) -> Result<Vec<HealthDay>> {
        let conn = db.conn();
        let mut stmt =
            conn.prepare("SELECT * FROM health_day WHERE date BETWEEN ?1 AND ?2 ORDER BY date")?;
        let rows = stmt.query_map(params![write_date(from), write_date(to)], row_to_health)?;
        rows.map(|r| r.map_err(StoreError::from)).collect()
    }

    pub fn latest_date(db: &Db) -> Result<Option<Date>> {
        let conn = db.conn();
        let text: Option<String> = conn
            .query_row("SELECT MAX(date) FROM health_day", [], |row| row.get(0))
            .optional()?
            .flatten();
        text.as_deref().map(parse_date).transpose()
    }
}

fn row_to_health(row: &rusqlite::Row<'_>) -> rusqlite::Result<HealthDay> {
    let date: String = row.get("date")?;
    let id: String = row.get("id")?;
    let account: String = row.get("account_id")?;
    let sleep_json: Option<String> = row.get("sleep_json")?;
    Ok(HealthDay {
        id: id.parse().unwrap_or_default(),
        account: account.parse().unwrap_or_default(),
        date: parse_date(&date).unwrap_or(Date::MIN),
        resting_hr: crate::models::read_optional_u16(row, "resting_hr").map(HeartRate::new),
        avg_stress: row.get("avg_stress")?,
        high_stress_minutes: row
            .get::<_, Option<i64>>("high_stress_min")?
            .map(|v| v as u32),
        steps: row.get::<_, Option<i64>>("steps")?.map(|v| v as u32),
        provider_readiness: row
            .get::<_, Option<i64>>("provider_readiness")?
            .map(|v| v as u8),
        basal_energy: row.get("basal_energy")?,
        sleep: sleep_json.and_then(|j| serde_json::from_str(&j).ok()),
    })
}

// ---------------------------------------------------------------------------
// plans
// ---------------------------------------------------------------------------

/// Plans with their weeks and sessions.
pub struct PlanRepo;

impl PlanRepo {
    /// Persist a plan and everything under it, atomically.
    ///
    /// Weeks and sessions are deleted and re-inserted rather than merged: a
    /// re-plan can change week boundaries, and merging would leave orphaned
    /// sessions pointing at a week index that no longer means the same thing.
    pub fn save(db: &Db, plan: &Plan) -> Result<()> {
        db.transaction(|tx| {
            tx.execute(
                "INSERT INTO plan
                    (id, name, goal, status, anchor_json, resolution_json, athlete_json,
                     phases_json, ceiling_volume, emittable, external_json,
                     created_at, updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?12)
                 ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    goal = excluded.goal,
                    status = excluded.status,
                    anchor_json = excluded.anchor_json,
                    resolution_json = excluded.resolution_json,
                    athlete_json = excluded.athlete_json,
                    phases_json = excluded.phases_json,
                    ceiling_volume = excluded.ceiling_volume,
                    emittable = excluded.emittable,
                    external_json = excluded.external_json,
                    updated_at = excluded.updated_at",
                params![
                    plan.id.to_string(),
                    plan.name,
                    plan.goal.as_str(),
                    plan.status.as_str(),
                    write_json(&plan.anchor)?,
                    write_json(&plan.resolution)?,
                    write_json(&plan.athlete)?,
                    write_json(&plan.phases)?,
                    plan.ceiling_volume.as_f64(),
                    write_bool(plan.emittable_as_coros_plan),
                    write_json(&plan.external_ids)?,
                    write_timestamp(plan.created_at),
                ],
            )?;

            tx.execute(
                "DELETE FROM plan_week WHERE plan_id = ?1",
                params![plan.id.to_string()],
            )?;
            tx.execute(
                "DELETE FROM planned_session WHERE plan_id = ?1",
                params![plan.id.to_string()],
            )?;

            for week in &plan.weeks {
                tx.execute(
                    "INSERT INTO plan_week
                        (plan_id, idx, phase, start_date, end_date, target_volume,
                         previous_volume, step_pct, projected_acwr, is_deload)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                    params![
                        plan.id.to_string(),
                        i64::from(week.index),
                        week.phase.as_str(),
                        write_date(week.start),
                        write_date(week.end),
                        week.target_volume.as_f64(),
                        week.previous_volume.as_f64(),
                        week.step_pct,
                        week.projected_acwr,
                        write_bool(week.is_deload)
                    ],
                )?;
            }

            for week in &plan.weeks {
                for session in &week.sessions {
                    tx.execute(
                        "INSERT INTO planned_session
                            (id, plan_id, week_idx, date, start_time, kind, title, intent,
                             workout_json, target_volume, target_duration, target_pace,
                             rpe_target, quality, external_id)
                         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
                        params![
                            session.id.to_string(),
                            plan.id.to_string(),
                            i64::from(week.index),
                            write_date(session.date),
                            write_time(session.start),
                            session.kind.as_str(),
                            session.title,
                            session.intent,
                            write_json(&session.workout)?,
                            session.target_volume.as_f64(),
                            i64::from(session.target_duration.as_u32()),
                            session
                                .target_pace
                                .map(runalytics_core::Pace::as_secs_per_km),
                            session.rpe_target.map(i64::from),
                            write_bool(session.quality),
                            session.external_id
                        ],
                    )?;
                }
            }
            Ok(())
        })
    }

    pub fn load(db: &Db, id: PlanId) -> Result<Plan> {
        let conn = db.conn();

        // The plan header and its weeks come from one LEFT JOIN: a plan with no
        // weeks still produces a row, with NULL week columns.
        let mut plan = {
            let mut stmt = conn.prepare(
                "SELECT p.id, p.name, p.goal, p.status, p.anchor_json, p.resolution_json,
                        p.athlete_json, p.phases_json, p.ceiling_volume, p.emittable,
                        p.external_json, p.created_at,
                        w.idx, w.phase, w.start_date, w.end_date, w.target_volume,
                        w.previous_volume, w.step_pct, w.projected_acwr, w.is_deload
                 FROM plan p
                 LEFT JOIN plan_week w ON w.plan_id = p.id
                 WHERE p.id = ?1
                 ORDER BY w.idx",
            )?;
            let rows = stmt.query_map(params![id.to_string()], row_to_plan_week)?;
            let mut plan: Option<Plan> = None;
            for row in rows {
                let (row_plan, week) = row?;
                // The header is identical on every row of the join; take it once.
                let plan = plan.get_or_insert(row_plan);
                if let Some(week) = week {
                    plan.weeks.push(week);
                }
            }
            plan.ok_or_else(|| StoreError::NotFound {
                entity: "plan",
                id: id.to_string(),
            })?
        };

        // Sessions come from their own query: joining them into the plan query
        // would multiply the plan columns by the session count.
        let sessions: Vec<(i64, PlannedSession)> = {
            let mut stmt = conn.prepare(
                "SELECT id, week_idx, date, start_time, kind, title, intent, workout_json,
                        target_volume, target_duration, target_pace, rpe_target, quality,
                        external_id
                 FROM planned_session WHERE plan_id = ?1 ORDER BY date",
            )?;
            let rows = stmt.query_map(params![id.to_string()], |row| {
                let week_idx: i64 = row.get(1)?;
                let session = row_to_session(row)?;
                Ok((week_idx, session))
            })?;
            rows.map(|r| r.map_err(StoreError::from))
                .collect::<Result<Vec<_>>>()?
        };

        for week in &mut plan.weeks {
            let idx = i64::from(week.index);
            week.sessions = sessions
                .iter()
                .filter(|(week_idx, _)| *week_idx == idx)
                .map(|(_, session)| session.clone())
                .collect();
        }
        Ok(plan)
    }

    pub fn list(db: &Db) -> Result<Vec<PlanSummary>> {
        let conn = db.conn();
        let mut stmt = conn.prepare(
            "SELECT p.id, p.name, p.goal, p.status, p.resolution_json, p.created_at,
                    (SELECT COUNT(*) FROM planned_session s WHERE s.plan_id = p.id) AS sessions
             FROM plan p ORDER BY p.created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let goal: String = row.get(2)?;
            let status: String = row.get(3)?;
            let resolution: String = row.get(4)?;
            let created: String = row.get(5)?;
            let sessions: i64 = row.get(6)?;
            Ok(PlanSummary {
                id: id.parse().unwrap_or_default(),
                name: row.get(1)?,
                goal: goal.parse().unwrap_or(GoalKind::Maintain),
                status: status.parse().unwrap_or(PlanStatus::Draft),
                resolution: serde_json::from_str(&resolution).unwrap_or(AnchorResolution {
                    start: Date::MIN,
                    end: Date::MIN,
                    weeks: 0,
                    race_date: None,
                    adjusted: false,
                    note: None,
                }),
                created_at: parse_timestamp_loose(&created),
                session_count: sessions as usize,
            })
        })?;
        rows.map(|r| r.map_err(StoreError::from)).collect()
    }

    /// The plan currently driving the calendar and the dashboard.
    pub fn active(db: &Db) -> Result<Option<Plan>> {
        // `Self::load` re-enters the lock, so the id lookup must finish first.
        let id: Option<String> = {
            let conn = db.conn();
            conn.query_row(
                "SELECT id FROM plan WHERE status = 'active' ORDER BY created_at DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?
        };
        match id {
            None => Ok(None),
            Some(id) => {
                let parsed: PlanId =
                    id.parse()
                        .map_err(|e: uuid::Error| StoreError::InvalidValue {
                            field: "plan_id",
                            value: format!("{id} ({e})"),
                        })?;
                Self::load(db, parsed).map(Some)
            }
        }
    }

    pub fn set_status(db: &Db, id: PlanId, status: PlanStatus) -> Result<()> {
        let conn = db.conn();
        let changes = conn.execute(
            "UPDATE plan SET status = ?2, updated_at = ?3 WHERE id = ?1",
            params![id.to_string(), status.as_str(), write_timestamp(now())],
        )?;
        if changes == 0 {
            return Err(StoreError::NotFound {
                entity: "plan",
                id: id.to_string(),
            });
        }
        Ok(())
    }

    /// Record a provider-side id, so a later re-plan updates instead of
    /// creating a duplicate plan on the watch.
    pub fn record_external_id(
        db: &Db,
        id: PlanId,
        provider: &str,
        external_id: &str,
    ) -> Result<()> {
        let conn = db.conn();
        let mut current: Vec<(String, String)> = conn
            .query_row(
                "SELECT external_json FROM plan WHERE id = ?1",
                params![id.to_string()],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .map(|text| serde_json::from_str(&text).unwrap_or_default())
            .unwrap_or_default();

        current.retain(|(p, _)| p != provider);
        current.push((provider.to_string(), external_id.to_string()));
        conn.execute(
            "UPDATE plan SET external_json = ?2, updated_at = ?3 WHERE id = ?1",
            params![
                id.to_string(),
                write_json(&current)?,
                write_timestamp(now())
            ],
        )?;
        Ok(())
    }

    /// Update one session in place, used by the MCP `update_plan_days` tool.
    pub fn update_session(db: &Db, session: &PlannedSession) -> Result<()> {
        let conn = db.conn();
        let changes = conn.execute(
            "UPDATE planned_session
             SET date = ?2, start_time = ?3, kind = ?4, title = ?5, intent = ?6,
                 workout_json = ?7, target_volume = ?8, target_duration = ?9,
                 target_pace = ?10, rpe_target = ?11, quality = ?12, external_id = ?13
             WHERE id = ?1",
            params![
                session.id.to_string(),
                write_date(session.date),
                write_time(session.start),
                session.kind.as_str(),
                session.title,
                session.intent,
                write_json(&session.workout)?,
                session.target_volume.as_f64(),
                i64::from(session.target_duration.as_u32()),
                session
                    .target_pace
                    .map(runalytics_core::Pace::as_secs_per_km),
                session.rpe_target.map(i64::from),
                write_bool(session.quality),
                session.external_id
            ],
        )?;
        if changes == 0 {
            return Err(StoreError::NotFound {
                entity: "planned_session",
                id: session.id.to_string(),
            });
        }
        Ok(())
    }

    /// Sessions scheduled from `from` onward across every mutable plan, which
    /// is what the calendar sink and the "next up" widget need.
    pub fn upcoming_sessions(db: &Db, from: Date) -> Result<Vec<PlannedSession>> {
        let conn = db.conn();
        let mut stmt = conn.prepare(
            "SELECT s.id, s.week_idx, s.date, s.start_time, s.kind, s.title, s.intent,
                    s.workout_json, s.target_volume, s.target_duration, s.target_pace,
                    s.rpe_target, s.quality, s.external_id
             FROM planned_session s
             JOIN plan p ON p.id = s.plan_id
             WHERE s.date >= ?1 AND p.status IN ('draft','active','paused')
             ORDER BY s.date",
        )?;
        let rows = stmt.query_map(params![write_date(from)], row_to_session)?;
        rows.map(|r| r.map_err(StoreError::from)).collect()
    }

    pub fn session_plan_id(db: &Db, session_id: PlannedSessionId) -> Result<PlanId> {
        let conn = db.conn();
        let text: String = conn
            .query_row(
                "SELECT plan_id FROM planned_session WHERE id = ?1",
                params![session_id.to_string()],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| StoreError::NotFound {
                entity: "planned_session",
                id: session_id.to_string(),
            })?;
        text.parse()
            .map_err(|e: uuid::Error| StoreError::InvalidValue {
                field: "plan_id",
                value: format!("{text} ({e})"),
            })
    }
}

/// A plan as listed, without its sessions.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanSummary {
    pub id: PlanId,
    pub name: String,
    pub goal: GoalKind,
    pub status: PlanStatus,
    pub resolution: AnchorResolution,
    pub created_at: Timestamp,
    pub session_count: usize,
}

/// Returns the plan alongside one week; the first row of the join carries the
/// plan itself and every row contributes one week.
fn row_to_plan_week(row: &rusqlite::Row<'_>) -> rusqlite::Result<(Plan, Option<PlanWeek>)> {
    let id: String = row.get(0)?;
    let goal: String = row.get(2)?;
    let status: String = row.get(3)?;
    let anchor: String = row.get(4)?;
    let resolution: String = row.get(5)?;
    let athlete: String = row.get(6)?;
    let phases: String = row.get(7)?;
    let external: String = row.get(10)?;
    let created: String = row.get(11)?;

    let plan = Plan {
        id: id.parse().unwrap_or_default(),
        name: row.get(1)?,
        goal: goal.parse().unwrap_or(GoalKind::Maintain),
        status: status.parse().unwrap_or(PlanStatus::Draft),
        anchor: serde_json::from_str(&anchor).unwrap_or(Anchor::Horizon { weeks: 4 }),
        resolution: serde_json::from_str(&resolution).unwrap_or(AnchorResolution {
            start: Date::MIN,
            end: Date::MIN,
            weeks: 0,
            race_date: None,
            adjusted: false,
            note: None,
        }),
        phases: serde_json::from_str(&phases).unwrap_or_default(),
        weeks: Vec::new(),
        athlete: serde_json::from_str(&athlete).unwrap_or(AthleteSnapshot::placeholder(
            "UTC".parse().unwrap_or(runalytics_core::Tz::UTC),
        )),
        ceiling_volume: VolumeKm(row.get(8)?),
        emittable_as_coros_plan: read_bool(row, "emittable"),
        external_ids: serde_json::from_str(&external).unwrap_or_default(),
        created_at: parse_timestamp_loose(&created),
    };

    // LEFT JOIN: a plan with no weeks still has a row, with NULL week columns.
    let week_idx: Option<i64> = row.get(12)?;
    let week = match week_idx {
        None => None,
        Some(idx) => {
            let phase: String = row.get(13)?;
            let start: String = row.get(14)?;
            let end: String = row.get(15)?;
            Some(PlanWeek {
                index: idx as u8,
                phase: phase.parse().unwrap_or(runalytics_core::Phase::Base),
                start: parse_date(&start).unwrap_or(Date::MIN),
                end: parse_date(&end).unwrap_or(Date::MIN),
                target_volume: VolumeKm(row.get(16)?),
                previous_volume: VolumeKm(row.get(17)?),
                step_pct: row.get(18)?,
                projected_acwr: row.get(19)?,
                sessions: Vec::new(),
                is_deload: read_bool(row, "is_deload"),
            })
        }
    };
    Ok((plan, week))
}

fn row_to_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<PlannedSession> {
    let id: String = row.get(0)?;
    let date: String = row.get(2)?;
    let start: String = row.get(3)?;
    let kind: String = row.get(4)?;
    let workout: String = row.get(7)?;
    let target_pace: Option<f64> = row.get(10)?;
    Ok(PlannedSession {
        id: id.parse().unwrap_or_default(),
        date: parse_date(&date).unwrap_or(Date::MIN),
        start: parse_time(&start).unwrap_or_default(),
        kind: kind.parse().unwrap_or(SessionKind::Easy),
        title: row.get(5)?,
        intent: row.get(6)?,
        workout: serde_json::from_str(&workout).unwrap_or(StructuredWorkout::default()),
        target_volume: VolumeKm(row.get(8)?),
        target_duration: runalytics_core::DurationSecs(row.get::<_, i64>(9)? as u32),
        target_pace: target_pace.map(Pace::new),
        rpe_target: row.get::<_, Option<i64>>(11)?.map(|v| v as u8),
        quality: read_bool(row, "quality"),
        external_id: row.get(13)?,
    })
}

// ---------------------------------------------------------------------------
// scores
// ---------------------------------------------------------------------------

/// A stored session score.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionScoreRow {
    pub session_id: PlannedSessionId,
    pub formula_version: i64,
    pub score: f64,
    pub tss: f64,
    pub intensity_factor: f64,
    pub duration_s: u32,
    pub load: f64,
    pub planned_quality: bool,
    pub executed_quality: bool,
    pub adherence: Option<f64>,
    pub components: serde_json::Value,
}

/// A stored daily readiness score.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadinessRow {
    pub date: Date,
    pub formula_version: i64,
    pub score: f64,
    pub components: serde_json::Value,
    pub inputs: serde_json::Value,
}

/// A stored daily injury-risk score.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InjuryRiskRow {
    pub date: Date,
    pub formula_version: i64,
    pub score: f64,
    pub band: String,
    pub drivers: serde_json::Value,
}

/// A stored daily load and performance snapshot.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricSnapshotRow {
    pub date: Date,
    pub formula_version: i64,
    pub chronic_load: f64,
    pub acute_load: f64,
    pub training_stress_balance: f64,
    pub acwr: f64,
    pub monotony: f64,
    pub weekly_spike: f64,
    pub vo2max_estimate: Option<f64>,
    pub threshold_pace: Option<f64>,
    pub performance_index: Option<f64>,
    pub components: serde_json::Value,
}

/// Session, readiness, injury-risk and load snapshots.
pub struct ScoreRepo;

impl ScoreRepo {
    pub fn save_session_score(db: &Db, row: &SessionScoreRow) -> Result<()> {
        db.conn().execute(
            "INSERT INTO session_score
                (session_id, formula_version, score, tss, intensity_factor, duration_s,
                 load, planned_quality, executed_quality, adherence, components_json, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
             ON CONFLICT(session_id) DO UPDATE SET
                formula_version = excluded.formula_version,
                score = excluded.score,
                tss = excluded.tss,
                intensity_factor = excluded.intensity_factor,
                duration_s = excluded.duration_s,
                load = excluded.load,
                planned_quality = excluded.planned_quality,
                executed_quality = excluded.executed_quality,
                adherence = excluded.adherence,
                components_json = excluded.components_json",
            params![
                row.session_id.to_string(),
                row.formula_version,
                row.score,
                row.tss,
                row.intensity_factor,
                i64::from(row.duration_s),
                row.load,
                write_bool(row.planned_quality),
                write_bool(row.executed_quality),
                row.adherence,
                row.components.to_string(),
                write_timestamp(now())
            ],
        )?;
        Ok(())
    }

    pub fn session_score(db: &Db, session_id: PlannedSessionId) -> Result<Option<SessionScoreRow>> {
        let conn = db.conn();
        conn.query_row(
            "SELECT session_id, formula_version, score, tss, intensity_factor, duration_s,
                    load, planned_quality, executed_quality, adherence, components_json
             FROM session_score WHERE session_id = ?1",
            params![session_id.to_string()],
            |row| {
                let id: String = row.get(0)?;
                let components: String = row.get(10)?;
                Ok(SessionScoreRow {
                    session_id: id.parse().unwrap_or_default(),
                    formula_version: row.get(1)?,
                    score: row.get(2)?,
                    tss: row.get(3)?,
                    intensity_factor: row.get(4)?,
                    duration_s: row.get::<_, i64>(5)? as u32,
                    load: row.get(6)?,
                    planned_quality: read_bool(row, "planned_quality"),
                    executed_quality: read_bool(row, "executed_quality"),
                    adherence: row.get(9)?,
                    components: serde_json::from_str(&components)
                        .unwrap_or(serde_json::Value::Null),
                })
            },
        )
        .optional()
        .map_err(StoreError::from)
    }

    /// Scores for every session in a date range, for the adherence chart.
    pub fn session_scores_range(db: &Db, from: Date, to: Date) -> Result<Vec<SessionScoreRow>> {
        let conn = db.conn();
        let mut stmt = conn.prepare(
            "SELECT s.session_id, s.formula_version, s.score, s.tss, s.intensity_factor,
                    s.duration_s, s.load, s.planned_quality, s.executed_quality, s.adherence,
                    s.components_json
             FROM session_score s
             JOIN planned_session p ON p.id = s.session_id
             WHERE p.date BETWEEN ?1 AND ?2
             ORDER BY p.date",
        )?;
        let rows = stmt.query_map(params![write_date(from), write_date(to)], |row| {
            let id: String = row.get(0)?;
            let components: String = row.get(10)?;
            Ok(SessionScoreRow {
                session_id: id.parse().unwrap_or_default(),
                formula_version: row.get(1)?,
                score: row.get(2)?,
                tss: row.get(3)?,
                intensity_factor: row.get(4)?,
                duration_s: row.get::<_, i64>(5)? as u32,
                load: row.get(6)?,
                planned_quality: read_bool(row, "planned_quality"),
                executed_quality: read_bool(row, "executed_quality"),
                adherence: row.get(9)?,
                components: serde_json::from_str(&components).unwrap_or(serde_json::Value::Null),
            })
        })?;
        rows.map(|r| r.map_err(StoreError::from)).collect()
    }

    /// Scores produced by a formula version older than `version`, so a formula
    /// change can be backfilled rather than silently diverging.
    pub fn stale_session_scores(db: &Db, version: i64) -> Result<Vec<SessionScoreRow>> {
        let conn = db.conn();
        let mut stmt = conn.prepare(
            "SELECT session_id, formula_version, score, tss, intensity_factor, duration_s,
                    load, planned_quality, executed_quality, adherence, components_json
             FROM session_score WHERE formula_version < ?1",
        )?;
        let rows = stmt.query_map(params![version], |row| {
            let id: String = row.get(0)?;
            let components: String = row.get(10)?;
            Ok(SessionScoreRow {
                session_id: id.parse().unwrap_or_default(),
                formula_version: row.get(1)?,
                score: row.get(2)?,
                tss: row.get(3)?,
                intensity_factor: row.get(4)?,
                duration_s: row.get::<_, i64>(5)? as u32,
                load: row.get(6)?,
                planned_quality: read_bool(row, "planned_quality"),
                executed_quality: read_bool(row, "executed_quality"),
                adherence: row.get(9)?,
                components: serde_json::from_str(&components).unwrap_or(serde_json::Value::Null),
            })
        })?;
        rows.map(|r| r.map_err(StoreError::from)).collect()
    }

    pub fn save_metric_snapshot(db: &Db, row: &MetricSnapshotRow) -> Result<()> {
        db.conn().execute(
            "INSERT INTO metric_snapshot
                (date, formula_version, chronic_load, acute_load, training_stress_balance,
                 acwr, monotony, weekly_spike, vo2max_estimate, threshold_pace,
                 performance_index, components_json, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)
             ON CONFLICT(date) DO UPDATE SET
                formula_version = excluded.formula_version,
                chronic_load = excluded.chronic_load,
                acute_load = excluded.acute_load,
                training_stress_balance = excluded.training_stress_balance,
                acwr = excluded.acwr,
                monotony = excluded.monotony,
                weekly_spike = excluded.weekly_spike,
                vo2max_estimate = excluded.vo2max_estimate,
                threshold_pace = excluded.threshold_pace,
                performance_index = excluded.performance_index,
                components_json = excluded.components_json",
            params![
                write_date(row.date),
                row.formula_version,
                row.chronic_load,
                row.acute_load,
                row.training_stress_balance,
                row.acwr,
                row.monotony,
                row.weekly_spike,
                row.vo2max_estimate,
                row.threshold_pace,
                row.performance_index,
                row.components.to_string(),
                write_timestamp(now())
            ],
        )?;
        Ok(())
    }

    pub fn metric_snapshots_range(db: &Db, from: Date, to: Date) -> Result<Vec<MetricSnapshotRow>> {
        let conn = db.conn();
        let mut stmt = conn.prepare(
            "SELECT date, formula_version, chronic_load, acute_load, training_stress_balance,
                    acwr, monotony, weekly_spike, vo2max_estimate, threshold_pace,
                    performance_index, components_json
             FROM metric_snapshot WHERE date BETWEEN ?1 AND ?2 ORDER BY date",
        )?;
        let rows = stmt.query_map(params![write_date(from), write_date(to)], |row| {
            let date: String = row.get(0)?;
            let components: String = row.get(11)?;
            let threshold: Option<f64> = row.get(9)?;
            Ok(MetricSnapshotRow {
                date: parse_date(&date).unwrap_or(Date::MIN),
                formula_version: row.get(1)?,
                chronic_load: row.get(2)?,
                acute_load: row.get(3)?,
                training_stress_balance: row.get(4)?,
                acwr: row.get(5)?,
                monotony: row.get(6)?,
                weekly_spike: row.get(7)?,
                vo2max_estimate: row.get(8)?,
                threshold_pace: threshold
                    .map(Pace::new)
                    .map(runalytics_core::Pace::as_secs_per_km),
                performance_index: row.get(10)?,
                components: serde_json::from_str(&components).unwrap_or(serde_json::Value::Null),
            })
        })?;
        rows.map(|r| r.map_err(StoreError::from)).collect()
    }

    pub fn latest_metric_snapshot(db: &Db) -> Result<Option<MetricSnapshotRow>> {
        let conn = db.conn();
        conn.query_row(
            "SELECT date, formula_version, chronic_load, acute_load, training_stress_balance,
                    acwr, monotony, weekly_spike, vo2max_estimate, threshold_pace,
                    performance_index, components_json
             FROM metric_snapshot ORDER BY date DESC LIMIT 1",
            [],
            |row| {
                let date: String = row.get(0)?;
                let components: String = row.get(11)?;
                Ok(MetricSnapshotRow {
                    date: parse_date(&date).unwrap_or(Date::MIN),
                    formula_version: row.get(1)?,
                    chronic_load: row.get(2)?,
                    acute_load: row.get(3)?,
                    training_stress_balance: row.get(4)?,
                    acwr: row.get(5)?,
                    monotony: row.get(6)?,
                    weekly_spike: row.get(7)?,
                    vo2max_estimate: row.get(8)?,
                    threshold_pace: row.get(9)?,
                    performance_index: row.get(10)?,
                    components: serde_json::from_str(&components)
                        .unwrap_or(serde_json::Value::Null),
                })
            },
        )
        .optional()
        .map_err(StoreError::from)
    }
}

/// Daily readiness and injury-risk scores.
pub struct ReadinessRepo;

impl ReadinessRepo {
    pub fn save_readiness(db: &Db, row: &ReadinessRow) -> Result<()> {
        db.conn().execute(
            "INSERT INTO readiness_day
                (date, formula_version, score, components_json, inputs_json, created_at)
             VALUES (?1,?2,?3,?4,?5,?6)
             ON CONFLICT(date) DO UPDATE SET
                formula_version = excluded.formula_version,
                score = excluded.score,
                components_json = excluded.components_json,
                inputs_json = excluded.inputs_json",
            params![
                write_date(row.date),
                row.formula_version,
                row.score,
                row.components.to_string(),
                row.inputs.to_string(),
                write_timestamp(now())
            ],
        )?;
        Ok(())
    }

    pub fn readiness(db: &Db, date: Date) -> Result<Option<ReadinessRow>> {
        let conn = db.conn();
        conn.query_row(
            "SELECT date, formula_version, score, components_json, inputs_json
             FROM readiness_day WHERE date = ?1",
            params![write_date(date)],
            |row| {
                let date: String = row.get(0)?;
                let components: String = row.get(3)?;
                let inputs: String = row.get(4)?;
                Ok(ReadinessRow {
                    date: parse_date(&date).unwrap_or(Date::MIN),
                    formula_version: row.get(1)?,
                    score: row.get(2)?,
                    components: serde_json::from_str(&components)
                        .unwrap_or(serde_json::Value::Null),
                    inputs: serde_json::from_str(&inputs).unwrap_or(serde_json::Value::Null),
                })
            },
        )
        .optional()
        .map_err(StoreError::from)
    }

    pub fn readiness_range(db: &Db, from: Date, to: Date) -> Result<Vec<ReadinessRow>> {
        let conn = db.conn();
        let mut stmt = conn.prepare(
            "SELECT date, formula_version, score, components_json, inputs_json
             FROM readiness_day WHERE date BETWEEN ?1 AND ?2 ORDER BY date",
        )?;
        let rows = stmt.query_map(params![write_date(from), write_date(to)], |row| {
            let date: String = row.get(0)?;
            let components: String = row.get(3)?;
            let inputs: String = row.get(4)?;
            Ok(ReadinessRow {
                date: parse_date(&date).unwrap_or(Date::MIN),
                formula_version: row.get(1)?,
                score: row.get(2)?,
                components: serde_json::from_str(&components).unwrap_or(serde_json::Value::Null),
                inputs: serde_json::from_str(&inputs).unwrap_or(serde_json::Value::Null),
            })
        })?;
        rows.map(|r| r.map_err(StoreError::from)).collect()
    }

    pub fn save_injury_risk(db: &Db, row: &InjuryRiskRow) -> Result<()> {
        db.conn().execute(
            "INSERT INTO injury_risk_day
                (date, formula_version, score, band, drivers_json, created_at)
             VALUES (?1,?2,?3,?4,?5,?6)
             ON CONFLICT(date) DO UPDATE SET
                formula_version = excluded.formula_version,
                score = excluded.score,
                band = excluded.band,
                drivers_json = excluded.drivers_json",
            params![
                write_date(row.date),
                row.formula_version,
                row.score,
                row.band,
                row.drivers.to_string(),
                write_timestamp(now())
            ],
        )?;
        Ok(())
    }

    pub fn injury_risk(db: &Db, date: Date) -> Result<Option<InjuryRiskRow>> {
        let conn = db.conn();
        conn.query_row(
            "SELECT date, formula_version, score, band, drivers_json
             FROM injury_risk_day WHERE date = ?1",
            params![write_date(date)],
            |row| {
                let date: String = row.get(0)?;
                let drivers: String = row.get(4)?;
                Ok(InjuryRiskRow {
                    date: parse_date(&date).unwrap_or(Date::MIN),
                    formula_version: row.get(1)?,
                    score: row.get(2)?,
                    band: row.get(3)?,
                    drivers: serde_json::from_str(&drivers).unwrap_or(serde_json::Value::Null),
                })
            },
        )
        .optional()
        .map_err(StoreError::from)
    }

    pub fn injury_risk_range(db: &Db, from: Date, to: Date) -> Result<Vec<InjuryRiskRow>> {
        let conn = db.conn();
        let mut stmt = conn.prepare(
            "SELECT date, formula_version, score, band, drivers_json
             FROM injury_risk_day WHERE date BETWEEN ?1 AND ?2 ORDER BY date",
        )?;
        let rows = stmt.query_map(params![write_date(from), write_date(to)], |row| {
            let date: String = row.get(0)?;
            let drivers: String = row.get(4)?;
            Ok(InjuryRiskRow {
                date: parse_date(&date).unwrap_or(Date::MIN),
                formula_version: row.get(1)?,
                score: row.get(2)?,
                band: row.get(3)?,
                drivers: serde_json::from_str(&drivers).unwrap_or(serde_json::Value::Null),
            })
        })?;
        rows.map(|r| r.map_err(StoreError::from)).collect()
    }
}

// ---------------------------------------------------------------------------
// feedback and sync runs
// ---------------------------------------------------------------------------

/// A subjective check-in.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackRow {
    pub id: uuid::Uuid,
    pub session_id: Option<PlannedSessionId>,
    pub date: Date,
    /// Perceived exertion, 1-10.
    pub rpe: Option<u8>,
    /// Mood, 1-5.
    pub mood: Option<u8>,
    /// Leg soreness, 1-5 where 5 is worst.
    pub legs: Option<u8>,
    /// Motivation, 1-5.
    pub motivation: Option<u8>,
    pub notes: String,
}

/// User-reported inputs that the scoring model treats as ground truth.
pub struct FeedbackRepo;

impl FeedbackRepo {
    pub fn insert(db: &Db, row: &FeedbackRow) -> Result<()> {
        db.conn().execute(
            "INSERT INTO feedback
                (id, session_id, date, rpe, mood, legs, motivation, notes, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                row.id.to_string(),
                row.session_id.map(|s| s.to_string()),
                write_date(row.date),
                row.rpe.map(i64::from),
                row.mood.map(i64::from),
                row.legs.map(i64::from),
                row.motivation.map(i64::from),
                row.notes,
                write_timestamp(now())
            ],
        )?;
        Ok(())
    }

    pub fn list_range(db: &Db, from: Date, to: Date) -> Result<Vec<FeedbackRow>> {
        let conn = db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, date, rpe, mood, legs, motivation, notes
             FROM feedback WHERE date BETWEEN ?1 AND ?2 ORDER BY date",
        )?;
        let rows = stmt.query_map(params![write_date(from), write_date(to)], |row| {
            let id: String = row.get(0)?;
            let session: Option<String> = row.get(1)?;
            let date: String = row.get(2)?;
            Ok(FeedbackRow {
                id: uuid::Uuid::parse_str(&id).unwrap_or_else(|_| uuid::Uuid::nil()),
                session_id: session.and_then(|s| s.parse().ok()),
                date: parse_date(&date).unwrap_or(Date::MIN),
                rpe: row.get::<_, Option<i64>>(3)?.map(|v| v as u8),
                mood: row.get::<_, Option<i64>>(4)?.map(|v| v as u8),
                legs: row.get::<_, Option<i64>>(5)?.map(|v| v as u8),
                motivation: row.get::<_, Option<i64>>(6)?.map(|v| v as u8),
                notes: row.get(7)?,
            })
        })?;
        rows.map(|r| r.map_err(StoreError::from)).collect()
    }
}

/// One provider pull, for the sync log in Settings.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncRunRow {
    pub id: uuid::Uuid,
    pub account_id: ProviderAccountId,
    pub data_class: String,
    pub started_at: Timestamp,
    pub finished_at: Option<Timestamp>,
    pub status: String,
    pub records_upserted: i64,
    pub error: Option<String>,
}

/// Sync history.
pub struct SyncRunRepo;

impl SyncRunRepo {
    pub fn start(db: &Db, account_id: ProviderAccountId, data_class: &str) -> Result<uuid::Uuid> {
        let id = uuid::Uuid::now_v7();
        db.conn().execute(
            "INSERT INTO sync_run (id, account_id, data_class, started_at, status)
             VALUES (?1,?2,?3,?4,'running')",
            params![
                id.to_string(),
                account_id.to_string(),
                data_class,
                write_timestamp(now())
            ],
        )?;
        Ok(id)
    }

    pub fn finish(
        db: &Db,
        id: uuid::Uuid,
        status: &str,
        records: i64,
        error: Option<&str>,
    ) -> Result<()> {
        db.conn().execute(
            "UPDATE sync_run SET finished_at = ?2, status = ?3, records_upserted = ?4,
                    error = ?5
             WHERE id = ?1",
            params![
                id.to_string(),
                write_timestamp(now()),
                status,
                records,
                error
            ],
        )?;
        Ok(())
    }

    pub fn recent(db: &Db, limit: usize) -> Result<Vec<SyncRunRow>> {
        let conn = db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, account_id, data_class, started_at, finished_at, status,
                    records_upserted, error
             FROM sync_run
             -- Timestamps are stored to the second, so runs in the same second
             -- would tie. The id is a v7 UUID, so it breaks ties by creation
             -- order without an extra column.
             ORDER BY started_at DESC, id DESC LIMIT ?1",
        )?;
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let rows = stmt.query_map(params![limit], |row| {
            let id: String = row.get(0)?;
            let account: String = row.get(1)?;
            let started: String = row.get(3)?;
            let finished: Option<String> = row.get(4)?;
            Ok(SyncRunRow {
                id: uuid::Uuid::parse_str(&id).unwrap_or_else(|_| uuid::Uuid::nil()),
                account_id: account.parse().unwrap_or_default(),
                data_class: row.get(2)?,
                started_at: parse_timestamp_loose(&started),
                finished_at: finished.as_deref().map(parse_timestamp_loose),
                status: row.get(5)?,
                records_upserted: row.get(6)?,
                error: row.get(7)?,
            })
        })?;
        rows.map(|r| r.map_err(StoreError::from)).collect()
    }
}

/// Existence check used by the UI to decide whether to show onboarding.
pub fn has_any_data(db: &Db) -> Result<bool> {
    let conn = db.conn();
    let activities: i64 = conn.query_row("SELECT COUNT(*) FROM activity", [], |r| r.get(0))?;
    let plans: i64 = conn.query_row("SELECT COUNT(*) FROM plan", [], |r| r.get(0))?;
    Ok(activities > 0 || plans > 0)
}

/// Confirm a session id exists, so MCP tools can fail with a clear message.
pub fn session_exists(db: &Db, id: PlannedSessionId) -> Result<bool> {
    exists(
        &db.conn(),
        "SELECT 1 FROM planned_session WHERE id = ?1 LIMIT 1",
        &id.to_string(),
    )
}

/// Convenience for callers that only need a fresh health-day id.
#[must_use]
pub fn new_health_day_id() -> HealthDayId {
    HealthDayId::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use runalytics_core::{
        ActivitySummary, Anchor, AnchorResolution, AthleteSnapshot, BlockTarget, DurationSecs,
        GoalKind, Phase, PhaseSpan, PlanConstraints, PlanWeek, ProviderAccountId, SessionKind,
        StructuredWorkout, WorkoutBlock,
    };

    fn date(y: i32, m: u32, d: u32) -> Date {
        Date::from_ymd_opt(y, m, d).expect("test date")
    }

    fn tz() -> runalytics_core::Tz {
        "Europe/Madrid".parse().expect("tz")
    }

    fn account(db: &Db) -> ProviderAccountId {
        ProviderAccountRepo::upsert(db, Provider::Coros, "athlete@example.com", Some("eu"))
            .expect("account")
    }

    /// A six-week plan with two sessions per week, used by every plan test.
    #[allow(clippy::too_many_lines)] // one Plan literal, kept flat for readability
    fn sample_plan(start: Date) -> Plan {
        let weeks = (0..6)
            .map(|idx| {
                let week_start = start + chrono::Duration::days(i64::from(idx) * 7);
                let week_end = week_start + chrono::Duration::days(6);
                let volume = 30.0 + f64::from(idx) * 2.0;
                let sessions = vec![
                    PlannedSession {
                        id: PlannedSessionId::new(),
                        date: week_start,
                        start: chrono::NaiveTime::from_hms_opt(7, 0, 0).expect("time"),
                        kind: SessionKind::Tempo,
                        title: format!("Tempo w{idx}"),
                        intent: "Threshold development".into(),
                        workout: StructuredWorkout::with_warmup_cooldown(
                            DurationSecs::from_minutes(15),
                            vec![WorkoutBlock::new(
                                BlockTarget::Steady,
                                DurationSecs::from_minutes(20),
                            )],
                            DurationSecs::from_minutes(10),
                        ),
                        target_volume: VolumeKm(10.0),
                        target_duration: DurationSecs::from_minutes(45),
                        target_pace: Some(Pace::new(270.0)),
                        rpe_target: Some(7),
                        quality: true,
                        external_id: None,
                    },
                    PlannedSession {
                        id: PlannedSessionId::new(),
                        date: week_end,
                        start: chrono::NaiveTime::from_hms_opt(9, 0, 0).expect("time"),
                        kind: SessionKind::LongRun,
                        title: format!("Long run w{idx}"),
                        intent: "Aerobic durability".into(),
                        workout: StructuredWorkout::continuous(
                            BlockTarget::Easy,
                            DurationSecs::from_minutes(75),
                        ),
                        target_volume: VolumeKm(volume * 0.35),
                        target_duration: DurationSecs::from_minutes(75),
                        target_pace: None,
                        rpe_target: Some(4),
                        quality: false,
                        external_id: None,
                    },
                ];
                PlanWeek {
                    index: idx,
                    phase: match idx {
                        0..=1 => Phase::Base,
                        2..=3 => Phase::Build,
                        4 => Phase::Peak,
                        _ => Phase::Taper,
                    },
                    start: week_start,
                    end: week_end,
                    target_volume: VolumeKm(volume),
                    previous_volume: VolumeKm(volume - 2.0),
                    step_pct: 0.07,
                    projected_acwr: 1.05,
                    sessions,
                    is_deload: idx == 5,
                }
            })
            .collect();

        Plan {
            id: PlanId::new(),
            name: "Berlin build".into(),
            goal: GoalKind::Marathon,
            status: PlanStatus::Draft,
            anchor: Anchor::Horizon { weeks: 6 },
            resolution: AnchorResolution {
                start,
                end: start + chrono::Duration::days(41),
                weeks: 6,
                race_date: None,
                adjusted: false,
                note: None,
            },
            phases: vec![
                PhaseSpan {
                    phase: Phase::Base,
                    from_week: 0,
                    to_week: 1,
                    note: "Prepare tissue".into(),
                },
                PhaseSpan {
                    phase: Phase::Taper,
                    from_week: 5,
                    to_week: 5,
                    note: "Shed fatigue".into(),
                },
            ],
            weeks,
            athlete: AthleteSnapshot::placeholder(tz()),
            ceiling_volume: VolumeKm(55.0),
            emittable_as_coros_plan: true,
            external_ids: Vec::new(),
            created_at: now(),
        }
    }

    fn sample_activity(account_id: ProviderAccountId) -> Activity {
        Activity {
            id: ActivityId::new(),
            account: account_id,
            provider_activity_id: "coros-9001".into(),
            name: "Morning tempo".into(),
            started_at: parse_timestamp("2026-10-06T05:00:00Z").expect("ts"),
            local_date: date(2026, 10, 6),
            summary: ActivitySummary {
                distance: VolumeKm(10.4),
                duration: DurationSecs::from_minutes(52),
                avg_pace: Some(Pace::new(300.0)),
                avg_hr: Some(HeartRate::new(152)),
                max_hr: Some(HeartRate::new(174)),
                elevation_gain: Some(84.0),
                avg_cadence: Some(172),
                training_load: Some(68.0),
            },
            laps: vec![
                ActivityLap {
                    index: 0,
                    start: parse_timestamp("2026-10-06T05:00:00Z").expect("ts"),
                    duration: DurationSecs::from_minutes(15),
                    distance: VolumeKm(3.0),
                    avg_pace: Some(Pace::new(300.0)),
                    avg_hr: Some(HeartRate::new(130)),
                    max_hr: None,
                    elevation_gain: Some(20.0),
                    cadence: Some(170),
                },
                ActivityLap {
                    index: 1,
                    start: parse_timestamp("2026-10-06T05:15:00Z").expect("ts"),
                    duration: DurationSecs::from_minutes(20),
                    distance: VolumeKm(4.4),
                    avg_pace: Some(Pace::new(272.0)),
                    avg_hr: Some(HeartRate::new(168)),
                    max_hr: Some(HeartRate::new(174)),
                    elevation_gain: Some(44.0),
                    cadence: Some(180),
                },
            ],
            intensity: Intensity::Tempo,
            matched_session: None,
            fetched_at: now(),
        }
    }

    #[test]
    fn account_upsert_is_idempotent_and_preserves_the_id() {
        let db = Db::in_memory().expect("db");
        let first = ProviderAccountRepo::upsert(&db, Provider::Coros, "me@example.com", Some("eu"))
            .expect("upsert");
        let second =
            ProviderAccountRepo::upsert(&db, Provider::Coros, "me@example.com", Some("us"))
                .expect("upsert");
        assert_eq!(first, second, "reconnecting must not mint a new id");
        assert_eq!(ProviderAccountRepo::list(&db).expect("list").len(), 1);

        let account = ProviderAccountRepo::get(&db, first).expect("get");
        assert_eq!(account.region.as_deref(), Some("us"), "region refreshes");
        assert!(account.writable, "COROS is writable");
        assert!(
            ProviderAccountRepo::find_by_provider(&db, Provider::Garmin)
                .expect("find")
                .is_none()
        );
    }

    #[test]
    fn sync_cursor_round_trips_per_data_class() {
        let db = Db::in_memory().expect("db");
        let id = account(&db);
        assert!(
            ProviderAccountRepo::cursor(&db, id, "activities")
                .expect("cursor")
                .is_none()
        );

        ProviderAccountRepo::set_cursor(&db, id, "activities", date(2026, 9, 1)).expect("set");
        ProviderAccountRepo::set_cursor(&db, id, "health", date(2026, 9, 20)).expect("set");
        ProviderAccountRepo::set_cursor(&db, id, "activities", date(2026, 10, 1)).expect("set");

        assert_eq!(
            ProviderAccountRepo::cursor(&db, id, "activities").expect("cursor"),
            Some(date(2026, 10, 1)),
            "the later write wins"
        );
        let cursors = ProviderAccountRepo::cursors(&db, id).expect("cursors");
        assert_eq!(cursors.len(), 2, "one row per data class");
    }

    #[test]
    fn activity_round_trips_with_laps() {
        let db = Db::in_memory().expect("db");
        let id = account(&db);
        let activity = sample_activity(id);
        let expected_id = activity.id;

        assert!(
            ActivityRepo::upsert(&db, &activity).expect("upsert"),
            "new row"
        );
        let loaded = ActivityRepo::get(&db, expected_id).expect("load");

        assert_eq!(loaded.provider_activity_id, "coros-9001");
        assert_eq!(loaded.laps.len(), 2);
        assert_eq!(loaded.laps[1].index, 1);
        assert_eq!(loaded.laps[1].avg_hr, Some(HeartRate::new(168)));
        assert_eq!(loaded.summary.distance, VolumeKm(10.4));
        assert_eq!(loaded.intensity, Intensity::Tempo);
        assert_eq!(loaded.local_date, date(2026, 10, 6));
    }

    #[test]
    fn re_syncing_an_activity_updates_rather_than_duplicates() {
        let db = Db::in_memory().expect("db");
        let id = account(&db);
        let mut activity = sample_activity(id);
        ActivityRepo::upsert(&db, &activity).expect("first");

        activity.name = "Renamed by provider".into();
        activity.summary.distance = VolumeKm(11.1);
        let is_new = ActivityRepo::upsert(&db, &activity).expect("second");
        assert!(!is_new, "a re-sync is not a new record");

        let loaded = ActivityRepo::get(&db, activity.id).expect("load");
        assert_eq!(loaded.name, "Renamed by provider");
        assert_eq!(loaded.summary.distance, VolumeKm(11.1));
        assert_eq!(
            ActivityRepo::list_range(&db, date(2026, 10, 1), date(2026, 10, 31))
                .expect("list")
                .len(),
            1,
            "still exactly one row"
        );
    }

    #[test]
    fn unmatched_activities_surface_for_the_auto_match_pass() {
        let db = Db::in_memory().expect("db");
        let id = account(&db);
        let activity = sample_activity(id);
        let activity_id = activity.id;
        ActivityRepo::upsert(&db, &activity).expect("upsert");

        assert_eq!(
            ActivityRepo::unmatched_on(&db, date(2026, 10, 6))
                .expect("unmatched")
                .len(),
            1
        );

        let session = PlannedSessionId::new();
        ActivityRepo::match_session(&db, activity_id, Some(session)).expect("match");
        assert_eq!(
            ActivityRepo::unmatched_on(&db, date(2026, 10, 6))
                .expect("unmatched")
                .len(),
            0
        );
        assert_eq!(
            ActivityRepo::get(&db, activity_id)
                .expect("load")
                .matched_session,
            Some(session)
        );
    }

    #[test]
    fn health_day_round_trips_including_sleep() {
        let db = Db::in_memory().expect("db");
        let id = account(&db);
        let day = HealthDay {
            id: new_health_day_id(),
            account: id,
            date: date(2026, 10, 6),
            resting_hr: Some(HeartRate::new(48)),
            avg_stress: Some(31.0),
            high_stress_minutes: Some(12),
            steps: Some(9_200),
            provider_readiness: Some(78),
            basal_energy: Some(1_690.0),
            sleep: Some(runalytics_core::SleepDay {
                date: date(2026, 10, 6),
                total: DurationSecs::from_minutes(430),
                deep: DurationSecs::from_minutes(96),
                light: DurationSecs::from_minutes(210),
                rem: DurationSecs::from_minutes(108),
                awake: DurationSecs::from_minutes(18),
                nap: DurationSecs::ZERO,
                score: Some(84),
                lowest_hr: Some(HeartRate::new(44)),
                hrv: Some(62.0),
                respiratory_rate: Some(14.0),
            }),
        };
        HealthRepo::upsert(&db, &day).expect("upsert");

        let loaded = HealthRepo::get(&db, date(2026, 10, 6))
            .expect("get")
            .expect("present");
        assert_eq!(loaded.resting_hr, Some(HeartRate::new(48)));
        let sleep = loaded.sleep.expect("sleep survived the round trip");
        assert_eq!(sleep.total, DurationSecs::from_minutes(430));
        assert_eq!(sleep.hrv, Some(62.0));
        assert_eq!(
            HealthRepo::latest_date(&db).expect("latest"),
            Some(date(2026, 10, 6))
        );
    }

    #[test]
    fn plan_round_trips_weeks_and_sessions() {
        let db = Db::in_memory().expect("db");
        let plan = sample_plan(date(2026, 10, 5));
        let plan_id = plan.id;
        let tempo_id = plan.weeks[0].sessions[0].id;

        PlanRepo::save(&db, &plan).expect("save");
        let loaded = PlanRepo::load(&db, plan_id).expect("load");

        assert_eq!(loaded.name, "Berlin build");
        assert_eq!(loaded.goal, GoalKind::Marathon);
        assert_eq!(loaded.weeks.len(), 6);
        assert_eq!(loaded.phases.len(), 2, "phase timeline survives a reload");
        assert_eq!(loaded.phases[1].phase, Phase::Taper);
        assert_eq!(loaded.phases[1].note, "Shed fatigue");
        assert_eq!(loaded.resolution.weeks, 6);
        assert!(loaded.emittable_as_coros_plan);
        assert_eq!(loaded.ceiling_volume, VolumeKm(55.0));

        // Every week must carry exactly its own two sessions.
        for week in &loaded.weeks {
            assert_eq!(week.sessions.len(), 2, "week {}", week.index);
            for session in &week.sessions {
                assert!(
                    session.date >= week.start && session.date <= week.end,
                    "session {} escaped week {}",
                    session.date,
                    week.index
                );
            }
        }

        let tempo = loaded.session_by_id(tempo_id).expect("tempo found");
        assert_eq!(tempo.kind, SessionKind::Tempo);
        assert_eq!(tempo.workout.blocks.len(), 3, "warmup, steady, cooldown");
        assert_eq!(
            tempo.target_pace.map(runalytics_core::Pace::as_secs_per_km),
            Some(270.0)
        );
        assert_eq!(tempo.rpe_target, Some(7));
        assert!(tempo.quality);
    }

    #[test]
    fn saving_a_plan_twice_replaces_sessions_instead_of_doubling_them() {
        let db = Db::in_memory().expect("db");
        let mut plan = sample_plan(date(2026, 10, 5));
        PlanRepo::save(&db, &plan).expect("first save");

        plan.weeks[0].sessions.pop();
        PlanRepo::save(&db, &plan).expect("second save");
        let loaded = PlanRepo::load(&db, plan.id).expect("load");
        assert_eq!(
            loaded.weeks[0].sessions.len(),
            1,
            "the removed session is gone"
        );
        assert_eq!(loaded.all_sessions().count(), 11);
    }

    #[test]
    fn plan_status_gates_mutability_and_the_active_lookup() {
        let db = Db::in_memory().expect("db");
        let plan = sample_plan(date(2026, 10, 5));
        let plan_id = plan.id;
        PlanRepo::save(&db, &plan).expect("save");

        assert!(
            PlanRepo::active(&db).expect("active").is_none(),
            "draft is not active"
        );
        PlanRepo::set_status(&db, plan_id, PlanStatus::Active).expect("activate");
        assert_eq!(
            PlanRepo::active(&db).expect("active").map(|p| p.id),
            Some(plan_id)
        );

        let missing = PlanId::new();
        assert!(
            PlanRepo::set_status(&db, missing, PlanStatus::Active)
                .expect_err("unknown plan")
                .to_string()
                .contains("plan")
        );
    }

    #[test]
    fn external_ids_accumulate_per_provider() {
        let db = Db::in_memory().expect("db");
        let plan = sample_plan(date(2026, 10, 5));
        let plan_id = plan.id;
        PlanRepo::save(&db, &plan).expect("save");

        PlanRepo::record_external_id(&db, plan_id, "coros", "plan-77").expect("record");
        PlanRepo::record_external_id(&db, plan_id, "coros", "plan-78").expect("re-record");
        PlanRepo::record_external_id(&db, plan_id, "garmin", "g-1").expect("second provider");

        let loaded = PlanRepo::load(&db, plan_id).expect("load");
        assert_eq!(
            loaded.external_ids,
            vec![
                ("coros".to_string(), "plan-78".to_string()),
                ("garmin".to_string(), "g-1".to_string())
            ],
            "the same provider overwrites, a different provider appends"
        );
    }

    #[test]
    fn a_single_session_can_be_updated_in_place() {
        let db = Db::in_memory().expect("db");
        let plan = sample_plan(date(2026, 10, 5));
        let plan_id = plan.id;
        let session_id = plan.weeks[0].sessions[0].id;
        PlanRepo::save(&db, &plan).expect("save");

        let mut session = PlanRepo::load(&db, plan_id)
            .expect("load")
            .session_by_id(session_id)
            .expect("session")
            .clone();
        session.kind = SessionKind::Easy;
        session.quality = false;
        session.title = "Moved to easy".into();
        session.external_id = Some("coros-workout-5".into());
        PlanRepo::update_session(&db, &session).expect("update");

        let mut reloaded_plan = PlanRepo::load(&db, plan_id).expect("load");
        let reloaded = reloaded_plan
            .session_by_id_mut(session_id)
            .expect("session");
        assert_eq!(reloaded.kind, SessionKind::Easy);
        assert!(!reloaded.quality);
        assert_eq!(reloaded.external_id.as_deref(), Some("coros-workout-5"));
    }

    #[test]
    fn upcoming_sessions_span_plans_and_stop_at_the_horizon() {
        let db = Db::in_memory().expect("db");
        let plan = sample_plan(date(2026, 10, 5));
        PlanRepo::save(&db, &plan).expect("save");

        let upcoming = PlanRepo::upcoming_sessions(&db, date(2026, 10, 26)).expect("upcoming");
        assert!(
            upcoming.iter().all(|s| s.date >= date(2026, 10, 26)),
            "nothing in the past"
        );
        // The plan starts 2026-10-05 with a session on Monday and Sunday, so
        // from 2026-10-26 the last three weeks contribute two sessions each.
        assert_eq!(upcoming.len(), 6);
        assert_eq!(upcoming.first().expect("first").date, date(2026, 10, 26));
    }

    #[test]
    fn scores_round_trip_and_stale_versions_are_findable() {
        let db = Db::in_memory().expect("db");
        let plan = sample_plan(date(2026, 10, 5));
        let plan_id = plan.id;
        let session_id = plan.weeks[0].sessions[0].id;
        PlanRepo::save(&db, &plan).expect("save");

        let row = SessionScoreRow {
            session_id,
            formula_version: 1,
            score: 82.0,
            tss: 74.5,
            intensity_factor: 0.92,
            duration_s: 2_700,
            load: 68.0,
            planned_quality: true,
            executed_quality: true,
            adherence: Some(0.94),
            components: serde_json::json!({"duration": 0.4, "intensity": 0.6}),
        };
        ScoreRepo::save_session_score(&db, &row).expect("save");

        let loaded = ScoreRepo::session_score(&db, session_id)
            .expect("get")
            .expect("present");
        assert_eq!(loaded.score, 82.0);
        assert_eq!(loaded.components["intensity"], serde_json::json!(0.6));
        assert_eq!(
            ScoreRepo::session_scores_range(&db, date(2026, 10, 1), date(2026, 10, 31))
                .expect("range")
                .len(),
            1
        );

        assert_eq!(
            ScoreRepo::stale_session_scores(&db, 2)
                .expect("stale")
                .len(),
            1
        );
        assert_eq!(
            ScoreRepo::stale_session_scores(&db, 1)
                .expect("not stale")
                .len(),
            0
        );
        let _ = plan_id;
    }

    #[test]
    fn metric_snapshot_and_readiness_round_trip() {
        let db = Db::in_memory().expect("db");
        ScoreRepo::save_metric_snapshot(
            &db,
            &MetricSnapshotRow {
                date: date(2026, 10, 6),
                formula_version: 1,
                chronic_load: 42.1,
                acute_load: 48.0,
                training_stress_balance: -3.4,
                acwr: 1.14,
                monotony: 1.6,
                weekly_spike: 1.1,
                vo2max_estimate: Some(54.0),
                threshold_pace: Some(275.0),
                performance_index: Some(88.0),
                components: serde_json::json!({}),
            },
        )
        .expect("snapshot");

        let latest = ScoreRepo::latest_metric_snapshot(&db)
            .expect("latest")
            .expect("present");
        assert!((latest.acwr - 1.14).abs() < f64::EPSILON);
        assert_eq!(latest.threshold_pace, Some(275.0));

        ReadinessRepo::save_readiness(
            &db,
            &ReadinessRow {
                date: date(2026, 10, 6),
                formula_version: 1,
                score: 71.0,
                components: serde_json::json!({"sleep": 0.3}),
                inputs: serde_json::json!({"hrv": 62.0}),
            },
        )
        .expect("readiness");
        ReadinessRepo::save_injury_risk(
            &db,
            &InjuryRiskRow {
                date: date(2026, 10, 6),
                formula_version: 1,
                score: 34.0,
                band: "moderate".into(),
                drivers: serde_json::json!(["spike"]),
            },
        )
        .expect("injury");

        assert_eq!(
            ReadinessRepo::readiness(&db, date(2026, 10, 6))
                .expect("readiness")
                .expect("present")
                .score,
            71.0
        );
        assert_eq!(
            ReadinessRepo::injury_risk(&db, date(2026, 10, 6))
                .expect("injury")
                .expect("present")
                .band,
            "moderate"
        );
        assert_eq!(
            ReadinessRepo::readiness_range(&db, date(2026, 10, 1), date(2026, 10, 31))
                .expect("range")
                .len(),
            1
        );
    }

    #[test]
    fn feedback_and_sync_runs_are_queryable() {
        let db = Db::in_memory().expect("db");
        let account_id = account(&db);

        FeedbackRepo::insert(
            &db,
            &FeedbackRow {
                id: uuid::Uuid::now_v7(),
                session_id: None,
                date: date(2026, 10, 6),
                rpe: Some(7),
                mood: Some(4),
                legs: Some(2),
                motivation: Some(5),
                notes: "Legs felt light".into(),
            },
        )
        .expect("feedback");
        let feedback =
            FeedbackRepo::list_range(&db, date(2026, 10, 1), date(2026, 10, 31)).expect("list");
        assert_eq!(feedback.len(), 1);
        assert_eq!(feedback[0].rpe, Some(7));
        assert_eq!(feedback[0].notes, "Legs felt light");

        let run = SyncRunRepo::start(&db, account_id, "activities").expect("start");
        SyncRunRepo::finish(&db, run, "ok", 12, None).expect("finish");
        let failing = SyncRunRepo::start(&db, account_id, "health").expect("start");
        SyncRunRepo::finish(&db, failing, "error", 0, Some("timeout")).expect("finish");

        let runs = SyncRunRepo::recent(&db, 10).expect("recent");
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].status, "error", "newest first");
        assert_eq!(runs[0].error.as_deref(), Some("timeout"));
        assert_eq!(runs[1].records_upserted, 12);
    }

    #[test]
    fn has_any_data_reflects_plans_and_activities() {
        let db = Db::in_memory().expect("db");
        assert!(!has_any_data(&db).expect("empty"));
        let plan = sample_plan(date(2026, 10, 5));
        PlanRepo::save(&db, &plan).expect("save");
        assert!(has_any_data(&db).expect("has plan"));
    }

    #[test]
    fn session_exists_answers_for_mcp_validation() {
        let db = Db::in_memory().expect("db");
        let plan = sample_plan(date(2026, 10, 5));
        let session_id = plan.weeks[2].sessions[1].id;
        PlanRepo::save(&db, &plan).expect("save");

        assert!(session_exists(&db, session_id).expect("exists"));
        assert!(!session_exists(&db, PlannedSessionId::new()).expect("missing"));
    }

    #[test]
    fn deleting_an_account_cascades_to_its_activities() {
        let db = Db::in_memory().expect("db");
        let account_id = account(&db);
        let activity = sample_activity(account_id);
        let activity_id = activity.id;
        ActivityRepo::upsert(&db, &activity).expect("upsert");

        db.conn()
            .execute(
                "DELETE FROM provider_account WHERE id = ?1",
                params![account_id.to_string()],
            )
            .expect("delete");
        assert!(
            ActivityRepo::get(&db, activity_id)
                .expect_err("activity should be gone")
                .to_string()
                .contains("activity")
        );
    }

    #[test]
    fn plan_constraints_are_persisted_with_the_athlete() {
        // The athlete snapshot carries the constraints' consequences rather than
        // the constraints themselves, so this asserts the snapshot survives.
        let db = Db::in_memory().expect("db");
        let mut plan = sample_plan(date(2026, 10, 5));
        plan.athlete.injury_flags = vec!["achilles".into()];
        plan.athlete.experience = runalytics_core::ExperienceLevel::Advanced;
        let plan_id = plan.id;
        PlanRepo::save(&db, &plan).expect("save");

        let loaded = PlanRepo::load(&db, plan_id).expect("load");
        assert_eq!(loaded.athlete.injury_flags, vec!["achilles".to_string()]);
        assert!(loaded.athlete.injury_constrained());
        let _ = PlanConstraints::default();
    }
}
