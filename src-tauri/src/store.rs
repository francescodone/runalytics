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

/// How many times a trackless activity's FIT file may be re-downloaded before
/// sync gives up on it. Two extra attempts cover a truncated download and a
/// provider-side repair, while a genuinely GPS-less session stops costing
/// quota after three.
pub const TRACK_RETRY_LIMIT: i64 = 3;

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
        -- The day the plan generated this session into, kept beside `day` so a
        -- reschedule stays visible. `day` is where the runner put it; this is
        -- where the plan said to. Databases created before these columns
        -- existed gain them through the drift repair in `migrate`.
        planned_date TEXT,
        kind TEXT NOT NULL,
        title TEXT NOT NULL,
        distance_m REAL,
        duration_min REAL,
        pace_lo_s_km INTEGER,
        pace_hi_s_km INTEGER,
        done INTEGER NOT NULL DEFAULT 0,
        status TEXT NOT NULL DEFAULT 'planned',
        moved_ts INTEGER,
        -- Why the plan softened this session on its own, in the words the
        -- softener used. Kept beside the session it explains rather than in a
        -- log table so the reason cannot outlive the session it belongs to, and
        -- so the calendar can say why without a second query per day.
        adjusted_reason TEXT
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
    --
    -- Rebuilt rather than emptied: a DELETE on a table whose parent
    -- `activities` is missing aborts the whole open under
    -- `PRAGMA foreign_keys=ON`, and a database in exactly that drifted state
    -- has to keep opening (that is what the schema-drift repair below exists
    -- for). Dropping and recreating has the same effect on a derived cache
    -- and is legal DDL even with the parent absent.
    DROP TABLE IF EXISTS activity_scores;
    CREATE TABLE IF NOT EXISTS activity_scores (
        activity_id INTEGER PRIMARY KEY REFERENCES activities(id) ON DELETE CASCADE,
        load REAL,
        risk INTEGER NOT NULL,
        risk_delta INTEGER NOT NULL DEFAULT 0
    );
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
    --
    -- The table is re-stated first because this migration cannot assume it is
    -- there: a database whose `user_version` is ahead of its actual schema (the
    -- drifted state the repair below exists to survive) can reach here with
    -- pain_messages absent, and an ALTER on a missing table aborts the whole
    -- open. CREATE IF NOT EXISTS is a no-op on every healthy database.
    CREATE TABLE IF NOT EXISTS pain_messages (
        id TEXT PRIMARY KEY,
        body_part TEXT NOT NULL,
        role TEXT NOT NULL,
        content TEXT NOT NULL,
        ts INTEGER NOT NULL
    );
    ALTER TABLE pain_messages ADD COLUMN entry_id TEXT;
    UPDATE pain_messages SET entry_id = (
        SELECT pe.id FROM pain_entries pe
        WHERE pe.body_part = pain_messages.body_part
        ORDER BY pe.started_on DESC, pe.created_ts DESC LIMIT 1
    ) WHERE entry_id IS NULL;
    CREATE INDEX IF NOT EXISTS idx_pain_msg_entry ON pain_messages (entry_id);
    "#,
    ),
    (
        11,
        r#"
    -- Bounded retries for FIT files that downloaded but carried no track:
    -- a GPS-less activity (strength, indoor) must not re-download forever,
    -- and a truncated file deserves more than the one attempt the ledger
    -- allowed. Counted per activity; the sync stops retrying at the bound.
    CREATE TABLE IF NOT EXISTS fit_track_retry (
        provider TEXT NOT NULL,
        provider_record_id TEXT NOT NULL,
        attempts INTEGER NOT NULL DEFAULT 0,
        PRIMARY KEY (provider, provider_record_id)
    );

    -- Barometric altitude per GPS sample, so each kilometre split can be
    -- priced with the elevation it actually climbed. Points synced before this
    -- column existed stay NULL and their splits honestly read "Not recorded".
    -- The table is re-stated first for the same reason migration 10 does it:
    -- a drifted database can reach here with trackpoints absent, and an ALTER
    -- on a missing table aborts the whole open.
    CREATE TABLE IF NOT EXISTS trackpoints (
        activity_id INTEGER NOT NULL REFERENCES activities (id) ON DELETE CASCADE,
        ts INTEGER NOT NULL,
        lat REAL NOT NULL,
        lon REAL NOT NULL,
        hr INTEGER
    );
    ALTER TABLE trackpoints ADD COLUMN alt REAL;
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

/// Merge COROS activities that the FIT download path created as duplicates.
///
/// The download used to key the ingested activity on the FIT blob's filename,
/// and COROS has been seen naming that file with its own file id
/// ("<fileid>_8hexchars.fit") instead of the requested labelId. The upsert
/// then created a *second* row for the same run — holding the track — while
/// the summary row from the record sync stayed trackless forever, because the
/// ledger was keyed on the suffixed id and the original never requalified for
/// download. That is the "activity registered with GPS but no track" bug.
///
/// A duplicate is recognised by the `_<8 hex>` suffix plus a coros row with
/// the same start time and an unsuffixed id. The repair moves the track to
/// the original (only when the original has none — the duplicate is the only
/// copy worth keeping), re-points or drops its completion, drops its derived
/// score, deletes the row, and re-keys the ledger entry so the original is
/// marked as downloaded. Idempotent: once merged, no suffixed row remains.
fn merge_coros_fit_duplicates(conn: &Connection) -> Result<()> {
    // The drifted-state databases this repair must survive can reach open
    // without an activities table at all (that is what the pain_entries
    // repair above guards for); a SELECT on a missing table would abort the
    // whole open, so the merge simply has nothing to do there.
    let activities_exists: bool = conn
        .prepare("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='activities'")?
        .query_row([], |r| r.get::<_, i64>(0))
        .map(|n| n > 0)?;
    if !activities_exists {
        return Ok(());
    }
    // GLOB, not LIKE: SQLite's LIKE has no character classes. A COROS
    // labelId is pure digits, so the only source of an underscore in a coros
    // record id is the FIT filename ("<digits>_<hex>") the old download path
    // ingested under — that shape is the duplicate.
    let dups: Vec<(i64, String, Option<i64>, Option<String>)> = {
        let mut stmt = conn.prepare(
            "SELECT d.id,
                    d.provider_record_id,
                    (SELECT o.id FROM activities o
                      WHERE o.provider = 'coros' AND o.start_ts = d.start_ts AND o.id <> d.id
                        AND o.provider_record_id NOT LIKE '%\\_%' ESCAPE '\\'
                      ORDER BY o.id LIMIT 1),
                    (SELECT o.provider_record_id FROM activities o
                      WHERE o.provider = 'coros' AND o.start_ts = d.start_ts AND o.id <> d.id
                        AND o.provider_record_id NOT LIKE '%\\_%' ESCAPE '\\'
                      ORDER BY o.id LIMIT 1)
             FROM activities d
             WHERE d.provider = 'coros'
               AND d.provider_record_id GLOB '*[0-9]_[0-9a-f]*'
             ORDER BY d.id",
        )?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };
    for (dup_id, dup_rid, orig_id, orig_rid) in dups {
        let (Some(orig_id), Some(orig_rid)) = (orig_id, orig_rid) else {
            // No same-start original to merge into: leave the row alone — it
            // may be a legitimately FIT-only activity, and deleting data on a
            // guess is worse than leaving one trackless row.
            continue;
        };
        conn.execute("BEGIN", [])?;
        let result = (|| -> Result<()> {
            // The track moves only if the original has none; a duplicate
            // whose original already has a track is simply redundant.
            let orig_points: i64 = conn.query_row(
                "SELECT COUNT(*) FROM trackpoints WHERE activity_id = ?1",
                params![orig_id],
                |r| r.get(0),
            )?;
            if orig_points == 0 {
                conn.execute(
                    "UPDATE trackpoints SET activity_id = ?1 WHERE activity_id = ?2",
                    params![orig_id, dup_id],
                )?;
            } else {
                conn.execute(
                    "DELETE FROM trackpoints WHERE activity_id = ?1",
                    params![dup_id],
                )?;
            }
            // A completion pointing at the duplicate follows it to the
            // original — unless the original already owns one, in which case
            // the duplicate's is the stale copy.
            let orig_completion: i64 = conn.query_row(
                "SELECT COUNT(*) FROM calendar_completions WHERE activity_id = ?1",
                params![orig_id],
                |r| r.get(0),
            )?;
            if orig_completion == 0 {
                conn.execute(
                    "UPDATE calendar_completions SET activity_id = ?1 WHERE activity_id = ?2",
                    params![orig_id, dup_id],
                )?;
            } else {
                conn.execute(
                    "DELETE FROM calendar_completions WHERE activity_id = ?1",
                    params![dup_id],
                )?;
            }
            // Scores are a derived cache, cleared and recomputed on refresh.
            conn.execute(
                "DELETE FROM activity_scores WHERE activity_id = ?1",
                params![dup_id],
            )?;
            conn.execute(
                "DELETE FROM activities WHERE id = ?1",
                params![dup_id],
            )?;
            // The ledger recorded the download under the suffixed id; move it
            // so the original counts as downloaded and cannot re-download.
            conn.execute(
                "DELETE FROM fit_file_ledger WHERE provider = 'coros' AND provider_record_id = ?1",
                params![dup_rid],
            )?;
            conn.execute(
                "INSERT OR REPLACE INTO fit_file_ledger (provider, provider_record_id, downloaded_ts)
                 VALUES ('coros', ?1, ?2)",
                params![orig_rid, Utc::now().timestamp()],
            )?;
            Ok(())
        })();
        match result {
            Ok(()) => {
                conn.execute("COMMIT", [])?;
                eprintln!("[store] merged coros fit duplicate {dup_id} into {orig_id}");
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK", []);
                eprintln!("[store] fit duplicate merge failed for {dup_id}: {e}");
            }
        }
    }
    Ok(())
}

fn parse_date(s: &str) -> Result<NaiveDate> {    NaiveDate::parse_from_str(s, "%Y-%m-%d")
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

/// The column list every `plan_workouts` read uses, in the order
/// [`workout_from_row`] expects. A SELECT that names its columns instead of
/// `SELECT *` and still forgets one of these fails loudly at read time rather
/// than silently handing back a workout missing its generated date.
const WORKOUT_COLS: &str = "id, plan_id, day, planned_date, kind, title, distance_m, \
    duration_min, pace_lo_s_km, pace_hi_s_km, done, status, moved_ts, customization_json";

/// Map one `plan_workouts` row (see [`WORKOUT_COLS`]) onto a workout.
///
/// A NULL `planned_date` falls back to the session's day: a row written before
/// the column existed has no recorded generated day, and the only honest
/// reading of that is that it was never moved.
fn workout_from_row(r: &rusqlite::Row<'_>) -> std::result::Result<PlanWorkout, rusqlite::Error> {
    let day = parse_date_row(&r.get::<_, String>(2)?)?;
    let planned_date = match r.get::<_, Option<String>>(3)? {
        Some(s) => parse_date_row(&s)?,
        None => day,
    };
    Ok(PlanWorkout {
        id: r.get(0)?,
        plan_id: r.get(1)?,
        date: day,
        planned_date,
        kind: kind_from(&r.get::<_, String>(4)?),
        title: r.get(5)?,
        distance_m: r.get(6)?,
        duration_min: r.get(7)?,
        pace_lo_s_km: r.get(8)?,
        pace_hi_s_km: r.get(9)?,
        done: r.get::<_, i64>(10)? != 0,
        status: workout_status_from(&r.get::<_, String>(11)?),
        moved_at: r.get::<_, Option<i64>>(12)?.map(ts_to_dt),
        customization: r.get::<_, Option<String>>(13)?.map(|json| serde_json::from_str(&json)
            .map_err(|error| rusqlite::Error::FromSqlConversionFailure(13, rusqlite::types::Type::Text, Box::new(error)))).transpose()?,
    })
}

/// The `plan_workouts` values a workout writes, in the order of
/// [`WORKOUT_INSERT_COLS` / the UPDATE in `update_workout`]. Returned as one
/// list so an INSERT and an UPDATE of the same workout cannot disagree about
/// which value belongs in which column.
fn workout_params(w: &PlanWorkout) -> Vec<rusqlite::types::Value> {
    let opt_real = |v: Option<f64>| v.map(rusqlite::types::Value::Real).unwrap_or(rusqlite::types::Value::Null);
    let opt_int = |v: Option<i64>| v.map(rusqlite::types::Value::Integer).unwrap_or(rusqlite::types::Value::Null);
    vec![
        rusqlite::types::Value::Text(w.id.clone()),
        rusqlite::types::Value::Text(w.plan_id.clone()),
        rusqlite::types::Value::Text(w.date.format("%Y-%m-%d").to_string()),
        rusqlite::types::Value::Text(w.planned_date.format("%Y-%m-%d").to_string()),
        rusqlite::types::Value::Text(kind_name(w.kind).to_string()),
        rusqlite::types::Value::Text(w.title.clone()),
        opt_real(w.distance_m),
        opt_real(w.duration_min),
        opt_int(w.pace_lo_s_km),
        opt_int(w.pace_hi_s_km),
        rusqlite::types::Value::Integer(w.done as i64),
        rusqlite::types::Value::Text(workout_status_name(w.status).to_string()),
        w.moved_at
            .map(|t| rusqlite::types::Value::Integer(t.timestamp()))
            .unwrap_or(rusqlite::types::Value::Null),
        w.customization.as_ref().map(|value| rusqlite::types::Value::Text(serde_json::to_string(value).expect("serializable customization")))
            .unwrap_or(rusqlite::types::Value::Null),
    ]
}

/// An INSERT column list and its matching `?1..?13` placeholders.
const WORKOUT_INSERT_COLS: &str = "INSERT INTO plan_workouts (id, plan_id, day, planned_date, \
    kind, title, distance_m, duration_min, pace_lo_s_km, pace_hi_s_km, done, status, moved_ts, customization_json) \
    VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)";

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
        conn.execute_batch(crate::strava::SCHEMA)?;
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
        add_column_if_missing(&conn, "training_plans", "threshold_state", "TEXT")?;
        add_column_if_missing(&conn, "training_plans", "training_method", "TEXT NOT NULL DEFAULT 'norwegian'")?;
        add_column_if_missing(&conn, "training_plans", "start_date", "TEXT")?;
        // What the runner calls the plan. Nullable rather than defaulted:
        // a plan nobody named genuinely has no name, and every naming
        // surface falls back to describing the plan instead — an empty
        // string would render as a blank title.
        add_column_if_missing(&conn, "training_plans", "name", "TEXT")?;
        // Why the plan softened a session on its own. Added as a repair rather
        // than a numbered migration for the usual reason, and nullable rather
        // than defaulted: a session nobody softened genuinely has no reason,
        // and "no reason" must not be renderable as an empty explanation.
        add_column_if_missing(&conn, "plan_workouts", "adjusted_reason", "TEXT")?;
        add_column_if_missing(&conn, "plan_workouts", "customization_json", "TEXT")?;
        // When the plan softened a session — the softener's own rate limit.
        // The threshold evidence that triggers softening can stay in warning
        // state for weeks, and without a timestamp of its own the softener
        // would give up one more hard session on every sync (see
        // `last_softened_at`). Nullable: a session nobody softened has no
        // softening instant, and the repair cannot know when older shipped
        // databases softened theirs, so NULL honestly means "unknown".
        add_column_if_missing(&conn, "plan_workouts", "adjusted_ts", "INTEGER")?;
        // The softener's cooldown clock, kept outside the plan so clearing
        // or recreating a plan cannot reset it (see `last_softened_at`).
        // Idempotent repair rather than a numbered migration, as usual.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS app_state (
                key TEXT PRIMARY KEY,
                value INTEGER NOT NULL
            );",
        )?;
        conn.execute("INSERT OR IGNORE INTO app_state(key,value) VALUES('feedback_started_at',strftime('%s','now'))", [])?;
        conn.execute_batch("CREATE TABLE IF NOT EXISTS difficulty_undo(plan_id TEXT PRIMARY KEY REFERENCES training_plans(id) ON DELETE CASCADE, revision INTEGER NOT NULL, content TEXT NOT NULL)")?;
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
        // Every session keeps the day the plan generated it into, so a
        // reschedule stays visible: without it there is nothing to reset to and
        // no way to tell a moved session from a generated one. Added as a repair
        // rather than a numbered migration for the usual reason, and `planned_date`
        // is nullable in the schema precisely because a row written before the
        // column existed genuinely has no recorded generated day — the backfill
        // below says "it was never moved", which is the only honest reading, and
        // readers fall back to `day` for any row that somehow lacks one.
        if add_column_if_missing(&conn, "plan_workouts", "planned_date", "TEXT")? {
            conn.execute(
                "UPDATE plan_workouts SET planned_date = day WHERE planned_date IS NULL",
                [],
            )?;
        }
        add_column_if_missing(
            &conn,
            "plan_workouts",
            "status",
            "TEXT NOT NULL DEFAULT 'planned'",
        )?;
        add_column_if_missing(&conn, "plan_workouts", "moved_ts", "INTEGER")?;
        // What the plan originally said about a session a benchmark overwrote.
        // Scheduling a race or test rewrites kind, title and distance in place,
        // and without this snapshot the planned session is gone for good — a
        // reset could only put dates back, never content. Nullable: a session
        // no benchmark touched genuinely has nothing stored.
        add_column_if_missing(&conn, "plan_workouts", "original_json", "TEXT")?;
        // Barometric altitude per GPS sample, read by every activity track query.
        // Migration 11 adds it, but only to a database *below* version 11: one
        // that is already at 11 (or ahead of it) while its trackpoints table
        // never gained the column stays broken forever, and opening any activity
        // fails with "no such column: alt". Same drift the repairs above exist to
        // heal, so it is checked on every open rather than version-gated.
        add_column_if_missing(&conn, "trackpoints", "alt", "REAL")?;
        // One-shot ledger for elevation re-downloads: activities whose FIT was
        // fetched while the upsert still discarded the decoded ascent need the
        // file again (it is the only place the device's climb total survives),
        // and each must be re-fetched exactly once. Created on every open
        // rather than version-gated, per the usual drift reason.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS fit_elevation_backfill (
                provider TEXT NOT NULL,
                provider_record_id TEXT NOT NULL,
                tried_ts INTEGER NOT NULL,
                PRIMARY KEY (provider, provider_record_id)
            );",
        )?;
        // The plan's revision: the wall-clock second of its last edit, which is
        // what cross-device sync compares. Plans written before the column
        // existed are backfilled from their creation time — the honest minimum,
        // since nothing edited them between creation and this open. A plain
        // repair rather than a numbered migration, per the usual reason.
        if add_column_if_missing(&conn, "training_plans", "rev", "INTEGER")? {
            conn.execute(
                "UPDATE training_plans SET rev = created_ts WHERE rev IS NULL",
                [],
            )?;
        }
        // Heal the tracks that landed on a duplicate row. Runs on every open:
        // it is a no-op once the duplicates are gone, and a numbered migration
        // would miss a database already at the newest version.
        merge_coros_fit_duplicates(&conn)?;
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
                provider_sport_code = COALESCE(excluded.provider_sport_code, provider_sport_code),
                -- The FIT file is the only pass that carries ascent and max HR
                -- (the COROS text report has neither) and it always arrives
                -- *after* the summary row, so without these the decoded values
                -- were dropped on every conflict and every outdoor run showed
                -- elevation as not recorded. COALESCE keeps whichever side
                -- knows: a later summary re-sync cannot wipe them again.
                elevation_gain_m = COALESCE(excluded.elevation_gain_m, elevation_gain_m),
                max_hr = COALESCE(excluded.max_hr, max_hr)",
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
                        "INSERT INTO trackpoints (activity_id, ts, lat, lon, hr, alt) VALUES (?1,?2,?3,?4,?5,?6)",
                        params![id, p.ts, p.lat, p.lon, p.hr, p.alt],
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

    /// Stamp the plan that owns `workout_id` as edited *now*, which is what
    /// cross-device sync compares. Executed as a statement rather than a
    /// `Store` method on purpose: every caller here already holds the
    /// connection lock (or is inside a transaction on it), and the mutex is
    /// not re-entrant — a method that locked again would deadlock.
    ///
    /// Monotonic by construction: never below the revision it already carries
    /// plus one, so two edits in the same second still order themselves, and
    /// never below the wall clock, so a device that was offline catches up
    /// rather than publishing a revision older than one it saw.
    const REV_BUMP: &'static str = "UPDATE training_plans SET rev = \
        MAX(IFNULL(rev, created_ts) + 1, CAST(strftime('%s','now') AS INTEGER)) \
         WHERE id = (SELECT plan_id FROM plan_workouts WHERE id = ?1)";

    /// The same stamp for a caller that knows the plan id directly.
    const REV_BUMP_PLAN: &'static str = "UPDATE training_plans SET rev = \
        MAX(IFNULL(rev, created_ts) + 1, CAST(strftime('%s','now') AS INTEGER)) \
         WHERE id = ?1";

    /// The revision of the current plan, for the sync comparison. `None` when
    /// there is no plan. `rev` is nullable in the schema (a row written before
    /// the column existed genuinely has no recorded edit), so the read falls
    /// back to the creation instant — the same backfill `migrate` applies.
    pub fn current_plan_rev(&self) -> Result<Option<i64>> {
        let conn = self.conn.lock().unwrap();
        Ok(conn
            .query_row(
                "SELECT COALESCE(rev, created_ts) FROM training_plans
                 WHERE status IN ('draft', 'active', 'pushed')
                 ORDER BY created_ts DESC LIMIT 1",
                [],
                |r| r.get::<_, i64>(0),
            )
            .optional()?)
    }

    pub fn feedback_started_at(&self) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        Ok(conn.query_row("SELECT value FROM app_state WHERE key='feedback_started_at'", [], |row| row.get(0))?)
    }

    pub fn commit_difficulty(&self, plan_id: &str, revision: i64, originals: &[PlanWorkout], workouts: &[PlanWorkout]) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let transaction = conn.transaction()?;
        let changed = transaction.execute(
            "UPDATE training_plans SET rev=MAX(rev+1,CAST(strftime('%s','now') AS INTEGER)) WHERE id=?1 AND COALESCE(rev,created_ts)=?2 AND status IN ('draft','active','pushed')",
            params![plan_id, revision],
        )?;
        if changed != 1 { return Err(AppError::Other("The plan changed. Refresh before trying again.".into())); }
        let mut previous = Vec::new();
        for workout in workouts {
            if !workout.done {
                let before = transaction.query_row("SELECT customization_json,duration_min,distance_m FROM plan_workouts WHERE id=?1 AND done=0", params![workout.id], |row|
                    Ok((workout.id.clone(), row.get::<_, Option<String>>(0)?, row.get::<_, Option<f64>>(1)?, row.get::<_, Option<f64>>(2)?)))?;
                previous.push(before);
            }
            let customization = workout.customization.as_ref().map(serde_json::to_string).transpose()?;
            let original = originals.iter().find(|original| original.id == workout.id)
                .ok_or_else(|| AppError::Other("A session changed. Refresh before trying again.".into()))?;
            let changed = transaction.execute(
                "UPDATE plan_workouts SET customization_json=?3, duration_min=?4, distance_m=?8 WHERE id=?1 AND plan_id=?2 AND done=?5 AND status='planned' AND day=?6 AND distance_m IS ?7 AND (?5 OR NOT EXISTS(SELECT 1 FROM calendar_completions WHERE workout_id=?1))",
                params![workout.id, plan_id, customization, workout.duration_min, workout.done, workout.date.to_string(), original.distance_m, workout.distance_m],
            )?;
            if changed != 1 { return Err(AppError::Other("A session changed. Refresh before trying again.".into())); }
        }
        transaction.execute("UPDATE plan_proposals SET status='dismissed',acted_ts=strftime('%s','now') WHERE source='pacing' AND status='pending'", [])?;
        if !previous.is_empty() {
            transaction.execute("INSERT OR REPLACE INTO difficulty_undo(plan_id,revision,content) SELECT id,rev,?2 FROM training_plans WHERE id=?1",
                params![plan_id, serde_json::to_string(&previous)?])?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn undo_difficulty(&self, plan_id: &str) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let transaction = conn.transaction()?;
        let content: Option<String> = transaction.query_row("SELECT u.content FROM difficulty_undo u JOIN training_plans p ON p.id=u.plan_id AND p.rev=u.revision WHERE u.plan_id=?1 AND p.status IN ('draft','active','pushed')", params![plan_id], |row| row.get(0)).optional()?;
        let content = content.ok_or_else(|| AppError::Other("The plan changed; this adjustment can no longer be undone.".into()))?;
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum PreviousDifficulty {
            WithDistance((String, Option<String>, Option<f64>, Option<f64>)),
            Legacy((String, Option<String>, Option<f64>)),
        }
        let previous: Vec<PreviousDifficulty> = serde_json::from_str(&content)?;
        for previous in previous {
            let (id, customization, duration, distance, restore_distance) = match previous {
                PreviousDifficulty::WithDistance((id, customization, duration, distance)) => (id, customization, duration, distance, true),
                PreviousDifficulty::Legacy((id, customization, duration)) => (id, customization, duration, None, false),
            };
            let changed = transaction.execute("UPDATE plan_workouts SET customization_json=?2,duration_min=?3,distance_m=CASE WHEN ?5 THEN ?6 ELSE distance_m END WHERE id=?1 AND done=0 AND status='planned' AND day>=?4 AND NOT EXISTS(SELECT 1 FROM calendar_completions WHERE workout_id=?1)", params![id, customization, duration, chrono::Local::now().date_naive().to_string(), restore_distance, distance])?;
            if changed != 1 { return Err(AppError::Other("A session changed; this adjustment can no longer be undone.".into())); }
        }
        transaction.execute(Self::REV_BUMP_PLAN, params![plan_id])?;
        transaction.execute("DELETE FROM difficulty_undo WHERE plan_id=?1", params![plan_id])?;
        transaction.commit()?;
        Ok(())
    }

    /// Per-week mileage of the current plan: what it prescribes and what the
    /// runner has actually banked, bucketed by Monday. The plan roadmap reads
    /// this to mark each week done / current / upcoming with real numbers —
    /// the same `RUN_SQL` filter as every other mileage figure, so a week's
    /// ran km on the roadmap can never disagree with the dashboard.
    ///
    /// Weeks with no sessions are absent, like [`weekly_volumes`]; the roadmap
    /// only walks weeks the plan contains anyway.
    pub fn plan_week_progress(&self) -> Result<Vec<(String, f64, f64)>> {
        let conn = self.conn.lock().unwrap();
        // The week bucket is computed in an inner select so the correlated
        // activity subquery can see it as a plain column — SQLite does not
        // expose the outer select's alias to a subquery in another select
        // list. Runs are capped at `now` so a week in progress never counts
        // mileage that has not happened yet.
        let mut stmt = conn.prepare(
            "SELECT wk, SUM(prescribed_m),
                    COALESCE((SELECT SUM(a.distance_m) FROM activities a
                       WHERE a.start_ts >= CAST(strftime('%s', x.wk) AS INTEGER)
                         AND a.start_ts < CAST(strftime('%s', x.wk, '+7 days') AS INTEGER)
                         AND a.start_ts < CAST(strftime('%s','now') AS INTEGER)
                         AND (lower(a.sport) IN ('run','running','jog','jogging')
                              OR lower(a.sport) LIKE '%run%')), 0)
             FROM (SELECT date(w.day, '-6 days', 'weekday 1') wk,
                          COALESCE(w.distance_m, 0) prescribed_m
                   FROM plan_workouts w
                   WHERE w.plan_id = (SELECT id FROM training_plans
                                      WHERE status IN ('draft','active','pushed')
                                      ORDER BY created_ts DESC LIMIT 1)) x
             GROUP BY wk ORDER BY wk",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?, r.get::<_, f64>(2)?))
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
    /// not this device's row id. A pull resolves these back against its own
    /// `activities` table, which is why completion can travel between devices
    /// that synced the same watch independently and never shared a database.
    pub fn calendar_completion_records(
        &self,
    ) -> Result<Vec<(String, String, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT c.workout_id, a.provider, a.provider_record_id
             FROM calendar_completions c
             JOIN activities a ON a.id = c.activity_id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    /// The local row id of a provider's activity, when this device has it.
    /// Plan sync uses this to resolve a completion that arrived from the
    /// other device: the pair names the run by the *provider's* id, and both
    /// devices synced the same watch, so the record id is the shared key.
    pub fn activity_id_by_record(
        &self,
        provider: &str,
        record_id: &str,
    ) -> Result<Option<i64>> {
        let conn = self.conn.lock().unwrap();
        Ok(conn
            .query_row(
                "SELECT id FROM activities WHERE provider=?1 AND provider_record_id=?2",
                params![provider, record_id],
                |r| r.get::<_, i64>(0),
            )
            .optional()?)
    }

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
        // Marking a session done is an edit the other device has to hear
        // about, so the plan that owns it is stamped as revised.
        tx.execute(Self::REV_BUMP, params![workout_id])?;
        tx.commit()?;
        Ok(())
    }

    pub fn set_completion_notes(&self, workout_id: &str, notes: &str) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let changed = tx.execute(
            "UPDATE calendar_completions SET original_notes=?2 WHERE workout_id=?1 AND original_notes<>?2",
            params![workout_id, notes],
        )?;
        if changed > 0 {
            tx.execute(Self::REV_BUMP, params![workout_id])?;
        }
        tx.commit()?;
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
                     kind, quality_offset_sec_km, quality_offset_from, rev, threshold_state, start_date, name, training_method)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19)",
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
                    .map(|d| d.format("%Y-%m-%d").to_string()),
                // A brand-new plan's revision is its creation instant; every
                // later edit moves it forward from there.
                p.created_at.timestamp(),
                p.threshold_state.as_ref().map(serde_json::to_string).transpose()?,
                p.start_date.map(|date| date.format("%Y-%m-%d").to_string()),
                p.name.as_deref().map(str::trim).filter(|n| !n.is_empty()),
                serde_json::to_value(p.training_method)?.as_str().unwrap(),
            ],
        )?;
        Ok(())
    }

    pub fn insert_workouts(&self, workouts: &[PlanWorkout]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        for w in workouts {
            conn.execute(WORKOUT_INSERT_COLS, rusqlite::params_from_iter(workout_params(w)))?;
        }
        Ok(())
    }

    /// Overwrite the sessions a benchmark touches, remembering first what the
    /// plan had there. The snapshot is written once: a second benchmark on the
    /// same day must not overwrite the first snapshot, or the session the plan
    /// actually generated is lost and no reset can bring it back. A row the
    /// benchmark creates (the plan had a rest day there) records that fact, so
    /// a reset can remove the session rather than invent content for it.
    pub fn save_benchmark_schedule(&self, workouts: &[PlanWorkout]) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let transaction = conn.transaction()?;
        let sql = format!("{WORKOUT_INSERT_COLS} ON CONFLICT(id) DO UPDATE SET kind=excluded.kind, title=excluded.title, distance_m=excluded.distance_m, duration_min=excluded.duration_min, pace_lo_s_km=excluded.pace_lo_s_km, pace_hi_s_km=excluded.pace_hi_s_km, customization_json=excluded.customization_json");
        for workout in workouts {
            // `None` = no row yet (the benchmark creates it); `Some(None)` =
            // a row with nothing snapshotted yet, whose content is about to be
            // lost; `Some(Some(_))` = an earlier benchmark already remembered.
            let stored: Option<Option<String>> = transaction
                .query_row(
                    "SELECT original_json FROM plan_workouts WHERE id=?1",
                    params![workout.id],
                    |r| r.get::<_, Option<String>>(0),
                )
                .optional()?;
            let before = stored
                .clone()
                .filter(|snap| snap.is_none())
                .map(|_| {
                    transaction.query_row(
                        "SELECT kind, title, distance_m, duration_min, pace_lo_s_km, pace_hi_s_km, customization_json \
                         FROM plan_workouts WHERE id=?1",
                        params![workout.id],
                        |r| {
                            Ok(OriginalContent {
                                customization: r.get::<_, Option<String>>(6)?.map(|json| serde_json::from_str(&json)
                                    .map_err(|error| rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Text, Box::new(error)))).transpose()?,
                                created: false,
                                kind: kind_from(&r.get::<_, String>(0)?),
                                title: r.get(1)?,
                                distance_m: r.get(2)?,
                                duration_min: r.get(3)?,
                                pace_lo_s_km: r.get(4)?,
                                pace_hi_s_km: r.get(5)?,
                            })
                        },
                    )
                })
                .transpose()?;
            transaction.execute(&sql, rusqlite::params_from_iter(workout_params(workout)))?;
            let snapshot = match before {
                Some(content) => content,
                None if stored.is_none() => OriginalContent {
                    customization: None,
                    created: true,
                    kind: WorkoutKind::Rest,
                    title: String::new(),
                    distance_m: None,
                    duration_min: None,
                    pace_lo_s_km: None,
                    pace_hi_s_km: None,
                },
                None => continue,
            };
            transaction.execute(
                "UPDATE plan_workouts SET original_json=?2 WHERE id=?1",
                params![workout.id, serde_json::to_string(&snapshot)?],
            )?;
        }
        if let Some(workout) = workouts.first() {
            transaction.execute(Self::REV_BUMP_PLAN, params![workout.plan_id])?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// The plan's own content for a session a benchmark overwrote, for the
    /// reset preview and the reset itself. `None` for a session no benchmark
    /// touched.
    pub fn workout_originals(&self) -> Result<std::collections::HashMap<String, OriginalContent>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT id, original_json FROM plan_workouts WHERE original_json IS NOT NULL")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        let mut out = std::collections::HashMap::new();
        for row in rows {
            let (id, json) = row?;
            if let Ok(content) = serde_json::from_str::<OriginalContent>(&json) {
                out.insert(id, content);
            }
        }
        Ok(out)
    }

    /// Rewrite a session back to the plan's own content *and* drop its
    /// benchmark snapshot, in one statement. A caller that used plain
    /// [`update_workout`](Self::update_workout) would restore the content but
    /// leave the snapshot behind, so the session would keep reading as
    /// benchmarked and offer the same reset again forever.
    pub fn update_workout_clearing_original(&self, w: &PlanWorkout) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE plan_workouts SET plan_id=?2, day=?3, planned_date=?4, kind=?5,
                title=?6, distance_m=?7, duration_min=?8, pace_lo_s_km=?9,
                pace_hi_s_km=?10, done=?11, status=?12, moved_ts=?13, customization_json=?14, original_json=NULL
             WHERE id=?1",
            rusqlite::params_from_iter(workout_params(w)),
        )?;
        conn.execute(Self::REV_BUMP, params![w.id])?;
        Ok(())
    }

    /// Remove a session a benchmark created on what the plan left as a rest
    /// day — the row did not exist before, so restoring the plan means deleting
    /// it.
    pub fn delete_workout(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM plan_workouts WHERE id=?1", params![id])?;
        Ok(())
    }

    /// The most recent plan that is not finished.
    pub fn current_plan(&self) -> Result<Option<(TrainingPlan, Vec<PlanWorkout>)>> {
        let plan = {
            let conn = self.conn.lock().unwrap();
            let row = conn.query_row(
                "SELECT id, distance, target_date, status, weekly_frequency, longrun_day,
                        increase, threshold_sec_km, created_ts, pushed_id, baseline_weekly_km,
                        kind, quality_offset_sec_km, quality_offset_from, threshold_state, start_date, name, training_method
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
                        r.get::<_, Option<String>>(14)?,
                        r.get::<_, Option<String>>(15)?,
                        r.get::<_, Option<String>>(16)?,
                        r.get::<_, String>(17)?,
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
                    threshold_state,
                    start_date,
                    name,
                    training_method,
                )) => TrainingPlan {
                    id,
                    distance: distance_from(&dist),
                    training_method: serde_json::from_value(serde_json::Value::String(training_method))?,
                    kind: plan_kind_from(&kind),
                    start_date: start_date.as_deref().map(parse_date).transpose()?,
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
                    threshold_state: threshold_state.as_deref().map(serde_json::from_str).transpose()?,
                    name,
                },
            }
        };
        let workouts = self.plan_workouts(plan.id.as_str())?;
        Ok(Some((plan, workouts)))
    }

    pub fn plan_workouts(&self, plan_id: &str) -> Result<Vec<PlanWorkout>> {
        let conn = self.conn.lock().unwrap();
        let sql = format!("SELECT {WORKOUT_COLS} FROM plan_workouts WHERE plan_id=?1 ORDER BY day");
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![plan_id], workout_from_row)?;
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

    /// Wipe every plan, its sessions (cascade), and every open or answered
    /// plan offer, while leaving the runner's history — activities, health,
    /// pain, tracks — untouched. This is what "clear the local plan data"
    /// means: the plan mirrors are rebuilt from whatever the runner creates
    /// next, and a stale offer pointing at a deleted session would only
    /// fail on apply. One transaction so a half-clear cannot strand an offer
    /// beside a missing plan or vice versa.
    pub fn clear_plan_data(&self) -> Result<()> {
        self.clear_synced_plan_data(None)
    }

    pub(crate) fn clear_synced_plan_data(&self, checkpoint: Option<&str>) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM plan_proposals", [])?;
        tx.execute("DELETE FROM training_plans", [])?;
        if let Some(checkpoint) = checkpoint {
            tx.execute("INSERT INTO app_state(key,value) VALUES('plan_sync_checkpoint',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [checkpoint])?;
        }
        tx.commit()?;
        Ok(())
    }

    pub(crate) fn plan_sync_checkpoint(&self) -> Result<Option<String>> {
        Ok(self.conn.lock().unwrap().query_row("SELECT value FROM app_state WHERE key='plan_sync_checkpoint'", [], |row| row.get(0)).optional()?)
    }

    pub(crate) fn save_plan_sync_checkpoint(&self, checkpoint: &str) -> Result<()> {
        self.conn.lock().unwrap().execute("INSERT INTO app_state(key,value) VALUES('plan_sync_checkpoint',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [checkpoint])?;
        Ok(())
    }

    /// Every plan the database holds, newest first, whatever its status.
    /// `current_plan` deliberately sees only the live one; a clear-all needs
    /// the whole list so no abandoned row survives to become current again.
    pub fn all_plans(&self) -> Result<Vec<TrainingPlan>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, distance, target_date, status, weekly_frequency, longrun_day,
                    increase, threshold_sec_km, created_ts, pushed_id, baseline_weekly_km,
                    kind, quality_offset_sec_km, quality_offset_from, threshold_state, start_date, name, training_method
             FROM training_plans ORDER BY created_ts DESC",
        )?;
        let rows = stmt.query_map([], |r| {
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
                r.get::<_, Option<String>>(14)?,
                r.get::<_, Option<String>>(15)?,
                r.get::<_, Option<String>>(16)?,
                r.get::<_, String>(17)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (
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
                threshold_state,
                start_date,
                name,
                training_method,
            ) = row?;
            out.push(TrainingPlan {
                id,
                distance: distance_from(&dist),
                training_method: serde_json::from_value(serde_json::Value::String(training_method))?,
                kind: plan_kind_from(&kind),
                start_date: start_date.as_deref().map(parse_date).transpose()?,
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
                threshold_state: threshold_state.as_deref().map(serde_json::from_str).transpose()?,
                name,
            });
        }
        Ok(out)
    }

    /// Install a plan as the only plan on this device, replacing whatever was
    /// there. Used by plan sync: an imported plan must not sit beside a stale
    /// one, because `current_plan` picks by `created_ts` and two in-flight
    /// plans would make the calendar show both.
    ///
    /// `rev` is the revision being installed, recorded so the sync comparison
    /// on this device starts from what it actually took rather than from the
    /// plan's creation time — which is how a pull avoids immediately
    /// concluding that the copy it just installed is an update to import.
    ///
    /// One transaction, so a failure part-way through cannot leave the device
    /// with the old plan deleted and the new one unwritten.
    pub fn replace_plan(
        &self,
        plan: &TrainingPlan,
        workouts: &[PlanWorkout],
        rev: i64,
    ) -> Result<()> {
        self.replace_synced_plan(plan, workouts, rev, None)
    }

    pub(crate) fn replace_synced_plan(&self, plan: &TrainingPlan, workouts: &[PlanWorkout], rev: i64, checkpoint: Option<&str>) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM plan_proposals", [])?;
        tx.execute("DELETE FROM training_plans", [])?;
        tx.execute(
            "INSERT INTO training_plans (id, distance, target_date, status, weekly_frequency,
                longrun_day, increase, threshold_sec_km, created_ts, pushed_id, baseline_weekly_km,
                     kind, quality_offset_sec_km, quality_offset_from, rev, threshold_state, start_date, name, training_method)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19)",
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
                    .map(|d| d.format("%Y-%m-%d").to_string()),
                rev,
                plan.threshold_state.as_ref().map(serde_json::to_string).transpose()?,
                plan.start_date.map(|date| date.format("%Y-%m-%d").to_string()),
                plan.name.as_deref().map(str::trim).filter(|n| !n.is_empty()),
                serde_json::to_value(plan.training_method)?.as_str().unwrap(),
            ],
        )?;
        for w in workouts {
            tx.execute(WORKOUT_INSERT_COLS, rusqlite::params_from_iter(workout_params(w)))?;
        }
        if let Some(checkpoint) = checkpoint {
            tx.execute("INSERT INTO app_state(key,value) VALUES('plan_sync_checkpoint',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [checkpoint])?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Replace a workout's kind/title (used by adaptive downgrades).
    /// Soften a session the plan decided was too much, and record *why* on the
    /// same row. The reason has to travel with the session: an adjustment the
    /// runner cannot see the reason for reads as a bug, and a reason stored
    /// somewhere else is a second thing that can go missing.
    pub fn soften_workout(
        &self,
        id: &str,
        kind: WorkoutKind,
        title: &str,
        reason: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();
        conn.execute(
            "UPDATE plan_workouts SET kind=?2, title=?3, adjusted_reason=?4, adjusted_ts=?5 WHERE id=?1",
            params![id, kind_name(kind), title, reason, now],
        )?;
        // The cooldown instant also goes to `app_state`, outside the plan:
        // clearing or replacing the plan deletes the session row, and a
        // softener whose memory dies with the plan softens the replacement
        // on its very first sync — the "erase, recreate, overridden again"
        // report. The per-session `adjusted_ts` stays the display record;
        // this row is the rate limit's clock.
        conn.execute(
            "INSERT INTO app_state (key, value) VALUES ('last_softened_ts', ?1)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![now],
        )?;
        conn.execute(Self::REV_BUMP, params![id])?;
        Ok(())
    }

    /// When the softener last gave up a session, in wall-clock time, across
    /// every plan that ever existed on this device. The adaptive softener's
    /// rate limit reads this: the evidence that triggers softening (a
    /// threshold state in warning, a load ratio still high) persists for
    /// weeks, and without a cooldown every sync would give up one more hard
    /// session. The `app_state` row is the authoritative clock because it
    /// survives the plan it was written for — a recreated plan must not
    /// inherit a reset cooldown — with the per-session column as the
    /// fallback for databases that softened a session before the clock
    /// existed.
    pub fn last_softened_at(&self) -> Result<Option<DateTime<Utc>>> {
        let conn = self.conn.lock().unwrap();
        let clock: Option<i64> = conn
            .query_row(
                "SELECT value FROM app_state WHERE key='last_softened_ts'",
                [],
                |r| r.get(0),
            )
            .optional()?
            .flatten();
        let on_sessions: Option<i64> = conn
            .query_row(
                "SELECT MAX(adjusted_ts) FROM plan_workouts WHERE adjusted_reason IS NOT NULL",
                [],
                |r| r.get(0),
            )
            .optional()?
            .flatten();
        Ok(clock
            .into_iter()
            .chain(on_sessions)
            .max()
            .and_then(|s| DateTime::<Utc>::from_timestamp(s, 0)))
    }

    /// Every session the plan softened, with the reason it gave. Read by the
    /// calendar so the chip, the agenda row and the session sheet can all say
    /// why the day changed without each of them re-deriving it.
    pub fn workout_adjustments(&self) -> Result<Vec<(String, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, adjusted_reason FROM plan_workouts WHERE adjusted_reason IS NOT NULL",
        )?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        Ok(rows.flatten().collect())
    }

    /// Rewrite a planned day. The date is part of the row: a caller that
    /// reschedules a session (or a test that re-dates one to simulate the
    /// past) must not have the change silently dropped while every other
    /// field lands.
    ///
    /// `planned_date` is written too, and callers must pass it through
    /// unchanged — it is the day the plan generated, not a field anything
    /// downstream is allowed to move.
    pub fn update_workout(&self, w: &PlanWorkout) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE plan_workouts SET plan_id=?2, day=?3, planned_date=?4, kind=?5,
                title=?6, distance_m=?7, duration_min=?8, pace_lo_s_km=?9,
                pace_hi_s_km=?10, done=?11, status=?12, moved_ts=?13, customization_json=?14
             WHERE id=?1",
            rusqlite::params_from_iter(workout_params(w)),
        )?;
        conn.execute(Self::REV_BUMP, params![w.id])?;
        Ok(())
    }

    pub fn workouts_between(&self, from: &str, to: &str) -> Result<Vec<PlanWorkout>> {
        let conn = self.conn.lock().unwrap();
        let sql =
            format!("SELECT {WORKOUT_COLS} FROM plan_workouts WHERE day BETWEEN ?1 AND ?2 ORDER BY day");
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![from, to], workout_from_row)?;
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
        conn.execute(Self::REV_BUMP_PLAN, params![plan_id])?;
        Ok(())
    }

    pub fn accept_pacing_proposal(&self, plan: &TrainingPlan, proposal: &PlanProposal) -> Result<()> {
        let offset = proposal.plan_quality_offset_sec_km.ok_or_else(|| AppError::Other("missing pace adjustment".into()))?;
        let from = proposal.plan_quality_offset_from.ok_or_else(|| AppError::Other("missing adjustment date".into()))?;
        let mut conn = self.conn.lock().unwrap();
        let transaction = conn.transaction()?;
        let changed = transaction.execute(
            "UPDATE plan_proposals SET status='applied', acted_ts=?2 WHERE id=?1 AND status='pending' AND source='pacing'",
            params![proposal.id, Utc::now().timestamp()],
        )?;
        if changed != 1 { return Err(AppError::Other("this offer was already answered".into())); }
        let changed = transaction.execute(
            "UPDATE training_plans SET quality_offset_sec_km=?2, quality_offset_from=?3 WHERE id=?1 AND quality_offset_sec_km=?4 AND status IN ('draft','active','pushed')",
            params![plan.id, offset, from.to_string(), plan.quality_offset_sec_km],
        )?;
        if changed != 1 { return Err(AppError::Other("the plan changed; refresh the proposal".into())); }
        for edit in &proposal.edits {
            let changed = transaction.execute(
                "UPDATE plan_workouts SET distance_m=?2, duration_min=?3 WHERE id=?1 AND plan_id=?4 AND done=0 AND status='planned' AND day=?5 AND kind=?6 AND title=?7 AND distance_m=?8 AND NOT EXISTS (SELECT 1 FROM calendar_completions WHERE workout_id=?1)",
                params![edit.workout_id, edit.distance_m, edit.duration_min, plan.id, edit.date.map(|date| date.to_string()), kind_name(edit.kind), edit.old_title, edit.old_distance_m],
            )?;
            if changed != 1 { return Err(AppError::Other("a proposed workout changed; refresh before applying".into())); }
        }
        transaction.execute(Self::REV_BUMP_PLAN, params![plan.id])?;
        transaction.commit()?;
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

    /// Coros activities whose FIT download is recorded in the ledger yet
    /// produced no track points — a truncated or GPS-less file that decoded
    /// without error stays out of [`Store::activities_missing_track`] forever,
    /// which is how runs end up permanently without a GPS route. Newest first,
    /// capped by `limit` so a backfill cannot flood the daily FIT quota.
    ///
    /// Retries are bounded per activity: a strength session's FIT will never
    /// carry positions, and re-downloading it every sync would burn the
    /// provider's daily file quota for nothing. After [`TRACK_RETRY_LIMIT`]
    /// attempts the activity is left alone.
    pub fn coros_track_backfill_candidates(&self, limit: i64) -> Result<Vec<(String, String, Option<i64>)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT a.provider_record_id, a.sport, a.provider_sport_code
             FROM activities a
             JOIN fit_file_ledger l
               ON l.provider = a.provider AND l.provider_record_id = a.provider_record_id
             LEFT JOIN fit_track_retry r
               ON r.provider = a.provider AND r.provider_record_id = a.provider_record_id
             WHERE a.provider = 'coros'
               AND NOT EXISTS (SELECT 1 FROM trackpoints t WHERE t.activity_id = a.id)
               AND COALESCE(r.attempts, 0) < ?2
             ORDER BY a.start_ts DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit, TRACK_RETRY_LIMIT], |r| {
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

    /// Coros activities whose FIT is in the ledger but whose elevation never
    /// reached the row: the upsert's conflict clause used to update only the
    /// sport code, so the FIT's ascent — which the COROS text report never
    /// carries — was decoded and thrown away on every download. The file is
    /// the only place the device's climb total survives, so those activities
    /// are re-fetched a few per sync, newest first, each exactly once via
    /// [`Store::record_elevation_backfill`]: a FIT that genuinely has no
    /// ascent must not burn the provider's daily file quota on every sync
    /// afterwards. Only activities that actually have a track are offered —
    /// a strength session never had a climb to record.
    pub fn coros_elevation_backfill_candidates(
        &self,
        limit: i64,
    ) -> Result<Vec<(String, String, Option<i64>)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT a.provider_record_id, a.sport, a.provider_sport_code
             FROM activities a
             JOIN fit_file_ledger l
               ON l.provider = a.provider AND l.provider_record_id = a.provider_record_id
             LEFT JOIN fit_elevation_backfill b
               ON b.provider = a.provider AND b.provider_record_id = a.provider_record_id
             WHERE a.provider = 'coros'
               AND a.elevation_gain_m IS NULL
               AND b.provider_record_id IS NULL
               AND EXISTS (SELECT 1 FROM trackpoints t WHERE t.activity_id = a.id)
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

    /// Record that an activity's FIT was already re-fetched once for its
    /// elevation, so a second attempt never happens — whatever the file
    /// contained.
    pub fn record_elevation_backfill(&self, provider: ProviderKind, record_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO fit_elevation_backfill (provider, provider_record_id, tried_ts)
             VALUES (?1,?2,?3)",
            params![provider_name(provider), record_id, Utc::now().timestamp()],
        )?;
        Ok(())
    }

    /// Count one re-download attempt for a ledgered-but-trackless activity and
    /// return the attempts now recorded. The ledger entry stays: deleting it
    /// would push the activity back into [`Store::activities_missing_track`],
    /// which has no bound, and a GPS-less session would be re-fetched on every
    /// sync forever.
    pub fn record_fit_retry(&self, provider: ProviderKind, record_id: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO fit_track_retry (provider, provider_record_id, attempts)
             VALUES (?1,?2,1)
             ON CONFLICT (provider, provider_record_id) DO UPDATE SET attempts = attempts + 1",
            params![provider_name(provider), record_id],
        )?;
        let attempts: i64 = conn.query_row(
            "SELECT attempts FROM fit_track_retry WHERE provider=?1 AND provider_record_id=?2",
            params![provider_name(provider), record_id],
            |r| r.get(0),
        )?;
        Ok(attempts)
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
            "SELECT ts, lat, lon, hr, alt FROM trackpoints
             WHERE activity_id=?1 AND rowid % ?2 = 0 ORDER BY ts",
        )?;
        let rows = stmt.query_map(params![activity_id, every], |r| {
            Ok(TrackPoint {
                ts: r.get(0)?,
                lat: r.get(1)?,
                lon: r.get(2)?,
                hr: r.get(3)?,
                alt: r.get(4)?,
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
                        alt: None,
                    },
                    crate::fit::TrackPoint {
                        ts: ts + 60,
                        lat: 52.51,
                        lon: 13.41,
                        hr: None,
                        alt: None,
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

    /// The COROS text report carries no elevation, so the summary row is
    /// always inserted with elevation NULL and the FIT upsert — the only pass
    /// that knows the ascent — arrives later and hits the conflict path. The
    /// clause used to update only the sport code, so the decoded climb was
    /// thrown away and every outdoor run read "Elevation gain: Not recorded".
    /// The re-sync must lift ascent and max HR onto the row, and a later
    /// elevation-less re-sync must not wipe them again.
    #[test]
    fn fit_resync_backfills_elevation_and_max_hr() {
        let s = store();
        let day = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
        let ts = day.and_hms_opt(9, 0, 0).unwrap().and_utc().timestamp();
        let base = crate::fit::DecodedActivity {
            record_id: format!("{day}-elev"),
            sport: "run".into(),
            provider_sport_code: Some(100),
            start_ts: ts,
            duration_s: 3000.0,
            distance_m: 10_000.0,
            avg_hr: Some(150),
            max_hr: None,
            elevation_gain_m: None,
            points: Vec::new(),
        };
        s.upsert_activity(ProviderKind::Coros, &base).expect("summary row");

        // The FIT pass: same record id, now carrying ascent and max HR.
        s.upsert_activity(
            ProviderKind::Coros,
            &crate::fit::DecodedActivity {
                elevation_gain_m: Some(450.0),
                max_hr: Some(171),
                ..base.clone()
            },
        )
        .expect("fit upsert");
        let id = activity_id(&s, &format!("{day}-elev"));
        let act = s.activity_by_id(id).expect("get").expect("row");
        assert_eq!(act.elevation_gain_m, Some(450.0), "FIT ascent must land");
        assert_eq!(act.max_hr, Some(171), "FIT max HR must land");

        // A later text re-sync knows neither value; it must not erase them.
        s.upsert_activity(ProviderKind::Coros, &base).expect("text re-sync");
        let act = s.activity_by_id(id).expect("get").expect("row");
        assert_eq!(act.elevation_gain_m, Some(450.0), "re-sync must not wipe ascent");
        assert_eq!(act.max_hr, Some(171), "re-sync must not wipe max HR");
    }

    /// The elevation repair re-fetches a FIT at most once per activity: a
    /// ledgered activity with a track and NULL elevation is a candidate, one
    /// already tried is not, and one that already has an elevation (or no
    /// track at all — a strength FIT never had a climb) never enters the
    /// queue, because the provider's download quota is ten files per day.
    #[test]
    fn elevation_backfill_offers_each_ledgered_track_once() {
        let s = store();
        let day = NaiveDate::from_ymd_opt(2026, 4, 2).unwrap();
        let ts = day.and_hms_opt(9, 0, 0).unwrap().and_utc().timestamp();
        let tracked = |rid: String, elev: Option<f64>| crate::fit::DecodedActivity {
            record_id: rid,
            sport: "run".into(),
            provider_sport_code: Some(100),
            start_ts: ts,
            duration_s: 1800.0,
            distance_m: 6000.0,
            avg_hr: None,
            max_hr: None,
            elevation_gain_m: elev,
            points: vec![
                crate::fit::TrackPoint { ts, lat: 38.7, lon: -9.1, hr: None, alt: Some(10.0) },
                crate::fit::TrackPoint { ts: ts + 60, lat: 38.71, lon: -9.11, hr: None, alt: Some(20.0) },
            ],
        };
        // Candidate: FIT ledgered, track present, elevation never stored.
        s.upsert_activity(ProviderKind::Coros, &tracked("needs-elev".into(), None))
            .expect("a");
        s.mark_fit_downloaded(ProviderKind::Coros, "needs-elev").expect("ledger");
        // Not a candidate: elevation already known.
        s.upsert_activity(ProviderKind::Coros, &tracked("has-elev".into(), Some(80.0)))
            .expect("b");
        s.mark_fit_downloaded(ProviderKind::Coros, "has-elev").expect("ledger");
        // Not a candidate: no track, so the plain track loops own it.
        s.upsert_activity(
            ProviderKind::Coros,
            &crate::fit::DecodedActivity {
                points: Vec::new(),
                ..tracked("no-track".into(), None)
            },
        )
        .expect("c");
        s.mark_fit_downloaded(ProviderKind::Coros, "no-track").expect("ledger");

        let cands = s.coros_elevation_backfill_candidates(10).expect("candidates");
        assert_eq!(cands.len(), 1, "only the trackless-elevation row queues");
        assert_eq!(cands[0].0, "needs-elev");

        // Once tried, never again — whatever the re-fetched file contained.
        s.record_elevation_backfill(ProviderKind::Coros, "needs-elev").expect("try");
        assert!(s.coros_elevation_backfill_candidates(10).expect("after").is_empty());
    }

    /// End-to-end check against a copy of the developer's real synced data:
    /// migration 3 applies to an existing DB, the mileage figures are
    /// plausible, and a plan generated from them opens at that volume.
    /// Run with `cargo test -- --ignored --nocapture`.
    #[test]
    #[ignore = "reads the real app database"]
    fn real_db_fit_duplicates_are_merged() {
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
        let dst = std::env::temp_dir().join(format!("rk-fitdup-real-{}.sqlite", std::process::id()));
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
        let s = Store::open(&dst).expect("open runs the merge");
        let left: i64 = s
            .conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM activities
                  WHERE provider='coros' AND provider_record_id GLOB '*[0-9]_[0-9a-f]*'",
                [],
                |r| r.get(0),
            )
            .expect("count");
        let trackless: i64 = s
            .conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM activities a
                  WHERE a.provider='coros'
                    AND NOT EXISTS (SELECT 1 FROM trackpoints t WHERE t.activity_id=a.id)
                    AND lower(a.sport) IN ('run','ride','cycling','hike')",
                [],
                |r| r.get(0),
            )
            .expect("count");
        println!("suffixed rows left = {left}, trackless gps sports = {trackless}");
        assert_eq!(left, 0, "every FIT duplicate merged into its original");
        let _ = std::fs::remove_file(&dst);
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
            training_method: crate::types::TrainingMethod::Norwegian,
            start_date: None,
            distance: crate::types::RaceDistance::K21,
            kind: crate::types::PlanKind::Race,
            target_date: Utc::now().date_naive() + chrono::Duration::days(70),
            threshold_sec_km: 285.0,
            weekly_frequency: 4,
            longrun_day: 6,
            quality_days: vec![],
            increase: crate::types::IncreaseMethod::Moderate,
            current_weekly_km: None,
            name: None,
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

    /// A database already at the newest `user_version` whose `trackpoints`
    /// table never gained `alt` (the drift migration 11 cannot reach, since it
    /// only runs below version 11) made opening any activity fail with
    /// "no such column: alt". The repair must add the column on open and stay
    /// idempotent on reopen.
    #[test]
    fn migrate_repairs_trackpoints_missing_alt() {
        let path =
            std::env::temp_dir().join(format!("rk-alt-drift-{}.sqlite", uuid::Uuid::new_v4()));
        {
            let conn = Connection::open(&path).expect("open raw");
            conn.execute_batch(&format!(
                "CREATE TABLE trackpoints (
                    activity_id INTEGER NOT NULL,
                    ts INTEGER NOT NULL,
                    lat REAL NOT NULL,
                    lon REAL NOT NULL,
                    hr INTEGER
                 );
                 INSERT INTO trackpoints VALUES (1, 100, 1.0, 2.0, 150);
                 PRAGMA user_version={};",
                MIGRATIONS.last().unwrap().0
            ))
            .expect("seed drifted schema");
            assert!(conn.prepare("SELECT alt FROM trackpoints").is_err());
        }
        let s = Store::open(&path).expect("open repairs the drifted schema");
        // The drifted row survives the ALTER and the track query that was
        // failing now reads it back with alt = None.
        let points = s.track_points(1, 100).expect("track_points");
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].alt, None);
        drop(s);
        let again = Store::open(&path).expect("reopen is idempotent");
        assert_eq!(again.track_points(1, 100).expect("track_points").len(), 1);
        let _ = std::fs::remove_file(&path);
    }

    /// The FIT download bug stored a run under the blob's filename
    /// ("<fileid>_8hex.fit") instead of the requested labelId, so the track
    /// landed on a duplicate row while the synced summary row stayed
    /// trackless. Opening the database must move the track to the original,
    /// delete the duplicate, and re-key the ledger so the original never
    /// re-downloads — and it must do this idempotently, since it runs on
    /// every open.
    #[test]
    fn migrate_merges_coros_fit_duplicates_onto_their_original() {
        let path =
            std::env::temp_dir().join(format!("rk-fitdup-{}.sqlite", uuid::Uuid::new_v4()));
        let s = Store::open(&path).expect("open store");
        let ts = 1_793_253_674i64;
        // The summary row from the record sync: labelId key, no track.
        let summary = crate::fit::DecodedActivity {
            record_id: "481373822608507082".into(),
            sport: "run".into(),
            provider_sport_code: Some(100),
            start_ts: ts,
            duration_s: 3600.0,
            distance_m: 10_120.0,
            avg_hr: Some(150),
            max_hr: None,
            elevation_gain_m: None,
            points: Vec::new(),
        };
        s.upsert_activity(ProviderKind::Coros, &summary)
            .expect("summary row");
        // The duplicate the old code created: file-id key, holds the track.
        let dup = crate::fit::DecodedActivity {
            record_id: "481373827708780651_a7c4417262".into(),
            sport: "1".into(),
            provider_sport_code: None,
            start_ts: ts,
            duration_s: 3600.0,
            distance_m: 10_120.0,
            avg_hr: Some(150),
            max_hr: None,
            elevation_gain_m: None,
            points: vec![
                crate::fit::TrackPoint { ts, lat: 38.7, lon: -9.1, hr: Some(140), alt: None },
                crate::fit::TrackPoint { ts: ts + 1, lat: 38.71, lon: -9.11, hr: Some(142), alt: None },
            ],
        };
        s.upsert_activity(ProviderKind::Coros, &dup)
            .expect("duplicate row");
        s.mark_fit_downloaded(ProviderKind::Coros, "481373827708780651_a7c4417262")
            .expect("ledger entry under the suffixed id");
        let summary_id = activity_id(&s, "481373822608507082");
        drop(s);

        let s = Store::open(&path).expect("open runs the merge");
        // The duplicate is gone; the original carries the track.
        assert!(activity_id_opt(&s, "481373827708780651_a7c4417262").is_none());
        let merged_id = activity_id(&s, "481373822608507082");
        let merged = s.activity_by_id(merged_id).expect("get").expect("original survives");
        assert!(merged.has_track, "track moved onto the original");
        assert_eq!(s.track_points(merged.id, 100).expect("track").len(), 2);
        // The ledger now names the original, so it cannot re-download.
        assert!(s.fit_downloaded(ProviderKind::Coros, "481373822608507082").expect("ledger"));
        assert!(!s
            .fit_downloaded(ProviderKind::Coros, "481373827708780651_a7c4417262")
            .expect("ledger"));
        // The original no longer qualifies for a track download.
        assert!(s.activities_missing_track(10).expect("missing").is_empty());
        assert_eq!(merged.id, summary_id, "the original row was kept, not replaced");
        drop(s);

        // Idempotent: reopening must not fail or change anything.
        let s = Store::open(&path).expect("reopen is idempotent");
        assert!(s
            .activity_by_id(activity_id(&s, "481373822608507082"))
            .expect("get")
            .expect("still there")
            .has_track);
        let _ = std::fs::remove_file(&path);
    }

    /// A suffixed row with no same-start twin is left alone: it may be a
    /// legitimately FIT-only activity, and the merge must never delete the
    /// only copy of a track on a guess.
    #[test]
    fn migrate_leaves_unpaired_fit_rows_alone() {
        let path =
            std::env::temp_dir().join(format!("rk-fitlonely-{}.sqlite", uuid::Uuid::new_v4()));
        let s = Store::open(&path).expect("open store");
        let lonely = crate::fit::DecodedActivity {
            record_id: "481373827708780651_a7c4417262".into(),
            sport: "run".into(),
            provider_sport_code: None,
            start_ts: 1_793_253_674,
            duration_s: 3600.0,
            distance_m: 10_120.0,
            avg_hr: None,
            max_hr: None,
            elevation_gain_m: None,
            points: vec![crate::fit::TrackPoint {
                ts: 1_793_253_674,
                lat: 38.7,
                lon: -9.1,
                hr: None,
                alt: None,
            }],
        };
        s.upsert_activity(ProviderKind::Coros, &lonely)
            .expect("insert lonely row");
        drop(s);
        let s = Store::open(&path).expect("open keeps unpaired row");
        let kept = activity_id(&s, "481373827708780651_a7c4417262");
        assert_eq!(s.track_points(kept, 100).expect("track").len(), 1);
        let _ = std::fs::remove_file(&path);
    }

    fn activity_id_opt(store: &Store, rid: &str) -> Option<i64> {
        store
            .conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT id FROM activities WHERE provider='coros' AND provider_record_id=?1",
                params![rid],
                |r| r.get(0),
            )
            .optional()
            .expect("query")
    }

    fn activity_id(store: &Store, rid: &str) -> i64 {
        activity_id_opt(store, rid).unwrap_or_else(|| panic!("no activity for {rid}"))
    }

    /// The adaptive plan's reason must land on the session it explains and be
    /// readable in one query, because the calendar reads every day's events at
    /// once — a reason stored anywhere else is a second thing that can go
    /// missing, and a per-day lookup would be N queries per month view.
    ///
    /// The test opens a database whose `plan_workouts` predates the column,
    /// which is what every existing install looks like: the column arrives as
    /// an open-time repair, not a numbered migration, so a shipped database
    /// must gain it without losing its rows.
    #[test]
    fn soften_workout_records_the_reason_and_survives_the_repair() {
        let path = std::env::temp_dir().join(format!("rk-adjust-{}.sqlite", uuid::Uuid::new_v4()));
        let day = Utc::now().date_naive() + chrono::Duration::days(3);

        // Seed the pre-repair schema: a plan_workouts row without the column.
        // The parent plan is seeded too — `open` turns foreign keys on, so a
        // workout naming a plan that never existed would be rejected.
        {
            let conn = Connection::open(&path).expect("open raw");
            conn.execute_batch(
                "CREATE TABLE training_plans (
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
                 INSERT INTO training_plans
                    VALUES ('p1','K10','2026-08-01','active',4,6,'moderate',285.0,1735948800,NULL);
                 CREATE TABLE plan_workouts (
                    id TEXT PRIMARY KEY,
                    plan_id TEXT NOT NULL REFERENCES training_plans (id) ON DELETE CASCADE,
                    day TEXT NOT NULL,
                    planned_date TEXT,
                    kind TEXT NOT NULL,
                    title TEXT NOT NULL,
                    distance_m REAL,
                    duration_min REAL,
                    pace_lo_s_km INTEGER,
                    pace_hi_s_km INTEGER,
                    done INTEGER NOT NULL DEFAULT 0,
                    status TEXT NOT NULL DEFAULT 'planned',
                    moved_ts INTEGER
                 );
                 INSERT INTO plan_workouts (id, plan_id, day, kind, title, distance_m)
                    VALUES ('w1','p1','2026-06-01','tempo','6 x 1 km tempo',8000.0);",
            )
            .expect("seed pre-repair schema");
            assert!(conn
                .prepare("SELECT adjusted_reason FROM plan_workouts")
                .is_err());
        }

        let s = Store::open(&path).expect("open repairs the schema");
        // The repair kept the row and made the column readable as NULL.
        assert!(s.workout_adjustments().expect("adjustments").is_empty());

        s.soften_workout("w1", WorkoutKind::Easy, "Easy run (was: 6 x 1 km tempo)", "recovery score 38/100")
            .expect("soften_workout");
        assert_eq!(
            s.workout_adjustments().expect("adjustments"),
            vec![("w1".to_string(), "recovery score 38/100".to_string())],
            "the reason reads back with the session it explains"
        );

        // A session nobody softened never appears, so the calendar's map
        // stays sparse and a clean month shows no banner.
        let quiet = PlanWorkout {
            id: "w2".into(),
            plan_id: "p1".into(),
            date: day,
            planned_date: day,
            kind: WorkoutKind::LongRun,
            title: "Long run 18 km".into(),
            distance_m: Some(18000.0),
            duration_min: None,
            pace_lo_s_km: None,
            pace_hi_s_km: None,
            done: false,
            status: WorkoutStatus::Planned,
            moved_at: None,
            customization: None,
        };
        s.update_workout(&quiet).expect("seed second workout");
        assert_eq!(s.workout_adjustments().expect("still one").len(), 1);

        // Reopening must not fail on the column that is now there.
        drop(s);
        let again = Store::open(&path).expect("reopen is idempotent");
        assert_eq!(again.workout_adjustments().expect("adjustments").len(), 1);
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
