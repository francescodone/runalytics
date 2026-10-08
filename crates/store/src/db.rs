//! Connection handling.
//!
//! `Db` is cheap to clone and shareable across Tauri's async command threads,
//! which is why the connection sits behind a `Mutex` rather than a pool: SQLite
//! in WAL mode serialises writers anyway, and a single connection keeps the
//! "read your own write" behaviour the sync path depends on.

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::Connection;

use crate::error::{Result, StoreError};
use crate::migrations;

/// A shared, migrated SQLite database.
#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

impl Db {
    /// Open (or create) a database at `path` and migrate it.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mut conn = Connection::open(path)?;
        configure(&mut conn)?;
        migrations::migrate(&mut conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// An in-memory database, used by tests and by the plan preview flow where
    /// nothing should be persisted.
    pub fn in_memory() -> Result<Self> {
        let mut conn = Connection::open_in_memory()?;
        configure(&mut conn)?;
        migrations::migrate(&mut conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Borrow the connection.
    ///
    /// The lock is poisoned only if a repository panicked mid-statement; in
    /// that case the database is still consistent because every write path runs
    /// inside a transaction, so recovering the guard is safe and better than
    /// taking the whole app down.
    pub(crate) fn conn(&self) -> MutexGuard<'_, Connection> {
        self.conn
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Run `f` inside a transaction, committing on `Ok`.
    ///
    /// Multi-row writes (a plan with its weeks and sessions, a batch of
    /// activities) must land atomically: a half-written plan renders as a
    /// corrupt week in the UI.
    pub fn transaction<T>(
        &self,
        f: impl FnOnce(&mut rusqlite::Transaction<'_>) -> Result<T>,
    ) -> Result<T> {
        let mut guard = self.conn();
        let mut tx = guard.transaction()?;
        let out = f(&mut tx)?;
        tx.commit()?;
        Ok(out)
    }

    /// Referential integrity state, surfaced in Settings as a health check.
    pub fn check_integrity(&self) -> Result<()> {
        let guard = self.conn();
        let report: String = guard.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        if report != "ok" {
            return Err(StoreError::InvalidValue {
                field: "integrity_check",
                value: report,
            });
        }
        Ok(())
    }

    /// Whether foreign-key enforcement is active on this connection.
    pub fn foreign_keys_enabled(&self) -> Result<bool> {
        let guard = self.conn();
        let enabled: i64 = guard.query_row("PRAGMA foreign_keys", [], |r| r.get(0))?;
        Ok(enabled != 0)
    }

    /// File size on disk, surfaced in Settings as a sync-health indicator.
    pub fn page_count_bytes(&self) -> Result<i64> {
        let guard = self.conn();
        let pages: i64 = guard.query_row("PRAGMA page_count", [], |r| r.get(0))?;
        let size: i64 = guard.query_row("PRAGMA page_size", [], |r| r.get(0))?;
        Ok(pages * size)
    }
}

fn configure(conn: &mut Connection) -> Result<()> {
    // WAL: the MCP server and the desktop app read the same file concurrently.
    conn.pragma_update(None, "journal_mode", "WAL")?;
    // Readers never block the writer, and a 5 s busy timeout covers a normal
    // sync batch without surfacing SQLITE_BUSY to the user.
    conn.pragma_update(None, "busy_timeout", 5_000)?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    // Enforced, not advisory: cascade deletes in the schema rely on this.
    let enabled: i64 = conn.query_row("PRAGMA foreign_keys", [], |r| r.get(0))?;
    if enabled == 0 {
        return Err(StoreError::InvalidValue {
            field: "foreign_keys",
            value: "off".into(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clones_share_one_database() {
        let db = Db::in_memory().expect("db");
        let clone = db.clone();
        db.conn()
            .execute(
                "INSERT INTO athlete (id, timezone, profile_json, created_at, updated_at)
                 VALUES ('a', 'Europe/Madrid', '{}', 'x', 'x')",
                [],
            )
            .expect("insert");
        let count: i64 = clone
            .conn()
            .query_row("SELECT COUNT(*) FROM athlete", [], |r| r.get(0))
            .expect("count");
        assert_eq!(count, 1, "a clone must see the same data");
    }

    #[test]
    fn transaction_rolls_back_on_error() {
        let db = Db::in_memory().expect("db");
        let result: Result<()> = db.transaction(|tx| {
            tx.execute(
                "INSERT INTO athlete (id, timezone, profile_json, created_at, updated_at)
                 VALUES ('b', 'Europe/Madrid', '{}', 'x', 'x')",
                [],
            )?;
            Err(StoreError::NotFound {
                entity: "test",
                id: "boom".into(),
            })
        });
        assert!(result.is_err());
        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM athlete", [], |r| r.get(0))
            .expect("count");
        assert_eq!(count, 0, "the failed transaction must not persist");
    }

    #[test]
    fn file_database_round_trips() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("runalytics.sqlite");
        {
            let db = Db::open(&path).expect("open");
            db.conn()
                .execute(
                    "INSERT INTO athlete (id, timezone, profile_json, created_at, updated_at)
                     VALUES ('c', 'Europe/Madrid', '{}', 'x', 'x')",
                    [],
                )
                .expect("insert");
        }
        let reopened = Db::open(&path).expect("reopen");
        let count: i64 = reopened
            .conn()
            .query_row("SELECT COUNT(*) FROM athlete", [], |r| r.get(0))
            .expect("count");
        assert_eq!(count, 1);
    }
}
