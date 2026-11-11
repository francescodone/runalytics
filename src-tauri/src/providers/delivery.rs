use rusqlite::{params, OptionalExtension};

use crate::error::{AppError, Result};
use crate::store::Store;

pub(crate) const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS workout_delivery (
    workspace_id TEXT NOT NULL,
    destination TEXT NOT NULL,
    account_id TEXT NOT NULL,
    plan_id TEXT NOT NULL,
    generation INTEGER NOT NULL CHECK(generation > 0),
    desired_revision INTEGER NOT NULL DEFAULT 0 CHECK(desired_revision >= 0),
    desired_hash TEXT NOT NULL,
    desired_action TEXT NOT NULL CHECK(desired_action IN ('push','cancel')),
    confirmed_generation INTEGER,
    remote_id TEXT,
    state TEXT NOT NULL CHECK(state IN ('pending','confirmed','retry')),
    retry_at INTEGER,
    failure_reason TEXT,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY(workspace_id,destination,account_id,plan_id)
);
CREATE TABLE IF NOT EXISTS workout_delivery_cleanup (
    workspace_id TEXT NOT NULL,
    destination TEXT NOT NULL,
    account_id TEXT NOT NULL,
    plan_id TEXT NOT NULL,
    remote_id TEXT NOT NULL,
    generation INTEGER NOT NULL CHECK(generation > 0),
    created_at INTEGER NOT NULL,
    PRIMARY KEY(workspace_id,destination,account_id,plan_id,remote_id),
    FOREIGN KEY(workspace_id,destination,account_id,plan_id)
      REFERENCES workout_delivery(workspace_id,destination,account_id,plan_id) ON DELETE CASCADE
);
";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeliveryAction {
    Push,
    Cancel,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum DeliveryFailure {
    Offline,
    Authentication,
    Provider,
    Verification,
}

impl DeliveryFailure {
    fn as_str(self) -> &'static str {
        match self {
            Self::Offline => "offline",
            Self::Authentication => "authentication",
            Self::Provider => "provider",
            Self::Verification => "verification",
        }
    }
}

impl DeliveryAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Push => "push",
            Self::Cancel => "cancel",
        }
    }
}

pub(crate) struct DeliveryKey<'a> {
    pub workspace: &'a str,
    pub destination: &'a str,
    pub account: &'a str,
    pub plan: &'a str,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct DeliveryState {
    pub generation: i64,
    pub desired_revision: i64,
    pub desired_hash: String,
    pub action: String,
    pub state: String,
    pub confirmed_generation: Option<i64>,
    pub remote_id: Option<String>,
    pub retry_at: Option<i64>,
    pub failure_reason: Option<String>,
}

fn valid_key(key: &DeliveryKey<'_>) -> bool {
    [key.workspace, key.destination, key.account, key.plan]
        .iter()
        .all(|value| !value.trim().is_empty())
}

fn invalid() -> AppError {
    AppError::Other("Invalid workout delivery state.".into())
}

impl Store {
    pub(crate) fn due_deliveries(&self, workspace: &str, destination: &str, account: &str, now: i64) -> Result<Vec<String>> {
        if [workspace,destination,account].iter().any(|value| value.trim().is_empty()) || now < 0 {
            return Err(invalid());
        }
        let connection = self.conn.lock().unwrap();
        let mut statement = connection.prepare(
            "SELECT plan_id FROM workout_delivery
             WHERE workspace_id=?1 AND destination=?2 AND account_id=?3
               AND (state='pending' OR (state='retry' AND retry_at<=?4))
             ORDER BY updated_at,plan_id",
        )?;
        let rows = statement.query_map(params![workspace,destination,account,now], |row| row.get(0))?;
        Ok(rows.collect::<std::result::Result<Vec<_>,_>>()?)
    }

    pub(crate) fn enqueue_delivery(&self, key: &DeliveryKey<'_>, revision: i64, action: DeliveryAction, hash: &str, now: i64) -> Result<i64> {
        if !valid_key(key) || hash.trim().is_empty() || revision < 0 || now < 0 {
            return Err(invalid());
        }
        let mut connection = self.conn.lock().unwrap();
        let transaction = connection.transaction()?;
        let previous: Option<(i64,String,String)> = transaction.query_row(
            "SELECT desired_revision,desired_action,desired_hash FROM workout_delivery
             WHERE workspace_id=?1 AND destination=?2 AND account_id=?3 AND plan_id=?4",
            params![key.workspace,key.destination,key.account,key.plan],
            |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?)),
        ).optional()?;
        if previous.as_ref().is_some_and(|(old_revision,old_action,old_hash)| {
            revision < *old_revision || (revision == *old_revision && (old_action != action.as_str() || old_hash != hash))
        }) {
            return Err(invalid());
        }
        transaction.execute(
            "INSERT INTO workout_delivery(workspace_id,destination,account_id,plan_id,generation,desired_hash,desired_action,state,updated_at,desired_revision)
             VALUES(?1,?2,?3,?4,1,?5,?6,'pending',?7,?8)
             ON CONFLICT(workspace_id,destination,account_id,plan_id) DO UPDATE SET
               generation=workout_delivery.generation+CASE WHEN workout_delivery.desired_hash<>excluded.desired_hash OR workout_delivery.desired_action<>excluded.desired_action THEN 1 ELSE 0 END,
               state=CASE WHEN workout_delivery.desired_hash<>excluded.desired_hash OR workout_delivery.desired_action<>excluded.desired_action THEN 'pending' ELSE workout_delivery.state END,
               retry_at=CASE WHEN workout_delivery.desired_hash<>excluded.desired_hash OR workout_delivery.desired_action<>excluded.desired_action THEN NULL ELSE workout_delivery.retry_at END,
               failure_reason=CASE WHEN workout_delivery.desired_hash<>excluded.desired_hash OR workout_delivery.desired_action<>excluded.desired_action THEN NULL ELSE workout_delivery.failure_reason END,
               desired_hash=excluded.desired_hash,desired_action=excluded.desired_action,
               desired_revision=excluded.desired_revision,updated_at=excluded.updated_at
             WHERE workout_delivery.desired_revision<excluded.desired_revision",
            params![key.workspace,key.destination,key.account,key.plan,hash,action.as_str(),now,revision],
        )?;
        let generation = transaction.query_row(
            "SELECT generation FROM workout_delivery WHERE workspace_id=?1 AND destination=?2 AND account_id=?3 AND plan_id=?4",
            params![key.workspace,key.destination,key.account,key.plan], |row| row.get(0),
        )?;
        transaction.commit()?;
        Ok(generation)
    }

    pub(crate) fn confirm_delivery(&self, key: &DeliveryKey<'_>, generation: i64, action: DeliveryAction, hash: &str, remote_id: Option<&str>, now: i64) -> Result<bool> {
        if !valid_key(key) || generation <= 0 || hash.trim().is_empty() || now < 0 || remote_id.is_some_and(|id| id.trim().is_empty()) {
            return Err(invalid());
        }
        let connection = self.conn.lock().unwrap();
        let updated = connection.execute(
            "UPDATE workout_delivery SET confirmed_generation=?5,state='confirmed',remote_id=COALESCE(?8,remote_id),updated_at=?9,retry_at=NULL,failure_reason=NULL
             WHERE workspace_id=?1 AND destination=?2 AND account_id=?3 AND plan_id=?4
                             AND generation=?5 AND desired_action=?6 AND desired_hash=?7 AND state<>'confirmed'
                             AND NOT EXISTS(SELECT 1 FROM workout_delivery_cleanup c
                                     WHERE c.workspace_id=?1 AND c.destination=?2 AND c.account_id=?3 AND c.plan_id=?4)",
            params![key.workspace,key.destination,key.account,key.plan,generation,action.as_str(),hash,remote_id,now],
        )?;
        Ok(updated == 1)
    }

    pub(crate) fn fail_delivery(&self, key: &DeliveryKey<'_>, generation: i64, failure: DeliveryFailure, retry_at: i64, now: i64) -> Result<bool> {
        if !valid_key(key) || generation <= 0 || now < 0 || retry_at < now {
            return Err(invalid());
        }
        let connection = self.conn.lock().unwrap();
        Ok(connection.execute(
            "UPDATE workout_delivery SET state='retry',updated_at=?6,retry_at=?7,failure_reason=?8
             WHERE workspace_id=?1 AND destination=?2 AND account_id=?3 AND plan_id=?4 AND generation=?5 AND state<>'confirmed'",
            params![key.workspace,key.destination,key.account,key.plan,generation,now,retry_at,failure.as_str()],
        )? == 1)
    }

    pub(crate) fn record_superseded_remote(&self, key: &DeliveryKey<'_>, generation: i64, remote_id: &str, now: i64) -> Result<bool> {
        if !valid_key(key) || generation <= 0 || remote_id.trim().is_empty() || now < 0 {
            return Err(invalid());
        }
        let mut connection = self.conn.lock().unwrap();
        let transaction = connection.transaction()?;
        let inserted = transaction.execute(
            "INSERT INTO workout_delivery_cleanup(workspace_id,destination,account_id,plan_id,remote_id,generation,created_at)
             SELECT workspace_id,destination,account_id,plan_id,?6,?5,?7 FROM workout_delivery
             WHERE workspace_id=?1 AND destination=?2 AND account_id=?3 AND plan_id=?4 AND generation>?5
             ON CONFLICT(workspace_id,destination,account_id,plan_id,remote_id) DO NOTHING",
            params![key.workspace,key.destination,key.account,key.plan,generation,remote_id,now],
        )? == 1;
        if inserted {
            transaction.execute(
                "UPDATE workout_delivery SET state='pending',updated_at=?5,retry_at=NULL,failure_reason=NULL
                 WHERE workspace_id=?1 AND destination=?2 AND account_id=?3 AND plan_id=?4",
                params![key.workspace,key.destination,key.account,key.plan,now],
            )?;
        }
        transaction.commit()?;
        Ok(inserted)
    }

    pub(crate) fn resolve_superseded_remote(&self, key: &DeliveryKey<'_>, remote_id: &str) -> Result<bool> {
        if !valid_key(key) || remote_id.trim().is_empty() {
            return Err(invalid());
        }
        Ok(self.conn.lock().unwrap().execute(
            "DELETE FROM workout_delivery_cleanup WHERE workspace_id=?1 AND destination=?2 AND account_id=?3 AND plan_id=?4 AND remote_id=?5",
            params![key.workspace,key.destination,key.account,key.plan,remote_id],
        )? == 1)
    }

    pub(crate) fn superseded_remotes(&self, key: &DeliveryKey<'_>) -> Result<Vec<String>> {
        if !valid_key(key) {
            return Err(invalid());
        }
        let connection = self.conn.lock().unwrap();
        let mut statement = connection.prepare(
            "SELECT remote_id FROM workout_delivery_cleanup WHERE workspace_id=?1 AND destination=?2 AND account_id=?3 AND plan_id=?4 ORDER BY created_at,remote_id",
        )?;
        let rows = statement.query_map(params![key.workspace,key.destination,key.account,key.plan], |row| row.get(0))?;
        Ok(rows.collect::<std::result::Result<Vec<_>,_>>()?)
    }

    pub(crate) fn delivery_state(&self, key: &DeliveryKey<'_>) -> Result<Option<DeliveryState>> {
        if !valid_key(key) {
            return Err(invalid());
        }
        Ok(self.conn.lock().unwrap().query_row(
            "SELECT generation,desired_hash,desired_action,state,confirmed_generation,remote_id,desired_revision,retry_at,failure_reason FROM workout_delivery
             WHERE workspace_id=?1 AND destination=?2 AND account_id=?3 AND plan_id=?4",
            params![key.workspace,key.destination,key.account,key.plan],
            |row| Ok(DeliveryState { generation:row.get(0)?,desired_hash:row.get(1)?,action:row.get(2)?,
                state:row.get(3)?,confirmed_generation:row.get(4)?,remote_id:row.get(5)?,desired_revision:row.get(6)?,
                retry_at:row.get(7)?,failure_reason:row.get(8)? }),
        ).optional()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> DeliveryKey<'static> {
        DeliveryKey { workspace: "workspace", destination: "coros", account: "runner", plan: "plan" }
    }

    #[test]
    fn due_work_respects_account_retry_deadlines_and_confirmation() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let pending = DeliveryKey { plan: "pending", ..key() };
        let retry = DeliveryKey { plan: "retry", ..key() };
        let confirmed = DeliveryKey { plan: "confirmed", ..key() };
        let other = DeliveryKey { account: "other", plan: "other", ..key() };
        store.enqueue_delivery(&pending,1,DeliveryAction::Push,"a",1).unwrap();
        let generation = store.enqueue_delivery(&retry,1,DeliveryAction::Cancel,"b",2).unwrap();
        assert!(store.fail_delivery(&retry,generation,DeliveryFailure::Offline,20,3).unwrap());
        let generation = store.enqueue_delivery(&confirmed,1,DeliveryAction::Push,"c",4).unwrap();
        assert!(store.confirm_delivery(&confirmed,generation,DeliveryAction::Push,"c",Some("remote"),5).unwrap());
        store.enqueue_delivery(&other,1,DeliveryAction::Push,"d",1).unwrap();
        assert_eq!(store.due_deliveries("workspace","coros","runner",19).unwrap(),vec!["pending"]);
        assert_eq!(store.due_deliveries("workspace","coros","runner",20).unwrap(),vec!["pending","retry"]);
        assert_eq!(store.due_deliveries("workspace","coros","other",20).unwrap(),vec!["other"]);
        assert!(store.due_deliveries("workspace","coros"," ",20).is_err());
    }

    #[test]
    fn stale_push_cannot_confirm_a_newer_revision_or_cancellation() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let key = key();
        let first = store.enqueue_delivery(&key,1,DeliveryAction::Push,"hash-a",1).unwrap();
        assert_eq!(store.enqueue_delivery(&key,1,DeliveryAction::Push,"hash-a",2).unwrap(),first);
        let second = store.enqueue_delivery(&key,2,DeliveryAction::Push,"hash-b",3).unwrap();
        assert!(second > first);
        assert!(!store.confirm_delivery(&key,first,DeliveryAction::Push,"hash-a",Some("remote"),4).unwrap());
        assert!(!store.fail_delivery(&key,first,DeliveryFailure::Offline,10,4).unwrap());
        assert!(store.confirm_delivery(&key,second,DeliveryAction::Push,"hash-b",Some("remote"),5).unwrap());
        let cancel = store.enqueue_delivery(&key,3,DeliveryAction::Cancel,"cancel",6).unwrap();
        assert!(!store.confirm_delivery(&key,second,DeliveryAction::Push,"hash-b",Some("remote"),7).unwrap());
        assert_eq!(store.delivery_state(&key).unwrap(),Some(DeliveryState {
            generation:cancel,desired_revision:3,desired_hash:"cancel".into(),action:"cancel".into(),state:"pending".into(),
            confirmed_generation:Some(second),remote_id:Some("remote".into()),retry_at:None,failure_reason:None,
        }));
        assert!(store.confirm_delivery(&key,cancel,DeliveryAction::Cancel,"cancel",None,8).unwrap());
    }

    #[test]
    fn retry_and_account_isolation_preserve_pending_state() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let other = DeliveryKey { account: "other", ..key() };
        let key = key();
        let generation = store.enqueue_delivery(&key,1,DeliveryAction::Push,"hash-a",1).unwrap();
        assert!(store.fail_delivery(&key,generation,DeliveryFailure::Offline,30,2).unwrap());
        assert_eq!(store.enqueue_delivery(&key,2,DeliveryAction::Push,"hash-a",3).unwrap(),generation);
        assert_eq!(store.delivery_state(&key).unwrap().unwrap().state,"retry");
        assert!(store.delivery_state(&other).unwrap().is_none());
        assert!(!store.confirm_delivery(&key,generation,DeliveryAction::Push,"other-hash",None,4).unwrap());
        assert!(store.confirm_delivery(&key,generation,DeliveryAction::Push,"hash-a",Some("remote"),4).unwrap());
        assert!(!store.confirm_delivery(&key,generation,DeliveryAction::Push,"hash-a",Some("different"),5).unwrap());
        assert_eq!(store.delivery_state(&key).unwrap().unwrap().remote_id.as_deref(),Some("remote"));
        assert!(!store.fail_delivery(&key,generation,DeliveryFailure::Offline,30,5).unwrap());
    }

    #[test]
    fn pending_cancellation_survives_database_reopen() {
        let path = std::env::temp_dir().join(format!("runalytics-delivery-{}.sqlite",uuid::Uuid::new_v4()));
        let key = key();
        let generation = {
            let store = Store::open(&path).unwrap();
            let push = store.enqueue_delivery(&key,1,DeliveryAction::Push,"hash",1).unwrap();
            assert!(store.confirm_delivery(&key,push,DeliveryAction::Push,"hash",Some("remote"),2).unwrap());
            store.enqueue_delivery(&key,2,DeliveryAction::Cancel,"cancel-hash",3).unwrap()
        };
        {
            let store = Store::open(&path).unwrap();
            let state = store.delivery_state(&key).unwrap().unwrap();
            assert_eq!((state.generation,state.desired_hash.as_str(),state.action.as_str(),state.state.as_str(),state.remote_id.as_deref()),
                (generation,"cancel-hash","cancel","pending",Some("remote")));
            assert!(store.confirm_delivery(&key,generation,DeliveryAction::Cancel,"cancel-hash",None,4).unwrap());
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn old_or_conflicting_snapshots_cannot_supersede_newer_delivery() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let key = key();
        let first = store.enqueue_delivery(&key,10,DeliveryAction::Push,"hash",1).unwrap();
        assert!(store.confirm_delivery(&key,first,DeliveryAction::Push,"hash",Some("remote"),2).unwrap());
        assert_eq!(store.enqueue_delivery(&key,11,DeliveryAction::Push,"hash",3).unwrap(),first);
        assert_eq!(store.delivery_state(&key).unwrap().unwrap().state,"confirmed");
        assert!(store.enqueue_delivery(&key,10,DeliveryAction::Push,"old",4).is_err());
        assert!(store.enqueue_delivery(&key,11,DeliveryAction::Cancel,"cancel",4).is_err());
        let cancel = store.enqueue_delivery(&key,12,DeliveryAction::Cancel,"cancel",5).unwrap();
        assert!(store.enqueue_delivery(&key,11,DeliveryAction::Push,"hash",6).is_err());
        let state = store.delivery_state(&key).unwrap().unwrap();
        assert_eq!((state.generation,state.desired_revision,state.action.as_str(),state.state.as_str()),(cancel,12,"cancel","pending"));
    }

    #[test]
    fn old_ledger_schema_is_repaired_without_losing_pending_work() {
        let path = std::env::temp_dir().join(format!("runalytics-delivery-migrate-{}.sqlite",uuid::Uuid::new_v4()));
        {
            let connection = rusqlite::Connection::open(&path).unwrap();
            connection.execute_batch(
                "CREATE TABLE workout_delivery (
                    workspace_id TEXT NOT NULL,destination TEXT NOT NULL,account_id TEXT NOT NULL,plan_id TEXT NOT NULL,
                    generation INTEGER NOT NULL,desired_hash TEXT NOT NULL,desired_action TEXT NOT NULL,
                    confirmed_generation INTEGER,remote_id TEXT,state TEXT NOT NULL,updated_at INTEGER NOT NULL,
                    PRIMARY KEY(workspace_id,destination,account_id,plan_id)
                );
                INSERT INTO workout_delivery VALUES('workspace','coros','runner','plan',2,'cancel','cancel',1,'remote','pending',3);"
            ).unwrap();
        }
        {
            let store = Store::open(&path).unwrap();
            let state = store.delivery_state(&key()).unwrap().unwrap();
            assert_eq!((state.generation,state.desired_revision,state.state.as_str(),state.remote_id.as_deref()),(2,0,"pending",Some("remote")));
            assert_eq!(store.enqueue_delivery(&key(),1,DeliveryAction::Cancel,"cancel",4).unwrap(),2);
            assert!(store.enqueue_delivery(&key(),0,DeliveryAction::Push,"old",5).is_err());
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn late_remote_identity_survives_cancellation_and_reopen_without_acknowledging_it() {
        let path = std::env::temp_dir().join(format!("runalytics-delivery-cleanup-{}.sqlite",uuid::Uuid::new_v4()));
        let other = DeliveryKey { account:"other", ..key() };
        let key = key();
        {
            let store = Store::open(&path).unwrap();
            let push = store.enqueue_delivery(&key,1,DeliveryAction::Push,"push",1).unwrap();
            let cancel = store.enqueue_delivery(&key,2,DeliveryAction::Cancel,"cancel",2).unwrap();
            assert!(!store.record_superseded_remote(&key,cancel,"remote",3).unwrap());
            assert!(!store.record_superseded_remote(&other,push,"remote",3).unwrap());
            assert!(store.record_superseded_remote(&key,push,"remote",3).unwrap());
            assert!(!store.record_superseded_remote(&key,push,"remote",4).unwrap());
            assert!(!store.confirm_delivery(&key,push,DeliveryAction::Push,"push",Some("remote"),4).unwrap());
            assert_eq!(store.delivery_state(&key).unwrap().unwrap().state,"pending");
        }
        {
            let store = Store::open(&path).unwrap();
            assert_eq!(store.superseded_remotes(&key).unwrap(),vec!["remote".to_string()]);
            assert!(store.superseded_remotes(&other).unwrap().is_empty());
            assert_eq!(store.delivery_state(&key).unwrap().unwrap().action,"cancel");
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn late_push_invalidates_confirmation_until_each_remote_is_resolved() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let other = DeliveryKey { account:"other", ..key() };
        let key = key();
        let push = store.enqueue_delivery(&key,1,DeliveryAction::Push,"push",1).unwrap();
        let cancel = store.enqueue_delivery(&key,2,DeliveryAction::Cancel,"cancel",2).unwrap();
        assert!(store.confirm_delivery(&key,cancel,DeliveryAction::Cancel,"cancel",None,3).unwrap());
        assert!(store.record_superseded_remote(&key,push,"remote-a",4).unwrap());
        assert!(store.record_superseded_remote(&key,push,"remote-b",5).unwrap());
        assert_eq!(store.delivery_state(&key).unwrap().unwrap().state,"pending");
        assert!(!store.confirm_delivery(&key,cancel,DeliveryAction::Cancel,"cancel",None,6).unwrap());
        assert!(!store.resolve_superseded_remote(&other,"remote-a").unwrap());
        assert!(store.resolve_superseded_remote(&key,"remote-a").unwrap());
        assert!(!store.confirm_delivery(&key,cancel,DeliveryAction::Cancel,"cancel",None,7).unwrap());
        assert!(store.resolve_superseded_remote(&key,"remote-b").unwrap());
        assert!(store.confirm_delivery(&key,cancel,DeliveryAction::Cancel,"cancel",None,8).unwrap());
    }

    #[test]
    fn retry_metadata_survives_restart_and_is_cleared_by_a_new_revision() {
        let path = std::env::temp_dir().join(format!("runalytics-delivery-retry-{}.sqlite",uuid::Uuid::new_v4()));
        let key = key();
        {
            let store = Store::open(&path).unwrap();
            let generation = store.enqueue_delivery(&key,1,DeliveryAction::Push,"first",1).unwrap();
            assert!(store.fail_delivery(&key,generation,DeliveryFailure::Authentication,60,2).unwrap());
            assert!(store.fail_delivery(&key,generation,DeliveryFailure::Provider,1,2).is_err());
        }
        {
            let store = Store::open(&path).unwrap();
            let state = store.delivery_state(&key).unwrap().unwrap();
            assert_eq!((state.state.as_str(),state.retry_at,state.failure_reason.as_deref()),("retry",Some(60),Some("authentication")));
            let next = store.enqueue_delivery(&key,2,DeliveryAction::Push,"second",3).unwrap();
            let state = store.delivery_state(&key).unwrap().unwrap();
            assert_eq!((state.state.as_str(),state.retry_at,state.failure_reason.as_deref()),("pending",None,None));
            assert!(store.fail_delivery(&key,next,DeliveryFailure::Verification,70,4).unwrap());
            assert!(store.confirm_delivery(&key,next,DeliveryAction::Push,"second",Some("remote"),5).unwrap());
            assert_eq!(store.delivery_state(&key).unwrap().unwrap().retry_at,None);
        }
        std::fs::remove_file(path).unwrap();
    }
}