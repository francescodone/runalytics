//! Row types and the conversions between rows and domain values.
//!
//! Rows are deliberately *not* the domain types. A `Plan` in memory owns its
//! weeks and sessions; a `PlanRow` is one flat database record. Keeping them
//! separate means a schema change touches this file and nowhere else, and the
//! domain types never carry serialisation concerns they do not need.

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use rusqlite::{OptionalExtension, Row, params};

use crate::error::{Result, StoreError};
use runalytics_core::{Date, Timestamp};

/// Parse a `YYYY-MM-DD` column.
pub fn parse_date(text: &str) -> Result<Date> {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").map_err(|e| StoreError::InvalidValue {
        field: "date",
        value: format!("{text} ({e})"),
    })
}

/// Render a calendar day for storage.
#[must_use]
pub fn write_date(date: Date) -> String {
    date.format("%Y-%m-%d").to_string()
}

/// Parse a wall-clock time, tolerating both `HH:MM:SS` and `HH:MM`.
pub fn parse_time(text: &str) -> Result<NaiveTime> {
    NaiveTime::parse_from_str(text, "%H:%M:%S")
        .or_else(|_| NaiveTime::parse_from_str(text, "%H:%M"))
        .map_err(|e| StoreError::InvalidValue {
            field: "time",
            value: format!("{text} ({e})"),
        })
}

#[must_use]
pub fn write_time(time: NaiveTime) -> String {
    time.format("%H:%M:%S").to_string()
}

/// Parse an RFC-3339 timestamp column.
pub fn parse_timestamp(text: &str) -> Result<Timestamp> {
    chrono::DateTime::parse_from_rfc3339(text)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .or_else(|_| {
            NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%S").map(|naive| naive.and_utc())
        })
        .map_err(|e| StoreError::InvalidValue {
            field: "timestamp",
            value: format!("{text} ({e})"),
        })
}

#[must_use]
pub fn write_timestamp(ts: Timestamp) -> String {
    ts.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Current instant, formatted for storage.
#[must_use]
pub fn now() -> Timestamp {
    chrono::Utc::now()
}

/// Read a `TEXT PRIMARY KEY` back into a domain id.
pub fn read_id<T>(text: &str) -> Result<T>
where
    T: std::str::FromStr<Err = uuid::Error>,
{
    text.parse::<T>().map_err(|e| StoreError::InvalidValue {
        field: "id",
        value: format!("{text} ({e})"),
    })
}

/// Read a JSON column, with the column name in the error so a corrupt row is
/// diagnosable from the message alone.
pub fn read_json<T>(row: &Row<'_>, column: &str) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let text: String = row.get(column)?;
    serde_json::from_str(&text).map_err(|e| {
        StoreError::CorruptJson(serde_json::Error::io(std::io::Error::other(format!(
            "{column}: {e}"
        ))))
    })
}

/// Read an optional JSON column, treating SQL NULL and empty string as absent.
pub fn read_json_opt<T>(row: &Row<'_>, column: &str) -> Result<Option<T>>
where
    T: serde::de::DeserializeOwned,
{
    let text: Option<String> = row.get(column)?;
    match text.as_deref() {
        None | Some("") => Ok(None),
        Some(text) => serde_json::from_str(text).map(Some).map_err(|e| {
            StoreError::CorruptJson(serde_json::Error::io(std::io::Error::other(format!(
                "{column}: {e}"
            ))))
        }),
    }
}

/// Render a value for a JSON column.
pub fn write_json<T: serde::Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value).map_err(StoreError::from)
}

/// SQLite stores booleans as integers.
#[must_use]
pub fn write_bool(value: bool) -> i64 {
    i64::from(value)
}

#[must_use]
pub fn read_bool(row: &Row<'_>, column: &str) -> bool {
    row.get::<_, i64>(column).unwrap_or(0) != 0
}

/// Read a nullable numeric column as an `Option<f64>` heart-rate-style integer.
pub fn read_optional_u16(row: &Row<'_>, column: &str) -> Option<u16> {
    row.get::<_, Option<i64>>(column)
        .ok()
        .flatten()
        .map(|v| v as u16)
}

#[must_use]
pub fn write_optional_u16(value: Option<u16>) -> Option<i64> {
    value.map(i64::from)
}

/// Convenience for `SELECT 1 FROM ... LIMIT 1` existence checks.
pub fn exists(conn: &rusqlite::Connection, sql: &str, id: &str) -> Result<bool> {
    let found: Option<i64> = conn
        .query_row(sql, params![id], |row| row.get(0))
        .optional()?;
    Ok(found.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_round_trip() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 8).expect("date");
        assert_eq!(write_date(date), "2026-10-08");
        assert_eq!(parse_date("2026-10-08").expect("parse"), date);
    }

    #[test]
    fn times_accept_the_short_form() {
        assert!(parse_time("07:00:00").is_ok());
        assert!(parse_time("07:00").is_ok());
        assert!(parse_time("nope").is_err());
    }

    #[test]
    fn timestamps_accept_offset_and_bare_forms() {
        assert!(parse_timestamp("2026-10-01T07:00:00Z").is_ok());
        assert!(parse_timestamp("2026-10-01T07:00:00+02:00").is_ok());
        assert!(parse_timestamp("2026-10-01T07:00:00").is_ok());
        assert!(parse_timestamp("2026-10-01").is_err());
    }

    #[test]
    fn offset_timestamps_normalise_to_utc() {
        let ts = parse_timestamp("2026-10-01T09:00:00+02:00").expect("parse");
        assert_eq!(write_timestamp(ts), "2026-10-01T07:00:00Z");
    }

    #[test]
    fn corrupt_json_names_the_column() {
        let conn = rusqlite::Connection::open_in_memory().expect("conn");
        conn.execute("CREATE TABLE t (payload TEXT)", [])
            .expect("create");
        conn.execute("INSERT INTO t VALUES ('{oops')", [])
            .expect("insert");
        let mut stmt = conn.prepare("SELECT payload FROM t").expect("prepare");
        let mut rows = stmt
            .query_and_then([], |row| read_json::<serde_json::Value>(row, "payload"))
            .expect("query");
        let err = rows
            .next()
            .expect("one row")
            .expect_err("read_json must fail");
        assert!(
            err.to_string().contains("payload"),
            "error names the column"
        );
    }
}
