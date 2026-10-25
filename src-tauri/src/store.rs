use std::path::Path;
use std::sync::Mutex;

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{AppError, Result};
use crate::types::*;

/// SQLite-backed local cache of provider data + locally generated plans.
/// All timestamps are stored as unix seconds (UTC) or ISO date strings.
pub struct Store {
    pub conn: Mutex<Connection>,
}

const MIGRATIONS: &[(u32, &str)] = &[
    (
        1,
        r#"
    CREATE TABLE IF NOT EXISTS activities (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        provider TEXT NOT NULL,
        provider_record_id TEXT NOT NULL,
        sport TEXT NOT NULL,
        start_ts INTEGER NOT NULL,
        duration_s REAL NOT NULL,
        distance_m REAL NOT NULL,
        avg_hr INTEGER,
        max_hr INTEGER,
        avg_pace_s_km REAL,
        elevation_gain_m REAL,
        UNIQUE (provider, provider_record_id)
    );
    CREATE INDEX IF NOT EXISTS idx_activities_start ON activities (start_ts);

    CREATE TABLE IF NOT EXISTS trackpoints (
        activity_id INTEGER NOT NULL REFERENCES activities (id) ON DELETE CASCADE,
        ts INTEGER NOT NULL,
        lat REAL NOT NULL,
        lon REAL NOT NULL,
        hr INTEGER
    );
    CREATE INDEX IF NOT EXISTS idx_trackpoints_activity ON trackpoints (activity_id);

    CREATE TABLE IF NOT EXISTS health_daily (
        provider TEXT NOT NULL,
        day TEXT NOT NULL,
        steps INTEGER,
        avg_hr INTEGER,
        resting_hr INTEGER,
        stress_avg INTEGER,
        sleep_score INTEGER,
        sleep_min INTEGER,
        deep_min INTEGER,
        rem_min INTEGER,
        hrv_avg REAL,
        PRIMARY KEY (provider, day)
    );

    CREATE TABLE IF NOT EXISTS fitness_assessment (
        provider TEXT NOT NULL,
        day TEXT NOT NULL,
        vo2max REAL,
        threshold_sec_km REAL,
        predict_5k_s INTEGER,
        predict_10k_s INTEGER,
        predict_half_s INTEGER,
        predict_marathon_s INTEGER,
        PRIMARY KEY (provider, day)
    );

    CREATE TABLE IF NOT EXISTS training_load (
        provider TEXT NOT NULL,
        day TEXT NOT NULL,
        short_term REAL,
        long_term REAL,
        ratio REAL,
        comment TEXT,
        PRIMARY KEY (provider, day)
    );

    CREATE TABLE IF NOT EXISTS training_plans (
        id TEXT PRIMARY KEY,
        distance TEXT NOT NULL,
        target_date TEXT NOT NULL,
        status TEXT NOT NULL,
        weekly_frequency INTEGER NOT NULL,
        longrun_day INTEGER NOT NULL,
        increase TEXT NOT NULL,
        threshold_sec_km REAL NOT NULL,
        created_ts INTEGER NOT NULL,
        pushed_id TEXT
    );

    CREATE TABLE IF NOT EXISTS plan_workouts (
        id TEXT PRIMARY KEY,
        plan_id TEXT NOT NULL REFERENCES training_plans (id) ON DELETE CASCADE,
        day TEXT NOT NULL,
        kind TEXT NOT NULL,
        title TEXT NOT NULL,
        distance_m REAL,
        duration_min REAL,
        pace_lo_s_km INTEGER,
        pace_hi_s_km INTEGER,
        done INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX IF NOT EXISTS idx_workouts_day ON plan_workouts (day);

    CREATE TABLE IF NOT EXISTS fit_file_ledger (
        provider TEXT NOT NULL,
        provider_record_id TEXT NOT NULL,
        downloaded_ts INTEGER NOT NULL,
        PRIMARY KEY (provider, provider_record_id)
    );

    CREATE TABLE IF NOT EXISTS sync_state (
        provider TEXT PRIMARY KEY,
        last_sync_ts INTEGER
    );
    "#,
    ),
    (
        2,
        r#"
    -- Cleanup for the stale-plan bug: superseded/aborted plans kept their
    -- workouts in plan_workouts, so the calendar showed duplicates that
    -- could not be removed. Keep only the most recent in-flight plan and
    -- cascade-delete everything else (workouts included).
    DELETE FROM training_plans WHERE id NOT IN (
        SELECT id FROM training_plans
        WHERE status IN ('draft', 'active', 'pushed')
        ORDER BY created_ts DESC LIMIT 1
    );
    "#,
    ),
    (
        3,
        r#"
    -- Remember the weekly mileage a plan was generated from. Before this the
    -- baseline was computed, used once and thrown away, so the active plan
    -- could not say what volume it started from.
    ALTER TABLE training_plans ADD COLUMN baseline_weekly_km REAL;
    "#,
    ),
    (
        4,
        r#"
    -- Injury journal. Everything here is typed by the runner, never synced:
    -- a pain entry is one report of where it hurts and how badly, `started_on`
    -- is when the runner says it began (not when it was logged), and
    -- `resolved_on` is set once they confirm it went away. NULL resolved_on
    -- means still hurting, which is what the risk blend and the proposals key
    -- off — a long-healed injury must not inflate risk forever.
    CREATE TABLE IF NOT EXISTS pain_entries (
        id TEXT PRIMARY KEY,
        body_part TEXT NOT NULL,
        intensity INTEGER NOT NULL CHECK (intensity BETWEEN 0 AND 10),
        started_on TEXT NOT NULL,
        notes TEXT NOT NULL DEFAULT '',
        created_ts INTEGER NOT NULL,
        resolved_on TEXT,
        -- When the "is it gone yet?" follow-up was last asked, so it fires
        -- once per episode instead of on every visit.
        followup_ts INTEGER
    );
    CREATE INDEX IF NOT EXISTS idx_pain_started ON pain_entries (started_on);
    CREATE INDEX IF NOT EXISTS idx_pain_part ON pain_entries (body_part);

    -- Assistant thread, one conversation per body region, so the journal and
    -- the chat about it survive page reloads (nav is full document loads).
    CREATE TABLE IF NOT EXISTS pain_messages (
        id TEXT PRIMARY KEY,
        body_part TEXT NOT NULL,
        role TEXT NOT NULL,
        content TEXT NOT NULL,
        ts INTEGER NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_pain_msg_part ON pain_messages (body_part);

    -- Plan changes the app offers to make a pain settle. The edit set is
    -- stored as JSON so the offer is reviewable as a diff and applying it is
    -- deterministic; `signature` suppresses repeats of the same offer.
    CREATE TABLE IF NOT EXISTS plan_proposals (
        id TEXT PRIMARY KEY,
        body_part TEXT NOT NULL,
        signature TEXT NOT NULL,
        summary TEXT NOT NULL,
        reason TEXT NOT NULL,
        payload TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'pending',
        created_ts INTEGER NOT NULL,
        acted_ts INTEGER,
        dismissed_reason TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_proposal_status ON plan_proposals (status);
    "#,
    ),
    (
        5,
        r#"
    CREATE TABLE IF NOT EXISTS calendar_completions (
        workout_id TEXT PRIMARY KEY,
        activity_id INTEGER NOT NULL UNIQUE REFERENCES activities(id) ON DELETE CASCADE,
        original_notes TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS calendar_revisions (
        uid TEXT PRIMARY KEY,
        content TEXT NOT NULL,
        sequence INTEGER NOT NULL,
        modified TEXT NOT NULL
    );
    "#,
    ),
    (
        6,
        r#"
    CREATE TABLE IF NOT EXISTS activity_scores (
        activity_id INTEGER PRIMARY KEY REFERENCES activities(id) ON DELETE CASCADE,
        load REAL,
        risk INTEGER NOT NULL
    );
    "#,
    ),
    (
        7,
        r#"
    -- The run-score snapshots are a derived cache: every one is recomputed
    -- from the run's own history by the next calendar refresh. The risk model
    -- gained the recent-sessions pressure (every run now moves the number,
    -- not just the excess over a comfortable week), so the frozen values —
    -- all computed by the old model, which pinned healthy runners at the 10
    -- floor — are cleared here and backfilled fresh on first refresh.
    DROP TABLE IF EXISTS activity_scores;
    CREATE TABLE activity_scores (
        activity_id INTEGER PRIMARY KEY REFERENCES activities(id) ON DELETE CASCADE,
        load REAL,
        risk INTEGER NOT NULL
    );
    "#,
    ),
    (
        8,
        r#"
    -- The risk delta changes meaning: from "change against the previous run's
    -- snapshot" (which compared two frozen days and said nothing about what
    -- the run itself cost) to "risk with this session minus risk without it"
    -- — the run's own impact, which is what the calendar and plan print.
    -- The rows are a derived cache, so they are cleared and recomputed with
    -- the new column on the next refresh.
    DROP TABLE IF EXISTS activity_scores;
    CREATE TABLE activity_scores (
        activity_id INTEGER PRIMARY KEY REFERENCES activities(id) ON DELETE CASCADE,
        load REAL,
        risk INTEGER NOT NULL,
        risk_delta INTEGER NOT NULL DEFAULT 0
    );
    "#,
    ),
    (
        9,
        r#"
    -- The snapshot's risk becomes the dashboard's *blended* headline as of
    -- the run's day (point-in-time formula averaged with the factor sum),
    -- instead of the factor-sum half alone — which is why a description
    -- could say 15% while the dashboard showed 12% for the same day. The
    -- cache is derived, so it is cleared and recomputed on first refresh.
    DELETE FROM activity_scores;
    "#,
    ),
    (
        10,
        r#"
    -- The assistant thread moves from one conversation per body region to
    -- one per pain entry, so each journal card carries its own chat. Legacy
    -- region-keyed messages are adopted by the newest entry for that region
    -- rather than dropped — the thread a runner already had must survive
    -- the upgrade.
    ALTER TABLE pain_messages ADD COLUMN entry_id TEXT;
    UPDATE pain_messages SET entry_id = (
        SELECT pe.id FROM pain_entries pe
        WHERE pe.body_part = pain_messages.body_part
        ORDER BY pe.started_on DESC, pe.created_ts DESC LIMIT 1
    ) WHERE entry_id IS NULL;
    CREATE INDEX IF NOT EXISTS idx_pain_msg_entry ON pain_messages (entry_id);
    "#,
    ),
];

/// `ALTER TABLE ... ADD COLUMN` for a column that a migration gained *after*
/// it had already run on existing installs. SQLite has no `IF NOT EXISTS` for
/// columns, and `PRAGMA table_info` returns no rows for a table that was never
/// created, so a missing table is skipped too — creating it stays the
/// migration's job.
fn add_column_if_missing(conn: &Connection, table: &str, column: &str, decl: &str) -> Result<bool> {
    // PRAGMA table_info returns no rows both when the column is absent and
    // when the table was never created, so the table itself is checked first:
    // ALTER TABLE on a missing table would abort opening the database, and
    // creating it stays the migration's job.
    let table_exists: bool = conn
        .prepare("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1")?
        .query_row(params![table], |r| r.get::<_, i64>(0))
        .map(|n| n > 0)?;
    if !table_exists {
        return Ok(false);
    }
    let exists: bool = conn
        .prepare(&format!("PRAGMA table_info({table})"))?
        .query_map([], |r| r.get::<_, String>(1))?
        .flatten()
        .any(|name| name == column);
    if exists {
        return Ok(false);
    }
    conn.execute(
        &format!("ALTER TABLE {table} ADD COLUMN {column} {decl}"),
        [],
    )?;
    Ok(true)
}

/// Best-effort backfill of `provider_sport_code` for activities synced before
/// the column existed, so their tracks become downloadable too. Matched on the
/// label `sport_label` wrote, in the same order that function decides, so the
/// two cannot disagree about which sport wins.
fn backfill_coros_sport_codes(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "UPDATE activities SET provider_sport_code = 100
          WHERE lower(sport) LIKE '%run%' OR lower(sport) LIKE '%jog%' OR lower(sport) LIKE '%walk%';
         UPDATE activities SET provider_sport_code = 200
          WHERE lower(sport) LIKE 'cycl%' OR lower(sport) LIKE '%ride%' OR lower(sport) LIKE '%bike%';
         UPDATE activities SET provider_sport_code = 300
          WHERE lower(sport) LIKE '%swim%';
         UPDATE activities SET provider_sport_code = 402
          WHERE lower(sport) LIKE '%strength%' OR lower(sport) LIKE '%gym%';
         UPDATE activities SET provider_sport_code = 104
          WHERE lower(sport) LIKE 'hik%';",
    )?;
    Ok(())
}

fn parse_date(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| AppError::Other(format!("bad date '{s}': {e}")))
}

/// Date parsing for command arguments (YYYY-MM-DD from the frontend).
pub fn parse_date_pub(s: &str) -> Result<NaiveDate> {
    parse_date(s)
}

/// Row-closure variant: query_map closures must fail with rusqlite::Error.
fn parse_date_row(s: &str) -> std::result::Result<NaiveDate, rusqlite::Error> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

fn ts_to_dt(ts: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(ts, 0).single().unwrap_or_else(Utc::now)
}

/// SQL predicate selecting running sessions. The `sport` column holds whatever
/// the ingest path labelled the activity: COROS record headers map to
/// `"run"`/`"ride"`/`"swim"`/`"strength"` (`coros.rs::sport_label`) while a FIT
/// session carries the Garmin enum, `"running"`. Trail/track runs arrive as
/// e.g. `"trail run"`, hence the LIKE fallback. Mileage that counts toward a
/// running plan must exclude the cycling and hiking rows.
const RUN_SQL: &str = "(lower(sport) IN ('run','running','jog','jogging') \
                        OR lower(sport) LIKE '%run%')";

/// SQL expression for the Monday (YYYY-MM-DD) of the week a row falls in.
/// SQLite has no ISO-week bucketing that survives a year boundary, so the
/// week is anchored arithmetically: step back 6 days then forward to the
/// nearest Monday, which maps every day Mon..Sun onto its own Monday.
const WEEK_START_SQL: &str = "date(start_ts, 'unixepoch', '-6 days', 'weekday 1')";

/// Unix seconds at 00:00 UTC on the Monday of the current week.
const THIS_MONDAY_TS_SQL: &str =
    "CAST(strftime('%s', date('now', '-6 days', 'weekday 1')) AS INTEGER)";

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let store = Store {
            conn: Mutex::new(conn),
        };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let version: u32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        for (v, sql) in MIGRATIONS {
            if version < *v {
                conn.execute_batch(sql)?;
                conn.pragma_update(None, "user_version", v)?;
            }
        }
        // Schema-drift repair. `followup_ts` was added to the pain_entries
        // definition in migration 4 *after* that migration had already run on
        // existing installs, and CREATE TABLE IF NOT EXISTS is a no-op once the
        // table exists — so those databases never gained the column while every
        // pain read and write names it. Logging pain, the journal, the body-map
        // tints, the dashboard's active pain and the injury-risk score all fail
        // together as a result. Bumping user_version would not help: the
        // databases in the wild are already at 4. This check is idempotent, so
        // it runs on every open and heals both cases.
        add_column_if_missing(&conn, "pain_entries", "followup_ts", "INTEGER")?;
        // COROS needs the numeric SportType back to fetch an activity's FIT
        // file (downloadActivityFitFiles requires labelId + sportType), and the
        // app's own sport label is lossy — "Hike" and "Cycling" collapse to a
        // lowercase word with no inverse, so those tracks could never be
        // downloaded. Added as an idempotent repair rather than a numbered
        // migration: SQLite has no ALTER TABLE ... ADD COLUMN IF NOT EXISTS, so
        // a migration would abort the whole open on a database whose
        // activities table is missing, which is exactly the drifted state the
        // repair path above exists to survive.
        if add_column_if_missing(&conn, "activities", "provider_sport_code", "INTEGER")? {
            backfill_coros_sport_codes(&conn)?;
        }
        // What a plan is *for* (race / improvement / maintenance) decides its
        // shape, so it has to survive a restart. Added as a repair rather than
        // a numbered migration for the same reason as the two above, and with
        // a DEFAULT: rows written before kinds existed were all race builds,
        // and a NOT NULL default says that in the schema instead of leaving
        // NULL for every reader to interpret.
        add_column_if_missing(
            &conn,
            "training_plans",
            "kind",
            "TEXT NOT NULL DEFAULT 'race'",
        )?;
        // The plan's pacing memory: how far its quality-pace ranges have been
        // moved by accepted pacing offers, and from which day. Both are added
        // with a default rather than a NULL for the same reason as `kind`
        // above — a plan that predates the feature has had its ranges moved
        // zero seconds, and saying so in the schema keeps every reader from
        // having to interpret NULL.
        add_column_if_missing(
            &conn,
            "training_plans",
            "quality_offset_sec_km",
            "REAL NOT NULL DEFAULT 0",
        )?;
        add_column_if_missing(&conn, "training_plans", "quality_offset_from", "TEXT")?;
        // What generated an offer. Pain offers and pacing offers apply through
        // the same path but belong on different pages, so the distinction has
        // to survive a restart; a pacing offer also carries the offset to store
        // when it is accepted, since it changes the plan rather than its days.
        add_column_if_missing(
            &conn,
            "plan_proposals",
            "source",
            "TEXT NOT NULL DEFAULT 'pain'",
        )?;
        add_column_if_missing(&conn, "plan_proposals", "plan_offset_sec_km", "REAL")?;
        add_column_if_missing(&conn, "plan_proposals", "plan_offset_from", "TEXT")?;
        Ok(())
    }

    // ---- activities ----------------------------------------------------

    /// Insert an activity (and optional track points). Returns the row id when
    /// the activity was *not* known before, None when it already existed —
    /// callers report "new activities" off this, so the pre-existing check has
    /// to happen before the write: with the `DO UPDATE` below (which backfills
    /// the sport code onto rows synced before that column existed) both the
    /// insert and the update path report one row changed.
    pub fn upsert_activity(
        &self,
        provider: ProviderKind,
        rec: &crate::fit::DecodedActivity,
    ) -> Result<Option<i64>> {
        let conn = self.conn.lock().unwrap();
        let pace = if rec.distance_m > 0.0 && rec.duration_s > 0.0 {
            Some(rec.duration_s / (rec.distance_m / 1000.0))
        } else {
            None
        };
        let known: i64 = conn.query_row(
            "SELECT COUNT(*) FROM activities WHERE provider=?1 AND provider_record_id=?2",
            params![provider_name(provider), rec.record_id],
            |r| r.get(0),
        )?;
        conn.execute(
            "INSERT INTO activities (provider, provider_record_id, sport, start_ts, duration_s,
                distance_m, avg_hr, max_hr, avg_pace_s_km, elevation_gain_m, provider_sport_code)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
             ON CONFLICT (provider, provider_record_id) DO UPDATE SET
                provider_sport_code = COALESCE(excluded.provider_sport_code, provider_sport_code)",
            params![
                provider_name(provider),
                rec.record_id,
                rec.sport,
                rec.start_ts,
                rec.duration_s,
                rec.distance_m,
                rec.avg_hr,
                rec.max_hr,
                pace,
                rec.elevation_gain_m,
                rec.provider_sport_code
            ],
        )?;
        let id: Option<i64> = conn
            .query_row(
                "SELECT id FROM activities WHERE provider=?1 AND provider_record_id=?2",
                params![provider_name(provider), rec.record_id],
                |r| r.get(0),
            )
            .optional()?;
        let Some(id) = id else {
            return Ok(None);
        };
        // Track points attach to rows that already existed too — this is how
        // the FIT download path fills in a track for a record first seen as a
        // summary row.
        if !rec.points.is_empty() {
            let existing: i64 = conn.query_row(
                "SELECT COUNT(*) FROM trackpoints WHERE activity_id=?1",
                params![id],
                |r| r.get(0),
            )?;
            if existing == 0 {
                for p in &rec.points {
                    conn.execute(
                        "INSERT INTO trackpoints (activity_id, ts, lat, lon, hr) VALUES (?1,?2,?3,?4,?5)",
                        params![id, p.ts, p.lat, p.lon, p.hr],
                    )?;
                }
            }
        }
        if known > 0 {
            return Ok(None);
        }
        Ok(Some(id))
    }

    /// Convenience for callers that track decode state on the activity row
    /// itself (the FIT ledger is the source of truth during sync).
    #[allow(dead_code)]
    pub fn activity_has_track(&self, provider: ProviderKind, record_id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM activities a
             JOIN trackpoints t ON t.activity_id = a.id
             WHERE a.provider=?1 AND a.provider_record_id=?2",
            params![provider_name(provider), record_id],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    pub fn list_activities(&self, limit: i64) -> Result<Vec<Activity>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, provider, provider_record_id, sport, start_ts, duration_s, distance_m,
                    avg_hr, max_hr, avg_pace_s_km, elevation_gain_m,
                    (SELECT COUNT(*) FROM trackpoints t WHERE t.activity_id = a.id) > 0
             FROM activities a ORDER BY start_ts DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, f64>(5)?,
                r.get::<_, f64>(6)?,
                r.get::<_, Option<i64>>(7)?,
                r.get::<_, Option<i64>>(8)?,
                r.get::<_, Option<f64>>(9)?,
                r.get::<_, Option<f64>>(10)?,
                r.get::<_, bool>(11)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, provider, rid, sport, ts, dur, dist, ahr, mhr, pace, elev, has_track) = row?;
            out.push(Activity {
                id,
                provider: provider_from(&provider),
                provider_record_id: rid,
                sport,
                start_time: ts_to_dt(ts),
                duration_s: dur,
                distance_m: dist,
                avg_hr: ahr,
                max_hr: mhr,
                avg_pace_s_km: pace,
                elevation_gain_m: elev,
                has_track,
            });
        }
        Ok(out)
    }

    /// Total running distance (m) per calendar week for the last `weeks`
    /// *complete* weeks, newest first, keyed by the week's Monday
    /// (`YYYY-MM-DD`). The in-progress week is excluded on purpose: including
    /// it would understate the baseline on a Monday and overstate nothing on a
    /// Sunday, so the plan would ramp from a number that moves with the
    /// calendar rather than with the runner.
    ///
    /// Weeks with no synced activity are absent from the result, so callers
    /// divide by `volumes.len()` — the number of weeks actually observed.
    ///
    /// Used by scoring and the plan engine baseline.
    pub fn weekly_volumes(&self, weeks: i64) -> Result<Vec<(String, f64)>> {
        let conn = self.conn.lock().unwrap();
        // NOTE: SQLite has no "NNN weeks" time modifier — `'-5 weeks'` makes
        // the whole expression NULL, which silently emptied this query and
        // left every plan built from a hardcoded fallback baseline. Weeks are
        // stepped in days instead.
        let days = weeks.max(1) * 7;
        let sql = format!(
            "SELECT {WEEK_START_SQL} wk, SUM(distance_m)
             FROM activities
             WHERE {RUN_SQL}
               AND start_ts >= CAST(strftime('%s', date('now', '-6 days', 'weekday 1'), '-{days} days') AS INTEGER)
               AND start_ts < {THIS_MONDAY_TS_SQL}
             GROUP BY wk ORDER BY wk DESC LIMIT {weeks}"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?)))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Same as [`weekly_volumes`](Self::weekly_volumes) but anchored on
    /// `as_of` instead of today: the `weeks` complete weeks strictly before
    /// the Monday of `as_of`'s week, newest first. Lets the dashboard
    /// re-evaluate as-of scoring for a past day without leaking later weeks
    /// into the result.
    pub fn weekly_volumes_as_of(&self, as_of: NaiveDate, weeks: i64) -> Result<Vec<(String, f64)>> {
        use chrono::Datelike;
        let conn = self.conn.lock().unwrap();
        let weeks = weeks.max(1);
        // Monday of as_of's week — mirrors WEEK_START_SQL's bucketing, so a
        // week only counts once it has finished before the anchor day.
        let monday = as_of - chrono::Duration::days(as_of.weekday().num_days_from_monday() as i64);
        let monday_ts = monday.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
        // SQLite has no "NNN weeks" modifier — step in days (see weekly_volumes).
        let from_ts = (monday - chrono::Duration::days(weeks * 7))
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();
        let mut stmt = conn.prepare(&format!(
            "SELECT {WEEK_START_SQL} wk, SUM(distance_m)
             FROM activities
             WHERE {RUN_SQL}
               AND start_ts >= ?1
               AND start_ts < ?2
             GROUP BY wk ORDER BY wk DESC LIMIT ?3"
        ))?;
        let rows = stmt.query_map(params![from_ts, monday_ts, weeks], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?))
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Mileage figures for the create-plan form and the plan's baseline.
    /// All distances are metres, matching the rest of the store.
    pub fn mileage_stats(&self) -> Result<MileageStats> {
        // Each query takes the connection lock in its own scope: the lock is
        // not re-entrant, so holding it across a call to weekly_volumes below
        // would deadlock.
        let (total_m, this_week_m, sessions, last_sync_ts) = {
            let conn = self.conn.lock().unwrap();
            let (total, this_week, n): (Option<f64>, Option<f64>, i64) = conn.query_row(
                &format!(
                    "SELECT SUM(distance_m),
                            SUM(CASE WHEN start_ts >= {THIS_MONDAY_TS_SQL}
                                     THEN distance_m END),
                            COUNT(*)
                     FROM activities WHERE {RUN_SQL}"
                ),
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?;
            let last_sync: Option<i64> = conn
                .query_row("SELECT MAX(last_sync_ts) FROM sync_state", [], |r| {
                    r.get::<_, Option<i64>>(0)
                })
                .optional()?
                .flatten();
            (total.unwrap_or(0.0), this_week.unwrap_or(0.0), n, last_sync)
        };
        let weeks = self.weekly_volumes(4)?;
        let weekly_avg_m = if weeks.is_empty() {
            None
        } else {
            Some(weeks.iter().map(|(_, d)| *d).sum::<f64>() / weeks.len() as f64)
        };
        Ok(MileageStats {
            weekly_avg_m,
            this_week_m,
            total_m,
            sessions,
            weeks_observed: weeks.len() as i64,
            last_sync: last_sync_ts.map(ts_to_dt),
        })
    }

    /// Total distance (m) RUN within a window `[from_days, to_days)` days
    /// ago. `to_days = 0` means "up to now". Run-only like every other volume
    /// the plan engine reads: a 90 km spin is not a volume spike.
    pub fn distance_window(&self, from_days: i64, to_days: i64) -> Result<f64> {
        let conn = self.conn.lock().unwrap();
        let total: Option<f64> = conn.query_row(
            &format!(
                "SELECT SUM(distance_m) FROM activities
                 WHERE {RUN_SQL}
                   AND start_ts >= strftime('%s','now', ?1)
                   AND start_ts < strftime('%s','now', ?2)"
            ),
            params![format!("-{from_days} days"), format!("-{to_days} days")],
            |r| r.get(0),
        )?;
        Ok(total.unwrap_or(0.0))
    }

    /// Aggregated activity stats for the last `days` days.
    pub fn range_stats(&self, days: i64) -> Result<RangeStats> {
        let conn = self.conn.lock().unwrap();
        let row = conn.query_row(
            "SELECT COALESCE(SUM(distance_m),0), COALESCE(SUM(duration_s),0), COUNT(*),
                    AVG(avg_hr)
             FROM activities
             WHERE start_ts >= strftime('%s','now', ?1)",
            params![format!("-{days} days")],
            |r| {
                Ok((
                    r.get::<_, f64>(0)?,
                    r.get::<_, f64>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, Option<f64>>(3)?,
                ))
            },
        )?;
        let (distance_m, duration_s, sessions, avg_hr) = row;
        let avg_pace_s_km = if distance_m > 0.0 {
            Some(duration_s / (distance_m / 1000.0))
        } else {
            None
        };
        Ok(RangeStats {
            distance_m,
            duration_s,
            sessions,
            avg_pace_s_km,
            avg_hr: avg_hr.map(|h| h.round() as i64),
        })
    }

    // ---- health / fitness / load -----------------------------------------

    pub fn upsert_health(&self, provider: ProviderKind, h: &HealthDaily) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO health_daily (provider, day, steps, avg_hr, resting_hr, stress_avg,
                sleep_score, sleep_min, deep_min, rem_min, hrv_avg)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
             ON CONFLICT (provider, day) DO UPDATE SET
                steps=COALESCE(excluded.steps, steps),
                avg_hr=COALESCE(excluded.avg_hr, avg_hr),
                resting_hr=COALESCE(excluded.resting_hr, resting_hr),
                stress_avg=COALESCE(excluded.stress_avg, stress_avg),
                sleep_score=COALESCE(excluded.sleep_score, sleep_score),
                sleep_min=COALESCE(excluded.sleep_min, sleep_min),
                deep_min=COALESCE(excluded.deep_min, deep_min),
                rem_min=COALESCE(excluded.rem_min, rem_min),
                hrv_avg=COALESCE(excluded.hrv_avg, hrv_avg)",
            params![
                provider_name(provider),
                h.date.format("%Y-%m-%d").to_string(),
                h.steps,
                h.avg_hr,
                h.resting_hr,
                h.stress_avg,
                h.sleep_score,
                h.sleep_min,
                h.deep_min,
                h.rem_min,
                h.hrv_avg
            ],
        )?;
        Ok(())
    }

    pub fn recent_health(&self, days: i64) -> Result<Vec<HealthDaily>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT day, steps, avg_hr, resting_hr, stress_avg, sleep_score, sleep_min,
                    deep_min, rem_min, hrv_avg
             FROM health_daily
             WHERE day >= strftime('%Y-%m-%d','now', ?1)
             ORDER BY day DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![format!("-{days} days"), days], |r| {
            Ok(HealthDaily {
                date: parse_date_row(&r.get::<_, String>(0)?)?,
                steps: r.get(1)?,
                avg_hr: r.get(2)?,
                resting_hr: r.get(3)?,
                stress_avg: r.get(4)?,
                sleep_score: r.get(5)?,
                sleep_min: r.get(6)?,
                deep_min: r.get(7)?,
                rem_min: r.get(8)?,
                hrv_avg: r.get(9)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn upsert_fitness(&self, provider: ProviderKind, f: &FitnessAssessment) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO fitness_assessment (provider, day, vo2max, threshold_sec_km,
                predict_5k_s, predict_10k_s, predict_half_s, predict_marathon_s)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
             ON CONFLICT (provider, day) DO UPDATE SET
                vo2max=excluded.vo2max, threshold_sec_km=excluded.threshold_sec_km,
                predict_5k_s=excluded.predict_5k_s, predict_10k_s=excluded.predict_10k_s,
                predict_half_s=excluded.predict_half_s,
                predict_marathon_s=excluded.predict_marathon_s",
            params![
                provider_name(provider),
                f.date.format("%Y-%m-%d").to_string(),
                f.vo2max,
                f.threshold_sec_km,
                f.predict_5k_s,
                f.predict_10k_s,
                f.predict_half_s,
                f.predict_marathon_s
            ],
        )?;
        Ok(())
    }

    pub fn latest_fitness(&self) -> Result<Option<FitnessAssessment>> {
        let conn = self.conn.lock().unwrap();
        let row = conn.query_row(
            "SELECT day, vo2max, threshold_sec_km, predict_5k_s, predict_10k_s,
                    predict_half_s, predict_marathon_s
             FROM fitness_assessment ORDER BY day DESC LIMIT 1",
            [],
            |r| {
                Ok(FitnessAssessment {
                    date: parse_date_row(&r.get::<_, String>(0)?)?,
                    vo2max: r.get(1)?,
                    threshold_sec_km: r.get(2)?,
                    predict_5k_s: r.get(3)?,
                    predict_10k_s: r.get(4)?,
                    predict_half_s: r.get(5)?,
                    predict_marathon_s: r.get(6)?,
                })
            },
        );
        match row.optional()? {
            Some(f) => Ok(Some(f)),
            None => Ok(None),
        }
    }

    pub fn upsert_training_load(&self, provider: ProviderKind, l: &TrainingLoad) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO training_load (provider, day, short_term, long_term, ratio, comment)
             VALUES (?1,?2,?3,?4,?5,?6)
             ON CONFLICT (provider, day) DO UPDATE SET
                short_term=excluded.short_term, long_term=excluded.long_term,
                ratio=excluded.ratio, comment=excluded.comment",
            params![
                provider_name(provider),
                l.date.format("%Y-%m-%d").to_string(),
                l.short_term,
                l.long_term,
                l.ratio,
                l.comment
            ],
        )?;
        Ok(())
    }

    pub fn latest_training_load(&self) -> Result<Option<TrainingLoad>> {
        let conn = self.conn.lock().unwrap();
        let row = conn.query_row(
            "SELECT day, short_term, long_term, ratio, comment
             FROM training_load ORDER BY day DESC LIMIT 1",
            [],
            |r| {
                Ok(TrainingLoad {
                    date: parse_date_row(&r.get::<_, String>(0)?)?,
                    short_term: r.get(1)?,
                    long_term: r.get(2)?,
                    ratio: r.get(3)?,
                    comment: r.get(4)?,
                })
            },
        );
        Ok(row.optional()?)
    }

    // ---- plans -----------------------------------------------------------

    pub fn calendar_completions(&self) -> Result<Vec<(String, i64, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT workout_id, activity_id, original_notes FROM calendar_completions")?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn complete_calendar_workout(
        &self,
        workout_id: &str,
        activity_id: i64,
        notes: &str,
    ) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        tx.execute("INSERT INTO calendar_completions (workout_id, activity_id, original_notes) VALUES (?1, ?2, ?3)", params![workout_id, activity_id, notes])?;
        tx.execute(
            "UPDATE plan_workouts SET done=1 WHERE id=?1",
            params![workout_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn set_completion_notes(&self, workout_id: &str, notes: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE calendar_completions SET original_notes=?2 WHERE workout_id=?1",
            params![workout_id, notes],
        )?;
        Ok(())
    }

    // ---- run score snapshots ----------------------------------------------

    /// Every stored run snapshot, oldest run first — the order the calendar
    /// needs to look a snapshot up by activity. The 4th element is the run's
    /// own risk impact: risk with the session minus risk without it.
    pub fn activity_scores(
        &self,
    ) -> Result<Vec<(i64, Option<f64>, u32, i64)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT s.activity_id, s.load, s.risk, s.risk_delta
             FROM activity_scores s JOIN activities a ON a.id = s.activity_id
             ORDER BY a.start_ts",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get::<_, i64>(2)? as u32,
                row.get::<_, i64>(3)?,
            ))
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    /// Record a run's snapshot once; a later refresh never overwrites it, so
    /// the calendar keeps saying what the numbers were when the run landed
    /// rather than what a later health sync would make them look like.
    pub fn insert_activity_score(
        &self,
        activity_id: i64,
        load: Option<f64>,
        risk: u32,
        risk_delta: i64,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO activity_scores (activity_id, load, risk, risk_delta) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(activity_id) DO NOTHING",
            params![activity_id, load, risk as i64, risk_delta],
        )?;
        Ok(())
    }

    pub fn calendar_revision(&self, uid: &str, content: &str) -> Result<(i64, String)> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO calendar_revisions (uid, content, sequence, modified) VALUES (?1, ?2, 0, ?3)
             ON CONFLICT(uid) DO UPDATE SET content=excluded.content, sequence=sequence+1, modified=excluded.modified
             WHERE calendar_revisions.content != excluded.content",
            params![uid, content, Utc::now().format("%Y%m%dT%H%M%SZ").to_string()],
        )?;
        Ok(conn.query_row(
            "SELECT sequence, modified FROM calendar_revisions WHERE uid=?1",
            params![uid],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?)
    }

    pub fn insert_plan(&self, p: &TrainingPlan) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO training_plans (id, distance, target_date, status, weekly_frequency,
                longrun_day, increase, threshold_sec_km, created_ts, pushed_id, baseline_weekly_km,
                kind, quality_offset_sec_km, quality_offset_from)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![
                p.id,
                distance_name(p.distance),
                p.target_date.format("%Y-%m-%d").to_string(),
                status_name(p.status),
                p.weekly_frequency,
                p.longrun_day,
                increase_name(p.increase),
                p.threshold_sec_km,
                p.created_at.timestamp(),
                p.pushed_id,
                p.baseline_weekly_km,
                plan_kind_name(p.kind),
                p.quality_offset_sec_km,
                p.quality_offset_from
                    .map(|d| d.format("%Y-%m-%d").to_string())
            ],
        )?;
        Ok(())
    }

    pub fn insert_workouts(&self, workouts: &[PlanWorkout]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        for w in workouts {
            conn.execute(
                "INSERT INTO plan_workouts (id, plan_id, day, kind, title, distance_m,
                    duration_min, pace_lo_s_km, pace_hi_s_km, done)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                params![
                    w.id,
                    w.plan_id,
                    w.date.format("%Y-%m-%d").to_string(),
                    kind_name(w.kind),
                    w.title,
                    w.distance_m,
                    w.duration_min,
                    w.pace_lo_s_km,
                    w.pace_hi_s_km,
                    w.done as i64
                ],
            )?;
        }
        Ok(())
    }

    /// The most recent plan that is not finished.
    pub fn current_plan(&self) -> Result<Option<(TrainingPlan, Vec<PlanWorkout>)>> {
        let plan = {
            let conn = self.conn.lock().unwrap();
            let row = conn.query_row(
                "SELECT id, distance, target_date, status, weekly_frequency, longrun_day,
                        increase, threshold_sec_km, created_ts, pushed_id, baseline_weekly_km,
                        kind, quality_offset_sec_km, quality_offset_from
                 FROM training_plans
                 WHERE status IN ('draft', 'active', 'pushed')
                 ORDER BY created_ts DESC LIMIT 1",
                [],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, u32>(4)?,
                        r.get::<_, u32>(5)?,
                        r.get::<_, String>(6)?,
                        r.get::<_, f64>(7)?,
                        r.get::<_, i64>(8)?,
                        r.get::<_, Option<String>>(9)?,
                        r.get::<_, Option<f64>>(10)?,
                        r.get::<_, String>(11)?,
                        r.get::<_, f64>(12)?,
                        r.get::<_, Option<String>>(13)?,
                    ))
                },
            );
            match row.optional()? {
                None => return Ok(None),
                Some((
                    id,
                    dist,
                    target,
                    status,
                    freq,
                    lrday,
                    inc,
                    thr,
                    created,
                    pushed,
                    base,
                    kind,
                    q_offset,
                    q_from,
                )) => TrainingPlan {
                    id,
                    distance: distance_from(&dist),
                    kind: plan_kind_from(&kind),
                    target_date: parse_date(&target)?,
                    status: status_from(&status),
                    weekly_frequency: freq,
                    longrun_day: lrday,
                    increase: increase_from(&inc),
                    threshold_sec_km: thr,
                    created_at: ts_to_dt(created),
                    pushed_id: pushed,
                    baseline_weekly_km: base,
                    quality_offset_sec_km: q_offset,
                    quality_offset_from: q_from.as_deref().map(parse_date).transpose()?,
                },
            }
        };
        let workouts = self.plan_workouts(plan.id.as_str())?;
        Ok(Some((plan, workouts)))
    }

    pub fn plan_workouts(&self, plan_id: &str) -> Result<Vec<PlanWorkout>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, plan_id, day, kind, title, distance_m, duration_min,
                    pace_lo_s_km, pace_hi_s_km, done
             FROM plan_workouts WHERE plan_id=?1 ORDER BY day",
        )?;
        let rows = stmt.query_map(params![plan_id], |r| {
            Ok(PlanWorkout {
                id: r.get(0)?,
                plan_id: r.get(1)?,
                date: parse_date_row(&r.get::<_, String>(2)?)?,
                kind: kind_from(&r.get::<_, String>(3)?),
                title: r.get(4)?,
                distance_m: r.get(5)?,
                duration_min: r.get(6)?,
                pace_lo_s_km: r.get(7)?,
                pace_hi_s_km: r.get(8)?,
                done: r.get::<_, i64>(9)? != 0,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn set_plan_status(&self, plan_id: &str, status: PlanStatus) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE training_plans SET status=?2 WHERE id=?1",
            params![plan_id, status_name(status)],
        )?;
        Ok(())
    }

    pub fn set_plan_pushed_id(&self, plan_id: &str, pushed_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE training_plans SET pushed_id=?2, status='pushed' WHERE id=?1",
            params![plan_id, pushed_id],
        )?;
        Ok(())
    }

    /// Remove a plan and (via ON DELETE CASCADE) all of its workouts.
    pub fn delete_plan(&self, plan_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM training_plans WHERE id=?1", params![plan_id])?;
        Ok(())
    }

    /// Install a plan as the only plan on this device, replacing whatever was
    /// there. Used by plan sync: an imported plan must not sit beside a stale
    /// one, because `current_plan` picks by `created_ts` and two in-flight
    /// plans would make the calendar show both.
    ///
    /// One transaction, so a failure part-way through cannot leave the device
    /// with the old plan deleted and the new one unwritten.
    pub fn replace_plan(&self, plan: &TrainingPlan, workouts: &[PlanWorkout]) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM training_plans", [])?;
        tx.execute(
            "INSERT INTO training_plans (id, distance, target_date, status, weekly_frequency,
                longrun_day, increase, threshold_sec_km, created_ts, pushed_id, baseline_weekly_km,
                kind, quality_offset_sec_km, quality_offset_from)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![
                plan.id,
                distance_name(plan.distance),
                plan.target_date.format("%Y-%m-%d").to_string(),
                status_name(plan.status),
                plan.weekly_frequency,
                plan.longrun_day,
                increase_name(plan.increase),
                plan.threshold_sec_km,
                plan.created_at.timestamp(),
                plan.pushed_id,
                plan.baseline_weekly_km,
                plan_kind_name(plan.kind),
                plan.quality_offset_sec_km,
                plan.quality_offset_from
                    .map(|d| d.format("%Y-%m-%d").to_string())
            ],
        )?;
        for w in workouts {
            tx.execute(
                "INSERT INTO plan_workouts (id, plan_id, day, kind, title, distance_m,
                    duration_min, pace_lo_s_km, pace_hi_s_km, done)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                params![
                    w.id,
                    w.plan_id,
                    w.date.format("%Y-%m-%d").to_string(),
                    kind_name(w.kind),
                    w.title,
                    w.distance_m,
                    w.duration_min,
                    w.pace_lo_s_km,
                    w.pace_hi_s_km,
                    w.done as i64
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Replace a workout's kind/title (used by adaptive downgrades).
    pub fn soften_workout(&self, id: &str, kind: WorkoutKind, title: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE plan_workouts SET kind=?2, title=?3 WHERE id=?1",
            params![id, kind_name(kind), title],
        )?;
        Ok(())
    }

    /// Rewrite a planned day. The date is part of the row: a caller that
    /// reschedules a session (or a test that re-dates one to simulate the
    /// past) must not have the change silently dropped while every other
    /// field lands.
    pub fn update_workout(&self, w: &PlanWorkout) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE plan_workouts SET day=?9, kind=?2, title=?3, distance_m=?4, duration_min=?5,
                pace_lo_s_km=?6, pace_hi_s_km=?7, done=?8 WHERE id=?1",
            params![
                w.id,
                kind_name(w.kind),
                w.title,
                w.distance_m,
                w.duration_min,
                w.pace_lo_s_km,
                w.pace_hi_s_km,
                w.done as i64,
                w.date.format("%Y-%m-%d").to_string()
            ],
        )?;
        Ok(())
    }

    pub fn workouts_between(&self, from: &str, to: &str) -> Result<Vec<PlanWorkout>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, plan_id, day, kind, title, distance_m, duration_min,
                    pace_lo_s_km, pace_hi_s_km, done
             FROM plan_workouts WHERE day BETWEEN ?1 AND ?2 ORDER BY day",
        )?;
        let rows = stmt.query_map(params![from, to], |r| {
            Ok(PlanWorkout {
                id: r.get(0)?,
                plan_id: r.get(1)?,
                date: parse_date_row(&r.get::<_, String>(2)?)?,
                kind: kind_from(&r.get::<_, String>(3)?),
                title: r.get(4)?,
                distance_m: r.get(5)?,
                duration_min: r.get(6)?,
                pace_lo_s_km: r.get(7)?,
                pace_hi_s_km: r.get(8)?,
                done: r.get::<_, i64>(9)? != 0,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    // ---- injury journal ------------------------------------------------------

    pub fn insert_pain_entry(&self, e: &PainEntry) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO pain_entries (id, body_part, intensity, started_on, notes,
                created_ts, resolved_on, followup_ts)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                e.id,
                e.body_part,
                e.intensity,
                e.started_on.format("%Y-%m-%d").to_string(),
                e.notes,
                e.created_at.timestamp(),
                e.resolved_on.map(|d| d.format("%Y-%m-%d").to_string()),
                e.followup_at.map(|t| t.timestamp())
            ],
        )?;
        Ok(())
    }

    /// Record that the "is it gone yet?" follow-up was asked for this episode.
    pub fn mark_pain_followup(&self, id: &str, when: DateTime<Utc>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE pain_entries SET followup_ts=?2 WHERE id=?1",
            params![id, when.timestamp()],
        )?;
        Ok(())
    }

    pub fn delete_pain_entry(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        // The entry's chat belongs to it, so it goes with the entry —
        // orphaned messages would be unreachable through any UI.
        conn.execute("DELETE FROM pain_messages WHERE entry_id=?1", params![id])?;
        conn.execute("DELETE FROM pain_entries WHERE id=?1", params![id])?;
        Ok(())
    }

    /// The entry a thread belongs to — `ask_assistant` needs its body region
    /// for the context it hands the model.
    pub fn pain_entry_by_id(&self, id: &str) -> Result<Option<PainEntry>> {
        let conn = self.conn.lock().unwrap();
        let row = conn
            .query_row(
                &format!(
                    "SELECT {cols} FROM pain_entries WHERE id=?1",
                    cols = Self::PAIN_COLS
                ),
                params![id],
                Self::pain_row,
            )
            .optional()?;
        Ok(row)
    }

    /// Mark an episode as settled on `day` — the runner said "it's gone".
    pub fn resolve_pain(&self, id: &str, day: NaiveDate) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE pain_entries SET resolved_on=?2 WHERE id=?1",
            params![id, day.format("%Y-%m-%d").to_string()],
        )?;
        Ok(())
    }

    fn pain_row(r: &rusqlite::Row<'_>) -> std::result::Result<PainEntry, rusqlite::Error> {
        Ok(PainEntry {
            id: r.get(0)?,
            body_part: r.get(1)?,
            intensity: r.get::<_, u32>(2)?,
            started_on: parse_date_row(&r.get::<_, String>(3)?)?,
            notes: r.get(4)?,
            created_at: ts_to_dt(r.get::<_, i64>(5)?),
            resolved_on: match r.get::<_, Option<String>>(6)? {
                Some(s) => Some(parse_date_row(&s)?),
                None => None,
            },
            followup_at: r.get::<_, Option<i64>>(7)?.map(ts_to_dt),
        })
    }

    const PAIN_COLS: &'static str =
        "id, body_part, intensity, started_on, notes, created_ts, resolved_on, followup_ts";

    /// Newest-reported first. Includes resolved episodes — the journal shows
    /// the whole history; risk queries filter on `resolved_on` themselves.
    pub fn list_pain_entries(&self, limit: i64) -> Result<Vec<PainEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&format!(
            "SELECT {cols} FROM pain_entries ORDER BY started_on DESC, created_ts DESC LIMIT ?1",
            cols = Self::PAIN_COLS
        ))?;
        let rows = stmt.query_map(params![limit], Self::pain_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// The entry the pain banner should show: the most recent *unresolved* one.
    pub fn latest_pain(&self) -> Result<Option<PainEntry>> {
        let conn = self.conn.lock().unwrap();
        let row = conn
            .query_row(
                &format!(
                    "SELECT {cols} FROM pain_entries WHERE resolved_on IS NULL
                     ORDER BY started_on DESC, created_ts DESC LIMIT 1",
                    cols = Self::PAIN_COLS
                ),
                [],
                Self::pain_row,
            )
            .optional()?;
        Ok(row)
    }

    /// Worst intensity among episodes still hurting on `day` (an episode hurts
    /// on day D when it started on or before D and has not resolved by D).
    /// The `resolved_on > day` arm matters: without it a long-healed injury
    /// would inflate the risk score forever.
    pub fn max_pain_intensity_since(&self, day: NaiveDate) -> Result<Option<(u32, String)>> {
        let conn = self.conn.lock().unwrap();
        let day_s = day.format("%Y-%m-%d").to_string();
        let row = conn
            .query_row(
                "SELECT intensity, body_part FROM pain_entries
                 WHERE started_on <= ?1
                   AND (resolved_on IS NULL OR resolved_on > ?1)
                 ORDER BY intensity DESC LIMIT 1",
                params![day_s],
                |r| Ok((r.get::<_, u32>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()?;
        Ok(row)
    }

    /// Per-day worst intensity over `[from, to]` for the calendar dots. Built
    /// in Rust rather than SQL: each entry owns the span from its `started_on`
    /// until the next entry for the same region starts, capped at its
    /// `resolved_on` and at today. A SQL window-function version would be
    /// unreadable and untestable.
    pub fn pain_days(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<(String, u32)>> {
        let entries = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare(&format!(
                "SELECT {cols} FROM pain_entries ORDER BY body_part, started_on",
                cols = Self::PAIN_COLS
            ))?;
            let rows = stmt.query_map([], Self::pain_row)?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            out
        };
        let today = Utc::now().date_naive();
        let mut by_region: std::collections::BTreeMap<&str, Vec<&PainEntry>> = Default::default();
        for e in &entries {
            by_region.entry(e.body_part.as_str()).or_default().push(e);
        }
        let mut per_day: std::collections::BTreeMap<NaiveDate, u32> = Default::default();
        for group in by_region.values() {
            for (i, e) in group.iter().enumerate() {
                let mut end = match group.get(i + 1) {
                    Some(next) => next.started_on - chrono::Duration::days(1),
                    None => today,
                };
                if let Some(r) = e.resolved_on {
                    // `resolved_on` is the day the pain is *gone*, so the last
                    // hurting day is the one before — matches the SQL filter in
                    // max_pain_intensity_since.
                    end = end.min(r - chrono::Duration::days(1));
                }
                end = end.min(today);
                if end < e.started_on {
                    continue; // resolved before the next entry — dead span
                }
                let lo = e.started_on.max(from);
                let hi = end.min(to);
                let mut d = lo;
                while d <= hi {
                    let slot = per_day.entry(d).or_insert(0);
                    *slot = (*slot).max(e.intensity);
                    d += chrono::Duration::days(1);
                }
            }
        }
        Ok(per_day
            .into_iter()
            .map(|(d, v)| (d.format("%Y-%m-%d").to_string(), v))
            .collect())
    }

    /// Worst intensity per region over the last `days` — tints the body map.
    /// Resolved episodes count inside their own window only (a healed region
    /// should fade out, not stay red).
    pub fn pain_by_region(&self, since_day: NaiveDate) -> Result<Vec<(String, u32)>> {
        let entries = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare(&format!(
                "SELECT {cols} FROM pain_entries WHERE started_on >= ?1",
                cols = Self::PAIN_COLS
            ))?;
            let rows = stmt.query_map(
                params![since_day.format("%Y-%m-%d").to_string()],
                Self::pain_row,
            )?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            out
        };
        let mut best: std::collections::BTreeMap<String, u32> = Default::default();
        for e in entries {
            // A resolved episode still tints its region while the window
            // covers it (the runner wants to see where trouble was); once it
            // falls out of the window it fades.
            if let Some(r) = e.resolved_on {
                if r < since_day {
                    continue;
                }
            }
            let slot = best.entry(e.body_part.clone()).or_insert(0);
            *slot = (*slot).max(e.intensity);
        }
        Ok(best.into_iter().collect())
    }

    // ---- assistant thread ----------------------------------------------------

    pub fn insert_pain_message(&self, m: &PainMessage) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO pain_messages (id, body_part, entry_id, role, content, ts)
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                m.id,
                m.body_part,
                m.entry_id,
                m.role,
                m.content,
                m.ts.timestamp()
            ],
        )?;
        Ok(())
    }

    /// One entry's thread, oldest first — chat reads top-down.
    pub fn list_pain_messages(&self, entry_id: &str) -> Result<Vec<PainMessage>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, body_part, entry_id, role, content, ts
             FROM pain_messages WHERE entry_id=?1 ORDER BY ts",
        )?;
        let rows = stmt.query_map(params![entry_id], Self::pain_msg_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn clear_pain_messages(&self, entry_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM pain_messages WHERE entry_id=?1",
            params![entry_id],
        )?;
        Ok(())
    }

    fn pain_msg_row(r: &rusqlite::Row<'_>) -> std::result::Result<PainMessage, rusqlite::Error> {
        Ok(PainMessage {
            id: r.get(0)?,
            body_part: r.get(1)?,
            entry_id: r.get(2)?,
            role: r.get(3)?,
            content: r.get(4)?,
            ts: ts_to_dt(r.get::<_, i64>(5)?),
        })
    }

    // ---- plan proposals -------------------------------------------------------

    pub fn insert_proposal(&self, p: &PlanProposal) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO plan_proposals (id, body_part, signature, summary, reason,
                payload, status, created_ts, acted_ts, dismissed_reason, source,
                plan_offset_sec_km, plan_offset_from)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![
                p.id,
                p.body_part,
                p.signature,
                p.summary,
                p.reason,
                serde_json::to_string(&p.edits)?,
                proposal_status_name(p.status),
                p.created_at.timestamp(),
                p.acted_at.map(|t| t.timestamp()),
                Option::<String>::None,
                proposal_source_name(p.source),
                p.plan_quality_offset_sec_km,
                p.plan_quality_offset_from
                    .map(|d| d.format("%Y-%m-%d").to_string())
            ],
        )?;
        Ok(())
    }

    fn proposal_row(r: &rusqlite::Row<'_>) -> std::result::Result<PlanProposal, rusqlite::Error> {
        let payload: String = r.get(5)?;
        let edits: Vec<WorkoutEdit> = serde_json::from_str(&payload).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(e))
        })?;
        Ok(PlanProposal {
            id: r.get(0)?,
            body_part: r.get(1)?,
            signature: r.get(2)?,
            summary: r.get(3)?,
            reason: r.get(4)?,
            edits,
            status: proposal_status_from(&r.get::<_, String>(6)?),
            created_at: ts_to_dt(r.get::<_, i64>(7)?),
            acted_at: r.get::<_, Option<i64>>(8)?.map(ts_to_dt),
            source: proposal_source_from(&r.get::<_, String>(9)?),
            plan_quality_offset_sec_km: r.get::<_, Option<f64>>(10)?,
            plan_quality_offset_from: r
                .get::<_, Option<String>>(11)?
                .as_deref()
                .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
        })
    }

    const PROPOSAL_COLS: &'static str = "id, body_part, signature, summary, reason, payload,
            status, created_ts, acted_ts, source, plan_offset_sec_km, plan_offset_from";

    /// The pending pain offer, which is what the dashboard banner and the
    /// Injuries page review. Pacing offers are deliberately excluded — they
    /// belong on the Plan page, beside the plan they would change.
    pub fn pending_proposal(&self) -> Result<Option<PlanProposal>> {
        let conn = self.conn.lock().unwrap();
        let row = conn
            .query_row(
                &format!(
                    "SELECT {} FROM plan_proposals WHERE status='pending' AND source='pain'
                     ORDER BY created_ts DESC LIMIT 1",
                    Self::PROPOSAL_COLS
                ),
                [],
                Self::proposal_row,
            )
            .optional()?;
        Ok(row)
    }

    /// The pending pacing offer for a plan, if one is waiting to be answered.
    pub fn pending_pacing_proposal(&self, plan_id: &str) -> Result<Option<PlanProposal>> {
        let conn = self.conn.lock().unwrap();
        let row = conn
            .query_row(
                &format!(
                    "SELECT {} FROM plan_proposals
                     WHERE status='pending' AND source='pacing' AND signature LIKE '%:' || ?1
                     ORDER BY created_ts DESC LIMIT 1",
                    Self::PROPOSAL_COLS
                ),
                params![plan_id],
                Self::proposal_row,
            )
            .optional()?;
        Ok(row)
    }

    /// True when a pacing offer for this plan is already unanswered, so the
    /// engine never stacks a second one on top of the first.
    pub fn has_pending_pacing(&self, plan_id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM plan_proposals
             WHERE status='pending' AND source='pacing' AND signature LIKE '%:' || ?1",
            params![plan_id],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    /// Store the plan's pacing offset once a pacing offer is accepted. Only the
    /// two offset columns change — the plan's days are untouched, because a
    /// pacing offer moves ranges rather than rewriting sessions.
    pub fn set_plan_quality_offset(
        &self,
        plan_id: &str,
        offset_sec_km: f64,
        from: NaiveDate,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE training_plans SET quality_offset_sec_km=?2, quality_offset_from=?3
             WHERE id=?1",
            params![plan_id, offset_sec_km, from.format("%Y-%m-%d").to_string()],
        )?;
        Ok(())
    }

    pub fn proposal_by_id(&self, id: &str) -> Result<Option<PlanProposal>> {
        let conn = self.conn.lock().unwrap();
        let row = conn
            .query_row(
                &format!(
                    "SELECT {} FROM plan_proposals WHERE id=?1",
                    Self::PROPOSAL_COLS
                ),
                params![id],
                Self::proposal_row,
            )
            .optional()?;
        Ok(row)
    }

    pub fn list_proposals(&self, limit: i64) -> Result<Vec<PlanProposal>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&format!(
            "SELECT {} FROM plan_proposals ORDER BY created_ts DESC LIMIT ?1",
            Self::PROPOSAL_COLS
        ))?;
        let rows = stmt.query_map(params![limit], Self::proposal_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn set_proposal_status(
        &self,
        id: &str,
        status: ProposalStatus,
        acted_ts: Option<DateTime<Utc>>,
        dismissed_reason: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE plan_proposals SET status=?2, acted_ts=?3, dismissed_reason=?4 WHERE id=?1",
            params![
                id,
                proposal_status_name(status),
                acted_ts.map(|t| t.timestamp()),
                dismissed_reason
            ],
        )?;
        Ok(())
    }

    /// When an identical offer was last dismissed — the suppression window in
    /// proposal::build reads this instead of nagging on every sync.
    pub fn last_dismissed(&self, signature: &str) -> Result<Option<DateTime<Utc>>> {
        let conn = self.conn.lock().unwrap();
        let ts = conn
            .query_row(
                "SELECT acted_ts FROM plan_proposals
                 WHERE signature=?1 AND status='dismissed'
                 ORDER BY acted_ts DESC LIMIT 1",
                params![signature],
                |r| r.get::<_, Option<i64>>(0),
            )
            .optional()?
            .flatten();
        Ok(ts.map(ts_to_dt))
    }

    /// True when a pending proposal already covers this plan window — the
    /// engine must not stack offers on top of an unanswered one.
    pub fn has_pending_signature_like(&self, region: &str, week_anchor: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM plan_proposals
             WHERE status='pending' AND source='pain' AND body_part=?1 AND signature LIKE '%' || ?2",
            params![region, week_anchor],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    // ---- FIT ledger / sync state -------------------------------------------

    /// True when a FIT file was already fetched for this record. Sync uses
    /// [`Store::activities_missing_track`], which answers the same question for
    /// a whole page of records in one query; this stays for single-record
    /// callers.
    #[allow(dead_code)]
    pub fn fit_downloaded(&self, provider: ProviderKind, record_id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM fit_file_ledger WHERE provider=?1 AND provider_record_id=?2",
            params![provider_name(provider), record_id],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    pub fn mark_fit_downloaded(&self, provider: ProviderKind, record_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO fit_file_ledger (provider, provider_record_id, downloaded_ts)
             VALUES (?1,?2,?3)",
            params![provider_name(provider), record_id, Utc::now().timestamp()],
        )?;
        Ok(())
    }

    /// Forget a ledger entry so the next sync retries that FIT download — used
    /// when a file came back truncated or failed to decode.
    pub fn forget_fit_downloaded(&self, provider: ProviderKind, record_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM fit_file_ledger WHERE provider=?1 AND provider_record_id=?2",
            params![provider_name(provider), record_id],
        )?;
        Ok(())
    }

    /// Activities still missing their FIT file, newest first, as
    /// `(provider_record_id, sport, provider_sport_code)`. The code is the raw
    /// COROS `SportType` captured at sync time; FIT download needs it whenever
    /// `labelId` is given.
    pub fn activities_missing_track(
        &self,
        limit: i64,
    ) -> Result<Vec<(String, String, Option<i64>)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT a.provider_record_id, a.sport, a.provider_sport_code
             FROM activities a
             LEFT JOIN fit_file_ledger l
               ON l.provider = a.provider AND l.provider_record_id = a.provider_record_id
             WHERE a.provider = 'coros' AND l.provider_record_id IS NULL
               AND NOT EXISTS (SELECT 1 FROM trackpoints t WHERE t.activity_id = a.id)
             ORDER BY a.start_ts DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<i64>>(2)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn set_last_sync(&self, provider: ProviderKind, ts: DateTime<Utc>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO sync_state (provider, last_sync_ts) VALUES (?1,?2)
             ON CONFLICT (provider) DO UPDATE SET last_sync_ts=excluded.last_sync_ts",
            params![provider_name(provider), ts.timestamp()],
        )?;
        Ok(())
    }

    pub fn last_sync(&self, provider: ProviderKind) -> Result<Option<DateTime<Utc>>> {
        let conn = self.conn.lock().unwrap();
        let ts: Option<i64> = conn
            .query_row(
                "SELECT last_sync_ts FROM sync_state WHERE provider=?1",
                params![provider_name(provider)],
                |r| r.get(0),
            )
            .optional()?;
        Ok(ts.map(ts_to_dt))
    }

    // ---- activity detail ---------------------------------------------------

    fn activity_from_row(r: &rusqlite::Row) -> std::result::Result<Activity, rusqlite::Error> {
        let provider: String = r.get(1)?;
        Ok(Activity {
            id: r.get(0)?,
            provider: provider_from(&provider),
            provider_record_id: r.get(2)?,
            sport: r.get(3)?,
            start_time: ts_to_dt(r.get(4)?),
            duration_s: r.get(5)?,
            distance_m: r.get(6)?,
            avg_hr: r.get(7)?,
            max_hr: r.get(8)?,
            avg_pace_s_km: r.get(9)?,
            elevation_gain_m: r.get(10)?,
            has_track: r.get(11)?,
        })
    }

    const ACTIVITY_COLS: &'static str = "SELECT a.id, a.provider, a.provider_record_id, a.sport, \
         a.start_ts, a.duration_s, a.distance_m, a.avg_hr, a.max_hr, a.avg_pace_s_km, \
         a.elevation_gain_m, \
         (SELECT COUNT(*) FROM trackpoints t WHERE t.activity_id = a.id) > 0 \
         FROM activities a";

    pub fn activity_by_id(&self, id: i64) -> Result<Option<Activity>> {
        let conn = self.conn.lock().unwrap();
        let sql = format!("{} WHERE a.id = ?1", Self::ACTIVITY_COLS);
        let row = conn
            .query_row(&sql, params![id], |r| Self::activity_from_row(r))
            .optional()?;
        Ok(row)
    }

    /// Track points for an activity, oldest first, sampled down to at most
    /// `max` points so the sheet stays light for long sessions.
    pub fn track_points(&self, activity_id: i64, max: i64) -> Result<Vec<TrackPoint>> {
        let conn = self.conn.lock().unwrap();
        let total: i64 = conn.query_row(
            "SELECT COUNT(*) FROM trackpoints WHERE activity_id=?1",
            params![activity_id],
            |r| r.get(0),
        )?;
        let every = (total / max.max(1)).max(1);
        let mut stmt = conn.prepare(
            "SELECT ts, lat, lon, hr FROM trackpoints
             WHERE activity_id=?1 AND rowid % ?2 = 0 ORDER BY ts",
        )?;
        let rows = stmt.query_map(params![activity_id, every], |r| {
            Ok(TrackPoint {
                ts: r.get(0)?,
                lat: r.get(1)?,
                lon: r.get(2)?,
                hr: r.get(3)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Health record for a specific day (any provider).
    pub fn health_on(&self, day: NaiveDate) -> Result<Option<HealthDaily>> {
        let conn = self.conn.lock().unwrap();
        let day_s = day.format("%Y-%m-%d").to_string();
        let row = conn
            .query_row(
                "SELECT steps, avg_hr, resting_hr, stress_avg, sleep_score, sleep_min,
                        deep_min, rem_min, hrv_avg
                 FROM health_daily WHERE day = ?1 LIMIT 1",
                params![day_s],
                |r| {
                    Ok(HealthDaily {
                        date: day,
                        steps: r.get(0)?,
                        avg_hr: r.get(1)?,
                        resting_hr: r.get(2)?,
                        stress_avg: r.get(3)?,
                        sleep_score: r.get(4)?,
                        sleep_min: r.get(5)?,
                        deep_min: r.get(6)?,
                        rem_min: r.get(7)?,
                        hrv_avg: r.get(8)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    /// Latest training load recorded on or before `day`.
    pub fn training_load_on_or_before(&self, day: NaiveDate) -> Result<Option<TrainingLoad>> {
        let conn = self.conn.lock().unwrap();
        let day_s = day.format("%Y-%m-%d").to_string();
        let row = conn
            .query_row(
                "SELECT day, short_term, long_term, ratio, comment
                 FROM training_load WHERE day <= ?1 ORDER BY day DESC LIMIT 1",
                params![day_s],
                |r| {
                    Ok(TrainingLoad {
                        date: parse_date_row(&r.get::<_, String>(0)?)?,
                        short_term: r.get(1)?,
                        long_term: r.get(2)?,
                        ratio: r.get(3)?,
                        comment: r.get(4)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    /// Daily load assessments in `[from_day, to_day]`, oldest→newest, for the
    /// training-status chart. One row per day: when several providers reported
    /// the same day, the later row wins (same tie-break the headline query's
    /// `ORDER BY day DESC LIMIT 1` implies).
    pub fn load_history(&self, from_day: NaiveDate, to_day: NaiveDate) -> Result<Vec<LoadPoint>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT day, short_term, long_term, ratio
             FROM training_load WHERE day >= ?1 AND day <= ?2 ORDER BY day ASC",
        )?;
        let rows = stmt.query_map(
            params![
                from_day.format("%Y-%m-%d").to_string(),
                to_day.format("%Y-%m-%d").to_string()
            ],
            |r| {
                let short_term: Option<f64> = r.get(1)?;
                let long_term: Option<f64> = r.get(2)?;
                let stored: Option<f64> = r.get(3)?;
                // COROS omits the ratio whenever the long-term load is still
                // zero; deriving it keeps the line unbroken, and yields None
                // (a gap) rather than a division by zero.
                let ratio = stored.or_else(|| match (short_term, long_term) {
                    (Some(s), Some(l)) if l > 0.0 => Some(s / l),
                    _ => None,
                });
                Ok(LoadPoint {
                    date: parse_date_row(&r.get::<_, String>(0)?)?,
                    short_term,
                    long_term,
                    ratio,
                })
            },
        )?;
        let mut by_day: std::collections::BTreeMap<NaiveDate, LoadPoint> = Default::default();
        for row in rows {
            let p = row?;
            by_day.insert(p.date, p);
        }
        Ok(by_day.into_values().collect())
    }

    /// Latest fitness assessment recorded on or before `day`.
    pub fn fitness_on_or_before(&self, day: NaiveDate) -> Result<Option<FitnessAssessment>> {
        let conn = self.conn.lock().unwrap();
        let day_s = day.format("%Y-%m-%d").to_string();
        let row = conn
            .query_row(
                "SELECT day, vo2max, threshold_sec_km, predict_5k_s, predict_10k_s,
                        predict_half_s, predict_marathon_s
                 FROM fitness_assessment WHERE day <= ?1 ORDER BY day DESC LIMIT 1",
                params![day_s],
                |r| {
                    Ok(FitnessAssessment {
                        date: parse_date_row(&r.get::<_, String>(0)?)?,
                        vo2max: r.get(1)?,
                        threshold_sec_km: r.get(2)?,
                        predict_5k_s: r.get(3)?,
                        predict_10k_s: r.get(4)?,
                        predict_half_s: r.get(5)?,
                        predict_marathon_s: r.get(6)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    /// Total distance (m) run with a day on each side (unix seconds, both
    /// ends inclusive of `from`, exclusive of `to`). Used for as-of windows.
    /// Run-only, mirroring [`distance_window`](Self::distance_window).
    pub fn distance_between(&self, from_ts: i64, to_ts: i64) -> Result<f64> {
        let conn = self.conn.lock().unwrap();
        let total: Option<f64> = conn.query_row(
            &format!(
                "SELECT SUM(distance_m) FROM activities
                 WHERE {RUN_SQL} AND start_ts >= ?1 AND start_ts < ?2"
            ),
            params![from_ts, to_ts],
            |r| r.get(0),
        )?;
        Ok(total.unwrap_or(0.0))
    }

    /// Run activities started within `[from_ts, to_ts)` (unix seconds),
    /// oldest first. Feeds the recent-sessions risk factor, which weighs each
    /// of the last week's runs by its own intensity — so it needs the per-run
    /// HR and pace columns, not just the distance sum.
    pub fn runs_between(&self, from_ts: i64, to_ts: i64) -> Result<Vec<Activity>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            &format!(
                "SELECT id, provider, provider_record_id, sport, start_ts, duration_s,
                        distance_m, avg_hr, max_hr, avg_pace_s_km, elevation_gain_m
                 FROM activities
                 WHERE {RUN_SQL} AND start_ts >= ?1 AND start_ts < ?2
                 ORDER BY start_ts"
            ),
        )?;
        let rows = stmt.query_map(params![from_ts, to_ts], |r| {
            Ok(Activity {
                id: r.get(0)?,
                provider: provider_from(&r.get::<_, String>(1)?),
                provider_record_id: r.get(2)?,
                sport: r.get(3)?,
                start_time: ts_to_dt(r.get::<_, i64>(4)?),
                duration_s: r.get(5)?,
                distance_m: r.get(6)?,
                avg_hr: r.get(7)?,
                max_hr: r.get(8)?,
                avg_pace_s_km: r.get(9)?,
                elevation_gain_m: r.get(10)?,
                has_track: false,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
    }

    /// Health records in `[from_day, to_day]`, newest first.
    pub fn health_between(
        &self,
        from_day: NaiveDate,
        to_day: NaiveDate,
    ) -> Result<Vec<HealthDaily>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT day, steps, avg_hr, resting_hr, stress_avg, sleep_score, sleep_min,
                    deep_min, rem_min, hrv_avg
             FROM health_daily WHERE day >= ?1 AND day <= ?2 ORDER BY day DESC",
        )?;
        let rows = stmt.query_map(
            params![
                from_day.format("%Y-%m-%d").to_string(),
                to_day.format("%Y-%m-%d").to_string()
            ],
            |r| {
                Ok(HealthDaily {
                    date: parse_date_row(&r.get::<_, String>(0)?)?,
                    steps: r.get(1)?,
                    avg_hr: r.get(2)?,
                    resting_hr: r.get(3)?,
                    stress_avg: r.get(4)?,
                    sleep_score: r.get(5)?,
                    sleep_min: r.get(6)?,
                    deep_min: r.get(7)?,
                    rem_min: r.get(8)?,
                    hrv_avg: r.get(9)?,
                })
            },
        )?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    // ---- heatmap -----------------------------------------------------------

    /// Sampled track points as (lat, lon) pairs for the heatmap layer.
    pub fn heat_points(&self, sample_every: i64) -> Result<Vec<HeatPoint>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT lat, lon FROM trackpoints WHERE rowid % ?1 = 0 ORDER BY ts")?;
        let rows = stmt.query_map(params![sample_every.max(1)], |r| {
            Ok(HeatPoint {
                lat: r.get(0)?,
                lon: r.get(1)?,
                weight: 1.0,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}

pub fn provider_name(p: ProviderKind) -> &'static str {
    match p {
        ProviderKind::Coros => "coros",
        ProviderKind::Garmin => "garmin",
    }
}

fn distance_name(d: RaceDistance) -> &'static str {
    match d {
        RaceDistance::K5 => "5K",
        RaceDistance::K10 => "10K",
        RaceDistance::K21 => "21K",
        RaceDistance::K42 => "42K",
    }
}

fn distance_from(s: &str) -> RaceDistance {
    match s {
        "5K" => RaceDistance::K5,
        "10K" => RaceDistance::K10,
        "21K" => RaceDistance::K21,
        _ => RaceDistance::K42,
    }
}

/// What a plan is built for. Rows written before `kind` existed default to
/// 'race' in the schema, so `plan_kind_from` only sees a name it doesn't know
/// if the column was edited by hand; falling back to Race keeps such a row
/// readable as the shape it almost certainly is.
fn plan_kind_name(k: PlanKind) -> &'static str {
    match k {
        PlanKind::Race => "race",
        PlanKind::Improvement => "improvement",
        PlanKind::Maintenance => "maintenance",
    }
}

fn plan_kind_from(s: &str) -> PlanKind {
    match s {
        "improvement" => PlanKind::Improvement,
        "maintenance" => PlanKind::Maintenance,
        _ => PlanKind::Race,
    }
}

fn status_name(s: PlanStatus) -> &'static str {
    match s {
        PlanStatus::Draft => "draft",
        PlanStatus::Active => "active",
        PlanStatus::Pushed => "pushed",
        PlanStatus::Completed => "completed",
        PlanStatus::Aborted => "aborted",
    }
}

fn status_from(s: &str) -> PlanStatus {
    match s {
        "draft" => PlanStatus::Draft,
        "active" => PlanStatus::Active,
        "pushed" => PlanStatus::Pushed,
        "completed" => PlanStatus::Completed,
        _ => PlanStatus::Aborted,
    }
}

fn increase_name(i: IncreaseMethod) -> &'static str {
    match i {
        IncreaseMethod::Conservative => "conservative",
        IncreaseMethod::Moderate => "moderate",
        IncreaseMethod::Ambitious => "ambitious",
    }
}

fn increase_from(s: &str) -> IncreaseMethod {
    match s {
        "conservative" => IncreaseMethod::Conservative,
        // "bullish" is the legacy name for "ambitious" in older plan rows.
        "ambitious" | "bullish" => IncreaseMethod::Ambitious,
        _ => IncreaseMethod::Moderate,
    }
}

fn kind_name(k: WorkoutKind) -> &'static str {
    match k {
        WorkoutKind::Rest => "rest",
        WorkoutKind::Easy => "easy",
        WorkoutKind::Tempo => "tempo",
        WorkoutKind::Intervals => "intervals",
        WorkoutKind::LongRun => "longrun",
        WorkoutKind::Race => "race",
    }
}

fn kind_from(s: &str) -> WorkoutKind {
    match s {
        "rest" => WorkoutKind::Rest,
        "tempo" => WorkoutKind::Tempo,
        "intervals" => WorkoutKind::Intervals,
        "longrun" => WorkoutKind::LongRun,
        "race" => WorkoutKind::Race,
        _ => WorkoutKind::Easy,
    }
}

fn proposal_status_name(s: ProposalStatus) -> &'static str {
    match s {
        ProposalStatus::Pending => "pending",
        ProposalStatus::Applied => "applied",
        ProposalStatus::Dismissed => "dismissed",
        ProposalStatus::Superseded => "superseded",
    }
}

fn proposal_status_from(s: &str) -> ProposalStatus {
    match s {
        "pending" => ProposalStatus::Pending,
        "applied" => ProposalStatus::Applied,
        "dismissed" => ProposalStatus::Dismissed,
        _ => ProposalStatus::Superseded,
    }
}

fn proposal_source_name(s: ProposalSource) -> &'static str {
    match s {
        ProposalSource::Pain => "pain",
        ProposalSource::Pacing => "pacing",
    }
}

fn proposal_source_from(s: &str) -> ProposalSource {
    match s {
        "pacing" => ProposalSource::Pacing,
        _ => ProposalSource::Pain,
    }
}

pub fn provider_from(s: &str) -> ProviderKind {
    match s {
        "garmin" => ProviderKind::Garmin,
        _ => ProviderKind::Coros,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Monday 00:00 UTC of the week containing `d` — mirrors WEEK_START_SQL.
    fn monday_of(d: NaiveDate) -> NaiveDate {
        use chrono::Datelike;
        d - chrono::Duration::days(d.weekday().num_days_from_monday() as i64)
    }

    fn store() -> Store {
        let path =
            std::env::temp_dir().join(format!("runalytics-test-{}.sqlite", uuid::Uuid::new_v4()));
        Store::open(&path).expect("open store")
    }

    fn run(store: &Store, day: NaiveDate, km: f64) {
        let ts = day.and_hms_opt(9, 0, 0).unwrap().and_utc().timestamp();
        let rec = crate::fit::DecodedActivity {
            record_id: format!("{day}-{km}"),
            sport: "run".into(),
            provider_sport_code: Some(100),
            start_ts: ts,
            duration_s: km * 300.0,
            distance_m: km * 1000.0,
            avg_hr: None,
            max_hr: None,
            elevation_gain_m: None,
            points: Vec::new(),
        };
        store
            .upsert_activity(ProviderKind::Coros, &rec)
            .expect("insert run");
    }

    fn ride(store: &Store, day: NaiveDate, km: f64) {
        let ts = day.and_hms_opt(9, 0, 0).unwrap().and_utc().timestamp();
        let rec = crate::fit::DecodedActivity {
            record_id: format!("{day}-ride"),
            sport: "cycling".into(),
            provider_sport_code: Some(200),
            start_ts: ts,
            duration_s: km * 120.0,
            distance_m: km * 1000.0,
            avg_hr: None,
            max_hr: None,
            elevation_gain_m: None,
            points: Vec::new(),
        };
        store
            .upsert_activity(ProviderKind::Coros, &rec)
            .expect("insert ride");
    }

    /// The plan baseline must be a real number. `weekly_volumes` once filtered
    /// with `'-N weeks'`, which SQLite rejects (no such modifier), so the
    /// expression yielded NULL, every row was filtered out, and every plan
    /// silently opened from the 15 km fallback.
    #[test]
    fn weekly_volumes_returns_real_recent_volume() {
        let s = store();
        let this_monday = monday_of(Utc::now().date_naive());
        for w in 1..=4 {
            run(&s, this_monday - chrono::Duration::days(7 * w), 20.0);
        }
        let volumes = s.weekly_volumes(4).expect("weekly_volumes");
        assert_eq!(volumes.len(), 4, "four complete weeks should be counted");
        // Magnitude, not just shape: a unit slip would hide behind a ratio.
        for (_, m) in &volumes {
            assert!(
                (m - 20_000.0).abs() < 1.0,
                "expected 20 km of metres, got {m}"
            );
        }
        assert_eq!(
            volumes[0].0,
            (this_monday - chrono::Duration::days(7))
                .format("%Y-%m-%d")
                .to_string()
        );
    }

    /// The readiness sparklines re-score every day of the range, and each day
    /// must see only the weeks that had finished *before* it. `weekly_volumes`
    /// is anchored on today, so reusing it for a past day would leak later
    /// weeks backwards and draw a trend line the runner never lived through.
    #[test]
    fn weekly_volumes_as_of_excludes_weeks_after_the_anchor() {
        let s = store();
        let this_monday = monday_of(Utc::now().date_naive());
        // Four complete weeks before this Monday, plus one after it (the
        // in-progress week) and one further future week for good measure.
        for w in 1..=4 {
            run(&s, this_monday - chrono::Duration::days(7 * w), 20.0);
        }
        run(&s, this_monday + chrono::Duration::days(3), 99.0);
        run(&s, this_monday + chrono::Duration::days(10), 99.0);

        // Anchor two weeks back: its own week is in progress then, so the
        // newest complete week is the one before it.
        let anchor = this_monday - chrono::Duration::days(14);
        let volumes = s
            .weekly_volumes_as_of(anchor, 8)
            .expect("weekly_volumes_as_of");
        assert_eq!(
            volumes.len(),
            2,
            "only the two weeks finished before the anchor's Monday count"
        );
        assert_eq!(
            volumes[0].0,
            (anchor - chrono::Duration::days(7))
                .format("%Y-%m-%d")
                .to_string(),
            "newest first"
        );
        // Magnitude pinned, and the 99 km weeks after the anchor must be absent.
        for (_, m) in &volumes {
            assert!((m - 20_000.0).abs() < 1.0, "expected 20 km, got {m}");
        }
    }

    /// The in-progress week is partial, so counting it would make the baseline
    /// drift with the day of the week the plan is generated on.
    #[test]
    fn weekly_volumes_excludes_the_in_progress_week() {
        let s = store();
        let this_monday = monday_of(Utc::now().date_naive());
        run(&s, this_monday, 9.0); // today-ish, inside the current week
        run(&s, this_monday - chrono::Duration::days(7), 20.0);
        let volumes = s.weekly_volumes(4).expect("weekly_volumes");
        assert_eq!(volumes.len(), 1, "only the completed week counts");
        assert!((volumes[0].1 - 20_000.0).abs() < 1.0);
    }

    /// A cycling trip must not set a running plan's baseline.
    #[test]
    fn mileage_counts_runs_and_not_other_sports() {
        let s = store();
        let this_monday = monday_of(Utc::now().date_naive());
        run(&s, this_monday - chrono::Duration::days(7), 10.0);
        ride(&s, this_monday - chrono::Duration::days(7), 90.0);
        ride(&s, this_monday - chrono::Duration::days(200), 40.0);

        let m = s.mileage_stats().expect("mileage_stats");
        let avg = m.weekly_avg_m.expect("one complete week");
        assert!(
            (avg - 10_000.0).abs() < 1.0,
            "weekly average should ignore the 90 km ride, got {avg}"
        );
        assert!(
            (m.total_m - 10_000.0).abs() < 1.0,
            "odometer counts runs only"
        );
        assert_eq!(m.weeks_observed, 1);
        assert_eq!(m.sessions, 1);
    }

    /// The volume-spike risk factor reads `distance_window`. It used to sum
    /// every sport, so a long weekend spin registered as a running volume
    /// spike and the plan got cut for a ride that never stressed the legs.
    #[test]
    fn distance_window_counts_runs_and_not_other_sports() {
        let s = store();
        let today = Utc::now().date_naive();
        run(&s, today - chrono::Duration::days(3), 12.0);
        ride(&s, today - chrono::Duration::days(2), 90.0);

        let metres = s.distance_window(7, 0).expect("distance_window");
        assert!(
            (metres - 12_000.0).abs() < 1.0,
            "7-day volume should ignore the 90 km ride, got {metres}"
        );

        let from = (today - chrono::Duration::days(7))
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();
        let to = (today + chrono::Duration::days(1))
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();
        let between = s.distance_between(from, to).expect("distance_between");
        assert!(
            (between - 12_000.0).abs() < 1.0,
            "as-of volume should ignore the ride too, got {between}"
        );
    }

    /// Nothing synced yet: the form must show an empty field rather than a
    /// fabricated number, and the odometer reads zero.
    #[test]
    fn mileage_stats_empty_history_abstains() {
        let s = store();
        let m = s.mileage_stats().expect("mileage_stats");
        assert!(m.weekly_avg_m.is_none(), "no weeks -> no average");
        assert_eq!(m.this_week_m, 0.0);
        assert_eq!(m.total_m, 0.0);
        assert_eq!(m.weeks_observed, 0);
        assert!(m.last_sync.is_none());
    }

    /// This week's running shows up as its own figure, separate from the
    /// completed-week average.
    #[test]
    fn mileage_stats_reports_current_week() {
        let s = store();
        let this_monday = monday_of(Utc::now().date_naive());
        run(&s, this_monday, 6.0);
        run(&s, this_monday - chrono::Duration::days(7), 20.0);
        let m = s.mileage_stats().expect("mileage_stats");
        assert!((m.this_week_m - 6_000.0).abs() < 1.0, "this week = 6 km");
        let avg = m.weekly_avg_m.expect("one complete week");
        assert!((avg - 20_000.0).abs() < 1.0, "average excludes this week");
    }

    /// The FIT download queue is driven by the ledger, not by a window of
    /// recent activities — it once scanned `list_activities(60)`, so an older
    /// record quietly fell off and never got a track. Each row must also carry
    /// the provider sport code, which `downloadActivityFitFiles` requires
    /// whenever a `labelId` is given.
    #[test]
    fn activities_missing_track_walks_the_ledger() {
        let s = store();
        let day = NaiveDate::from_ymd_opt(2026, 3, 2).unwrap();
        run(&s, day, 10.0);
        ride(&s, day - chrono::Duration::days(1), 40.0);

        let missing = s.activities_missing_track(10).expect("missing");
        assert_eq!(
            missing
                .iter()
                .map(|(id, sport, code)| (id.as_str(), sport.as_str(), *code))
                .collect::<Vec<_>>(),
            vec![
                // Newest first, and the sport code rides along.
                (format!("{day}-10").as_str(), "run", Some(100)),
                (
                    format!("{}-ride", day - chrono::Duration::days(1)).as_str(),
                    "cycling",
                    Some(200)
                ),
            ]
        );

        // Downloading one file takes it out of the queue; forgetting it puts
        // it back, which is how a truncated FIT gets retried.
        s.mark_fit_downloaded(ProviderKind::Coros, &format!("{day}-10"))
            .expect("mark");
        let missing = s.activities_missing_track(10).expect("missing");
        assert_eq!(missing.len(), 1, "only the ride is outstanding");
        s.forget_fit_downloaded(ProviderKind::Coros, &format!("{day}-10"))
            .expect("forget");
        assert_eq!(
            s.activities_missing_track(10).expect("missing").len(),
            2,
            "retried"
        );

        // A record whose track already landed needs no download even though
        // the ledger never saw it.
        let ts = day.and_hms_opt(9, 0, 0).unwrap().and_utc().timestamp();
        s.upsert_activity(
            ProviderKind::Coros,
            &crate::fit::DecodedActivity {
                record_id: "already-drawn".into(),
                sport: "run".into(),
                provider_sport_code: Some(100),
                start_ts: ts,
                duration_s: 600.0,
                distance_m: 2000.0,
                avg_hr: None,
                max_hr: None,
                elevation_gain_m: None,
                points: vec![
                    crate::fit::TrackPoint {
                        ts,
                        lat: 52.5,
                        lon: 13.4,
                        hr: None,
                    },
                    crate::fit::TrackPoint {
                        ts: ts + 60,
                        lat: 52.51,
                        lon: 13.41,
                        hr: None,
                    },
                ],
            },
        )
        .expect("insert tracked run");
        assert_eq!(
            s.activities_missing_track(10).expect("missing").len(),
            2,
            "a stored track is not a missing one"
        );

        // The limit is honoured, newest first.
        assert_eq!(s.activities_missing_track(1).expect("missing").len(), 1);
    }

    /// A record re-synced after its sport code was captured must not lose it,
    /// and one that arrives without a code must not wipe an existing one —
    /// otherwise the row becomes permanently undownloadable.
    #[test]
    fn resync_preserves_the_sport_code() {
        let s = store();
        let day = NaiveDate::from_ymd_opt(2026, 3, 2).unwrap();
        ride(&s, day, 40.0);
        let ts = day.and_hms_opt(9, 0, 0).unwrap().and_utc().timestamp();
        for code in [None, Some(200)] {
            s.upsert_activity(
                ProviderKind::Coros,
                &crate::fit::DecodedActivity {
                    record_id: format!("{day}-ride"),
                    sport: "cycling".into(),
                    provider_sport_code: code,
                    start_ts: ts,
                    duration_s: 4800.0,
                    distance_m: 40_000.0,
                    avg_hr: None,
                    max_hr: None,
                    elevation_gain_m: None,
                    points: Vec::new(),
                },
            )
            .expect("resync");
        }
        let missing = s.activities_missing_track(10).expect("missing");
        assert_eq!(missing[0].2, Some(200), "COALESCE keeps the known code");
    }

    /// End-to-end check against a copy of the developer's real synced data:
    /// migration 3 applies to an existing DB, the mileage figures are
    /// plausible, and a plan generated from them opens at that volume.
    /// Run with `cargo test -- --ignored --nocapture`.
    #[test]
    #[ignore = "reads the real app database"]
    fn real_db_mileage_and_plan_round_trip() {
        use chrono::Datelike;
        let home = match std::env::var("HOME") {
            Ok(h) => h,
            Err(_) => {
                println!("HOME unset — nothing to check");
                return;
            }
        };
        let src = std::path::PathBuf::from(home)
            .join("Library/Application Support/com.fradone.runalytics/runalytics.sqlite");
        if !src.exists() {
            println!("no real db at {src:?} — nothing to check");
            return;
        }
        let dst = std::env::temp_dir().join(format!("rk-real-{}.sqlite", std::process::id()));
        std::fs::copy(&src, &dst).expect("copy db");
        for ext in ["-wal", "-shm"] {
            let p = std::path::PathBuf::from(format!("{}{}", src.display(), ext));
            if p.exists() {
                let _ = std::fs::copy(
                    &p,
                    std::path::PathBuf::from(format!("{}{}", dst.display(), ext)),
                );
            }
        }

        let s = Store::open(&dst).expect("open (applies migrations)");
        let m = s.mileage_stats().expect("mileage_stats");
        let avg_km = m.weekly_avg_m.expect("real history has complete weeks") / 1000.0;
        println!("weekly_avg_km  = {avg_km:.1} over {} wk", m.weeks_observed);
        println!("this_week_km   = {:.1}", m.this_week_m / 1000.0);
        println!("total_km       = {:.1}", m.total_m / 1000.0);
        println!("sessions       = {}", m.sessions);
        println!("last_sync      = {:?}", m.last_sync);
        println!(
            "weeks          = {:?}",
            s.weekly_volumes(8)
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), (v / 1000.0) as i64))
                .collect::<Vec<_>>()
        );
        assert!(
            (5.0..200.0).contains(&avg_km),
            "implausible baseline {avg_km}"
        );
        assert!(m.total_m >= m.this_week_m);

        let req = crate::types::PlanRequest {
            distance: crate::types::RaceDistance::K21,
            kind: crate::types::PlanKind::Race,
            target_date: Utc::now().date_naive() + chrono::Duration::days(70),
            threshold_sec_km: 285.0,
            weekly_frequency: 4,
            longrun_day: 6,
            increase: crate::types::IncreaseMethod::Moderate,
            current_weekly_km: None,
        };
        let (plan, workouts) = crate::plan::engine::generate(&req, avg_km).unwrap();
        s.insert_plan(&plan).unwrap();
        s.insert_workouts(&workouts).unwrap();
        let (read, _) = s.current_plan().unwrap().expect("plan round-trips");
        println!("baseline_weekly_km = {:?}", read.baseline_weekly_km);
        assert_eq!(read.baseline_weekly_km, plan.baseline_weekly_km);

        let mut weeks: std::collections::BTreeMap<String, f64> = Default::default();
        for w in &workouts {
            let mon =
                w.date - chrono::Duration::days(w.date.weekday().num_days_from_monday() as i64);
            *weeks
                .entry(mon.format("%Y-%m-%d").to_string())
                .or_insert(0.0) += w.distance_m.unwrap_or(0.0) / 1000.0;
        }
        println!(
            "plan volumes   = {:?}",
            weeks.values().map(|v| *v as i64).collect::<Vec<_>>()
        );
        let first = *weeks.values().next().unwrap();
        assert!(
            (first - avg_km).abs() < avg_km * 0.4,
            "week 1 ({first:.1} km) should start near the runner's baseline ({avg_km:.1} km)"
        );
    }

    #[test]
    fn pain_journal_and_proposals_round_trip() {
        let s = store();
        let today = Utc::now().date_naive();
        let entry = PainEntry {
            id: uuid::Uuid::new_v4().to_string(),
            body_part: "knee.left".into(),
            intensity: 7,
            started_on: today - chrono::Duration::days(3),
            notes: "sharp on downhill".into(),
            created_at: Utc::now(),
            resolved_on: None,
            followup_at: None,
        };
        s.insert_pain_entry(&entry).unwrap();

        let listed = s.list_pain_entries(10).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].notes, "sharp on downhill");
        assert_eq!(s.latest_pain().unwrap().unwrap().id, entry.id);
        assert_eq!(
            s.max_pain_intensity_since(today).unwrap(),
            Some((7, "knee.left".into()))
        );
        // The episode spans started_on..today while unresolved.
        let days = s
            .pain_days(today - chrono::Duration::days(10), today)
            .unwrap();
        assert!(days.iter().all(|(_, i)| *i == 7));
        assert!(!days.is_empty());
        assert_eq!(
            s.pain_by_region(today - chrono::Duration::days(7)).unwrap(),
            vec![("knee.left".into(), 7u32)]
        );

        // A proposal with edits round-trips through its JSON payload.
        let proposal = PlanProposal {
            id: uuid::Uuid::new_v4().to_string(),
            body_part: "knee.left".into(),
            source: ProposalSource::Pain,
            signature: "deload40:knee.left:2026-01-05".into(),
            summary: "Take two rest days and cut 40%.".into(),
            reason: "pain 7/10".into(),
            edits: vec![WorkoutEdit {
                workout_id: "w1".into(),
                kind: crate::types::WorkoutKind::Rest,
                title: "Rest".into(),
                distance_m: None,
                duration_min: None,
                pace_lo_s_km: None,
                pace_hi_s_km: None,
                date: None,
                old_kind: None,
                old_title: None,
                old_distance_m: None,
            }],
            status: ProposalStatus::Pending,
            created_at: Utc::now(),
            acted_at: None,
            plan_quality_offset_sec_km: None,
            plan_quality_offset_from: None,
        };
        s.insert_proposal(&proposal).unwrap();
        let pending = s.pending_proposal().unwrap().expect("pending offer");
        assert_eq!(pending.id, proposal.id);
        assert_eq!(pending.edits.len(), 1);
        assert!(s
            .has_pending_signature_like("knee.left", "2026-01-05")
            .unwrap());

        s.set_proposal_status(
            &proposal.id,
            ProposalStatus::Dismissed,
            Some(Utc::now()),
            Some("feeling fine"),
        )
        .unwrap();
        assert!(s.pending_proposal().unwrap().is_none());
        assert!(s.last_dismissed(&proposal.signature).unwrap().is_some());

        // Messages persist per entry, oldest first.
        for (role, content) in [("user", "does this need rest?"), ("assistant", "yes")] {
            s.insert_pain_message(&PainMessage {
                id: uuid::Uuid::new_v4().to_string(),
                body_part: "knee.left".into(),
                entry_id: Some(entry.id.clone()),
                role: role.into(),
                content: content.into(),
                ts: Utc::now(),
            })
            .unwrap();
        }
        let msgs = s.list_pain_messages(&entry.id).unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "user");

        // Resolving takes the entry out of the active set. The day spans still
        // show the days it *did* hurt (started_on..=resolved_on-1), just not
        // the resolution day itself.
        s.resolve_pain(&entry.id, today).unwrap();
        assert!(s.latest_pain().unwrap().is_none());
        assert!(s.max_pain_intensity_since(today).unwrap().is_none());
        let days = s
            .pain_days(today - chrono::Duration::days(10), today)
            .unwrap();
        assert!(days.iter().all(|(_, i)| *i == 7));
        assert!(!days
            .iter()
            .any(|(d, _)| d == &today.format("%Y-%m-%d").to_string()));
        assert!(days.iter().any(|(d, _)| d
            == &(today - chrono::Duration::days(1))
                .format("%Y-%m-%d")
                .to_string()));
    }

    /// A database built before `followup_ts` joined migration 4's pain_entries
    /// definition stays at user_version 4, so no migration re-runs and the
    /// column never appears — while every pain statement names it. That drift
    /// is what made injury logging fail outright, so the repair must bring such
    /// a database up and keep an already-correct one untouched.
    #[test]
    fn migrate_repairs_pain_entries_missing_followup_ts() {
        let path = std::env::temp_dir().join(format!("rk-drift-{}.sqlite", uuid::Uuid::new_v4()));
        // Recreate the shipped-as-is schema: migration 4 already applied, but
        // pain_entries without followup_ts.
        {
            let conn = Connection::open(&path).expect("open raw");
            conn.execute_batch(
                "CREATE TABLE pain_entries (
                    id TEXT PRIMARY KEY,
                    body_part TEXT NOT NULL,
                    intensity INTEGER NOT NULL CHECK (intensity BETWEEN 0 AND 10),
                    started_on TEXT NOT NULL,
                    notes TEXT NOT NULL DEFAULT '',
                    created_ts INTEGER NOT NULL,
                    resolved_on TEXT
                 );
                 INSERT INTO pain_entries
                    VALUES ('legacy','knee.right',4,'2026-01-04','old',1735948800,NULL);
                 PRAGMA user_version=4;",
            )
            .expect("seed drifted schema");
            assert!(conn
                .prepare("SELECT followup_ts FROM pain_entries")
                .is_err());
        }

        let s = Store::open(&path).expect("open repairs the drifted schema");
        // The pre-existing row survives the ALTER and reads back.
        let listed = s.list_pain_entries(10).expect("list_pain_entries");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "legacy");
        assert_eq!(listed[0].followup_at, None);

        // The write that was failing: insert naming followup_ts, then the
        // follow-up stamp the "is it gone yet?" prompt relies on.
        let entry = PainEntry {
            id: uuid::Uuid::new_v4().to_string(),
            body_part: "calf.left".into(),
            intensity: 8,
            started_on: Utc::now().date_naive(),
            notes: "sharp on downhill".into(),
            created_at: Utc::now(),
            resolved_on: None,
            followup_at: None,
        };
        s.insert_pain_entry(&entry).expect("insert_pain_entry");
        s.mark_pain_followup(&entry.id, Utc::now())
            .expect("mark_pain_followup");
        let after = s
            .list_pain_entries(10)
            .expect("list")
            .into_iter()
            .find(|e| e.id == entry.id)
            .expect("new entry readable");
        assert!(after.followup_at.is_some(), "follow-up stamp persists");

        // Idempotent: reopening an already-repaired database must not fail on a
        // duplicate column.
        drop(s);
        let again = Store::open(&path).expect("reopen is idempotent");
        assert_eq!(again.list_pain_entries(10).expect("list").len(), 2);
        let _ = std::fs::remove_file(&path);
    }

    /// A database that predates migration 4 entirely must still get the table
    /// — the drift repair must not swallow the migration that creates it.
    #[test]
    fn migrate_creates_pain_entries_when_absent() {
        let path = std::env::temp_dir().join(format!("rk-fresh-{}.sqlite", uuid::Uuid::new_v4()));
        {
            let conn = Connection::open(&path).expect("open raw");
            conn.execute_batch("PRAGMA user_version=0;").expect("reset");
        }
        let s = Store::open(&path).expect("open applies migrations");
        assert!(s.list_pain_entries(10).expect("table exists").is_empty());
        assert_eq!(
            s.conn
                .lock()
                .unwrap()
                .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            MIGRATIONS.last().unwrap().0
        );
        let _ = std::fs::remove_file(&path);
    }

    /// The training-status chart draws one line from this query, so it needs a
    /// point per day in order. COROS reports `Load Ratio` only once the
    /// long-term load is non-zero, so the ratio is derived when missing —
    /// otherwise the first days of every history would be holes.
    #[test]
    fn load_history_is_ordered_and_fills_a_missing_ratio() {
        let s = store();
        let day0 = Utc::now().date_naive();
        let put = |back: i64, short: Option<f64>, long: Option<f64>, ratio: Option<f64>| {
            s.upsert_training_load(
                ProviderKind::Coros,
                &TrainingLoad {
                    date: day0 - chrono::Duration::days(back),
                    short_term: short,
                    long_term: long,
                    ratio,
                    comment: None,
                },
            )
            .expect("upsert load");
        };
        put(2, Some(40.0), Some(0.0), None); // brand-new runner: no ratio stored
        put(1, Some(52.0), Some(50.0), None); // ratio omitted but derivable
        put(0, Some(62.0), Some(55.0), Some(1.13));

        let hist = s
            .load_history(day0 - chrono::Duration::days(6), day0)
            .expect("load_history");
        assert_eq!(hist.len(), 3, "one point per synced day");
        assert!(
            hist.windows(2).all(|w| w[0].date < w[1].date),
            "oldest→newest, or the chart draws a zigzag"
        );
        // Derived, not fabricated: 40 ÷ 0 has no answer, so the gap stays.
        assert_eq!(hist[0].ratio, None, "zero chronic load has no ratio");
        assert_eq!(hist[1].ratio, Some(1.04), "52 ÷ 50 fills the missing ratio");
        assert_eq!(hist[2].ratio, Some(1.13), "a stored ratio is kept");

        // A day the provider left entirely blank must not invent a point.
        assert_eq!(
            s.load_history(
                day0 - chrono::Duration::days(30),
                day0 - chrono::Duration::days(3)
            )
            .expect("window before data")
            .len(),
            0
        );
    }
}
