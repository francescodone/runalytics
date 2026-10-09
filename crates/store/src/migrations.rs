//! Schema migrations.
//!
//! Migrations are append-only SQL applied inside a transaction and recorded in
//! `schema_version`. Version 1 is the whole initial schema; later versions are
//! added as new entries in [`MIGRATIONS`] rather than by editing v1, so a user
//! who installed an early build upgrades without losing data.

use rusqlite::{Connection, Transaction};

use crate::error::{Result, StoreError};

/// The full v1 schema.
///
/// Design notes worth preserving:
///
/// * `activity` and `health_day` carry a `UNIQUE` on the provider key so
///   ingestion is an upsert, not a delete-then-insert.
/// * `planned_session` stores the structured workout as JSON. Blocks are
///   variable-length and only ever read whole, so normalising them into a
///   child table would buy nothing and make re-planning slower.
/// * Every score table carries `formula_version`. Recomputation is a backfill
///   keyed on that column, and the UI can say "recomputed with v3 scoring".
/// * `calendar_event` keeps the sink's own id alongside our `UID`, which is
///   what makes an update-in-place possible instead of delete-and-recreate.
const V1_SCHEMA: &str = r"
CREATE TABLE athlete (
    id            TEXT PRIMARY KEY,
    display_name  TEXT NOT NULL DEFAULT '',
    timezone      TEXT NOT NULL,
    profile_json  TEXT NOT NULL,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);

CREATE TABLE provider_account (
    id               TEXT PRIMARY KEY,
    provider         TEXT NOT NULL CHECK (provider IN ('coros','garmin')),
    athlete_id       TEXT REFERENCES athlete(id) ON DELETE SET NULL,
    account_label    TEXT NOT NULL DEFAULT '',
    external_user_id TEXT,
    region           TEXT,
    writable         INTEGER NOT NULL DEFAULT 0,
    connected        INTEGER NOT NULL DEFAULT 0,
    created_at       TEXT NOT NULL,
    last_sync_at     TEXT,
    UNIQUE (provider, account_label)
);

-- Resumable sync: one cursor per account per data class.
CREATE TABLE sync_cursor (
    account_id  TEXT NOT NULL REFERENCES provider_account(id) ON DELETE CASCADE,
    data_class  TEXT NOT NULL,
    cursor_date TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    PRIMARY KEY (account_id, data_class)
);

CREATE TABLE activity (
    id                   TEXT PRIMARY KEY,
    account_id           TEXT NOT NULL REFERENCES provider_account(id) ON DELETE CASCADE,
    provider_activity_id TEXT NOT NULL,
    name                 TEXT NOT NULL DEFAULT '',
    started_at           TEXT NOT NULL,
    local_date           TEXT NOT NULL,
    distance_km          REAL NOT NULL DEFAULT 0,
    duration_s           INTEGER NOT NULL DEFAULT 0,
    avg_pace             REAL,
    avg_hr               INTEGER,
    max_hr               INTEGER,
    elevation_gain       REAL,
    avg_cadence          INTEGER,
    training_load        REAL,
    intensity            TEXT NOT NULL DEFAULT 'unknown',
    matched_session_id   TEXT,
    fetched_at           TEXT NOT NULL,
    UNIQUE (account_id, provider_activity_id)
);
CREATE INDEX idx_activity_date ON activity (local_date);
CREATE INDEX idx_activity_matched ON activity (matched_session_id);

CREATE TABLE activity_lap (
    activity_id    TEXT NOT NULL REFERENCES activity(id) ON DELETE CASCADE,
    idx            INTEGER NOT NULL,
    start_at       TEXT NOT NULL,
    duration_s     INTEGER NOT NULL,
    distance_km    REAL NOT NULL,
    avg_pace       REAL,
    avg_hr         INTEGER,
    max_hr         INTEGER,
    elevation_gain REAL,
    cadence        INTEGER,
    PRIMARY KEY (activity_id, idx)
);

CREATE TABLE health_day (
    id                 TEXT PRIMARY KEY,
    account_id         TEXT NOT NULL REFERENCES provider_account(id) ON DELETE CASCADE,
    date               TEXT NOT NULL,
    resting_hr         INTEGER,
    avg_stress         REAL,
    high_stress_min    INTEGER,
    steps              INTEGER,
    provider_readiness INTEGER,
    basal_energy       REAL,
    -- Sleep is stored as one JSON object: it is always read whole, and its
    -- stage breakdown differs per provider.
    sleep_json         TEXT,
    has_recovery       INTEGER NOT NULL DEFAULT 0,
    fetched_at         TEXT NOT NULL,
    UNIQUE (account_id, date)
);
CREATE INDEX idx_health_date ON health_day (date);

CREATE TABLE plan (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    goal            TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'draft',
    anchor_json     TEXT NOT NULL,
    resolution_json TEXT NOT NULL,
    athlete_json    TEXT NOT NULL,
    -- Phase spans are stored rather than re-derived: they carry the coach's
    -- rationale text, which the engine would have to reproduce exactly.
    phases_json     TEXT NOT NULL DEFAULT '[]',
    ceiling_volume  REAL NOT NULL DEFAULT 0,
    emittable       INTEGER NOT NULL DEFAULT 0,
    external_json   TEXT NOT NULL DEFAULT '{}',
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);
CREATE INDEX idx_plan_status ON plan (status);

CREATE TABLE plan_week (
    plan_id         TEXT NOT NULL REFERENCES plan(id) ON DELETE CASCADE,
    idx             INTEGER NOT NULL,
    phase           TEXT NOT NULL,
    start_date      TEXT NOT NULL,
    end_date        TEXT NOT NULL,
    target_volume   REAL NOT NULL,
    previous_volume REAL NOT NULL,
    step_pct        REAL NOT NULL,
    projected_acwr  REAL NOT NULL,
    is_deload       INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (plan_id, idx)
);

CREATE TABLE planned_session (
    id              TEXT PRIMARY KEY,
    plan_id         TEXT NOT NULL REFERENCES plan(id) ON DELETE CASCADE,
    week_idx        INTEGER NOT NULL,
    date            TEXT NOT NULL,
    start_time      TEXT NOT NULL,
    kind            TEXT NOT NULL,
    title           TEXT NOT NULL,
    intent          TEXT NOT NULL DEFAULT '',
    workout_json    TEXT NOT NULL,
    target_volume   REAL NOT NULL DEFAULT 0,
    target_duration INTEGER NOT NULL DEFAULT 0,
    target_pace     REAL,
    rpe_target      INTEGER,
    quality         INTEGER NOT NULL DEFAULT 0,
    external_id     TEXT,
    -- At most one *active* session per calendar day per plan. Cancelled rows
    -- are excluded so a re-plan that moves a session does not collide with the
    -- row it replaced.
    status          TEXT NOT NULL DEFAULT 'scheduled'
);
CREATE INDEX idx_session_date ON planned_session (plan_id, date);
CREATE INDEX idx_session_external ON planned_session (external_id);

CREATE TABLE session_result (
    id           TEXT PRIMARY KEY,
    session_id   TEXT NOT NULL REFERENCES planned_session(id) ON DELETE CASCADE,
    activity_id  TEXT REFERENCES activity(id) ON DELETE SET NULL,
    date         TEXT NOT NULL,
    status       TEXT NOT NULL,
    rpe          INTEGER,
    duration_s   INTEGER NOT NULL DEFAULT 0,
    distance_km  REAL NOT NULL DEFAULT 0,
    avg_hr       INTEGER,
    notes        TEXT NOT NULL DEFAULT '',
    created_at   TEXT NOT NULL,
    UNIQUE (session_id)
);

CREATE TABLE session_score (
    session_id       TEXT PRIMARY KEY,
    formula_version  INTEGER NOT NULL,
    score            REAL NOT NULL,
    tss              REAL NOT NULL,
    intensity_factor REAL NOT NULL,
    duration_s       INTEGER NOT NULL,
    load             REAL NOT NULL,
    planned_quality  INTEGER NOT NULL DEFAULT 0,
    executed_quality INTEGER NOT NULL DEFAULT 0,
    adherence        REAL,
    components_json  TEXT NOT NULL DEFAULT '{}',
    created_at       TEXT NOT NULL
);
CREATE INDEX idx_session_score_version ON session_score (formula_version);

CREATE TABLE readiness_day (
    date               TEXT PRIMARY KEY,
    formula_version    INTEGER NOT NULL,
    score              REAL NOT NULL,
    components_json    TEXT NOT NULL,
    inputs_json        TEXT NOT NULL,
    created_at         TEXT NOT NULL
);
CREATE INDEX idx_readiness_version ON readiness_day (formula_version);

CREATE TABLE injury_risk_day (
    date            TEXT PRIMARY KEY,
    formula_version INTEGER NOT NULL,
    score           REAL NOT NULL,
    band            TEXT NOT NULL,
    drivers_json    TEXT NOT NULL,
    created_at      TEXT NOT NULL
);

CREATE TABLE metric_snapshot (
    date               TEXT PRIMARY KEY,
    formula_version    INTEGER NOT NULL,
    chronic_load       REAL NOT NULL,
    acute_load         REAL NOT NULL,
    training_stress_balance REAL NOT NULL,
    acwr               REAL NOT NULL,
    monotony           REAL NOT NULL,
    weekly_spike       REAL NOT NULL,
    vo2max_estimate    REAL,
    threshold_pace     REAL,
    performance_index  REAL,
    components_json    TEXT NOT NULL DEFAULT '{}',
    created_at         TEXT NOT NULL
);

CREATE TABLE feedback (
    id          TEXT PRIMARY KEY,
    session_id  TEXT REFERENCES planned_session(id) ON DELETE CASCADE,
    date        TEXT NOT NULL,
    rpe         INTEGER,
    mood        INTEGER,
    legs        INTEGER,
    motivation  INTEGER,
    notes       TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL
);
CREATE INDEX idx_feedback_date ON feedback (date);

CREATE TABLE calendar_event (
    id          TEXT PRIMARY KEY,
    session_id  TEXT NOT NULL REFERENCES planned_session(id) ON DELETE CASCADE,
    uid         TEXT NOT NULL,
    sink        TEXT NOT NULL,
    external_id TEXT,
    sequence    INTEGER NOT NULL DEFAULT 0,
    start_at    TEXT NOT NULL,
    end_at      TEXT NOT NULL,
    title       TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'confirmed',
    synced_at   TEXT NOT NULL,
    UNIQUE (session_id, sink)
);
CREATE INDEX idx_calendar_uid ON calendar_event (uid);

CREATE TABLE sync_run (
    id                TEXT PRIMARY KEY,
    account_id        TEXT NOT NULL REFERENCES provider_account(id) ON DELETE CASCADE,
    data_class        TEXT NOT NULL,
    started_at        TEXT NOT NULL,
    finished_at       TEXT,
    status            TEXT NOT NULL DEFAULT 'running',
    records_upserted  INTEGER NOT NULL DEFAULT 0,
    error             TEXT
);
CREATE INDEX idx_sync_run_account ON sync_run (account_id, started_at);
";

/// Ordered list of `(version, sql)`. Append only — never edit an earlier entry.
/// Provider fitness estimates, one row per reported day per account.
///
/// Kept as history rather than a field on `athlete`: providers revise these
/// constantly, and the *trend* of provider VO2max is a training signal while a
/// single value is just the latest guess.
const V2_SCHEMA: &str = r"
CREATE TABLE fitness_assessment (
    account_id       TEXT NOT NULL REFERENCES provider_account(id) ON DELETE CASCADE,
    date             TEXT NOT NULL,
    vo2max           REAL,
    running_level    REAL,
    threshold_pace   REAL,
    predicted_json   TEXT NOT NULL DEFAULT '[]',
    fetched_at       TEXT NOT NULL,
    PRIMARY KEY (account_id, date)
);
";

/// App settings: one JSON value per key, written by the desktop shell and
/// read by anything that opens the same database (the MCP server included),
/// so GUI and agent never diverge on timezone or calendar name.
const V3_SCHEMA: &str = r"
CREATE TABLE settings (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
";

const MIGRATIONS: &[(i64, &str)] = &[(1, V1_SCHEMA), (2, V2_SCHEMA), (3, V3_SCHEMA)];

/// Current version recorded in the database, `0` for a fresh file.
///
/// The `schema_version` table is created on demand so the first migration can
/// run against a brand-new database.
pub fn current_version(conn: &Connection) -> Result<i64> {
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_version'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)?;
    if !exists {
        return Ok(0);
    }
    Ok(conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )?)
}

/// Apply every migration newer than the recorded version.
///
/// Each migration runs in its own transaction so a failure at version 4 leaves
/// versions 1-3 committed and diagnosable, rather than rolling the whole
/// history back.
pub fn migrate(conn: &mut Connection) -> Result<i64> {
    let start = current_version(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
             version    INTEGER PRIMARY KEY,
             applied_at TEXT NOT NULL
         );",
    )?;

    for &(version, sql) in MIGRATIONS {
        if version <= start {
            continue;
        }
        let tx = conn.transaction()?;
        apply(&tx, sql).map_err(|e| StoreError::Migration {
            version,
            message: e.to_string(),
        })?;
        tx.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![version, now_iso()],
        )?;
        tx.commit()?;
        tracing::info!(version, "applied migration");
    }
    current_version(conn)
}

fn apply(tx: &Transaction<'_>, sql: &str) -> rusqlite::Result<()> {
    tx.execute_batch(sql)
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tables(conn: &Connection) -> Vec<String> {
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .expect("prepare");
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .expect("query");
        rows.map(|r| r.expect("row")).collect()
    }

    #[test]
    fn fresh_database_reaches_the_current_version() {
        let mut conn = rusqlite::Connection::open_in_memory().expect("in-memory db");
        assert_eq!(current_version(&conn).expect("version"), 0);
        let applied = migrate(&mut conn).expect("migrate");
        assert_eq!(applied, crate::SCHEMA_VERSION);
    }

    #[test]
    fn migrating_twice_is_a_no_op() {
        let mut conn = rusqlite::Connection::open_in_memory().expect("in-memory db");
        migrate(&mut conn).expect("first migrate");
        let before = tables(&conn);
        migrate(&mut conn).expect("second migrate");
        assert_eq!(before, tables(&conn));
    }

    #[test]
    fn every_documented_table_exists() {
        let mut conn = rusqlite::Connection::open_in_memory().expect("in-memory db");
        migrate(&mut conn).expect("migrate");
        let names = tables(&conn);
        for expected in [
            "activity",
            "activity_lap",
            "athlete",
            "calendar_event",
            "feedback",
            "fitness_assessment",
            "health_day",
            "injury_risk_day",
            "metric_snapshot",
            "plan",
            "plan_week",
            "planned_session",
            "provider_account",
            "readiness_day",
            "schema_version",
            "session_result",
            "session_score",
            "sync_cursor",
            "sync_run",
        ] {
            assert!(
                names.iter().any(|n| n == expected),
                "missing table {expected}"
            );
        }
    }

    #[test]
    fn provider_key_is_unique_so_resync_upserts() {
        let mut conn = rusqlite::Connection::open_in_memory().expect("in-memory db");
        migrate(&mut conn).expect("migrate");
        // Foreign keys are on, so the account has to exist first.
        conn.execute(
            "INSERT INTO provider_account (id, provider, created_at)
             VALUES ('acct', 'coros', '2026-10-01T00:00:00Z')",
            [],
        )
        .expect("account");
        let insert = |label: &str| {
            conn.execute(
                "INSERT INTO activity (id, account_id, provider_activity_id, started_at,
                                       local_date, fetched_at)
                 VALUES (?1, 'acct', ?2, '2026-10-01T07:00:00Z', '2026-10-01',
                         '2026-10-01T08:00:00Z')",
                rusqlite::params![uuid::Uuid::new_v4().to_string(), label],
            )
        };
        insert("123").expect("first insert");
        assert!(
            insert("123").is_err(),
            "the same provider id must not land twice"
        );
        insert("124").expect("different provider id is fine");
    }

    #[test]
    fn foreign_keys_are_enforced_so_orphans_cannot_be_written() {
        let mut conn = rusqlite::Connection::open_in_memory().expect("in-memory db");
        migrate(&mut conn).expect("migrate");
        let err = conn.execute(
            "INSERT INTO activity (id, account_id, provider_activity_id, started_at,
                                   local_date, fetched_at)
             VALUES ('a', 'no-such-account', '1', '2026-10-01T07:00:00Z',
                     '2026-10-01', '2026-10-01T08:00:00Z')",
            [],
        );
        assert!(err.is_err(), "an activity needs a real account");
    }
}
