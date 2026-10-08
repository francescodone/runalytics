//! The resumable ingestion pipeline shared by every adapter.
//!
//! This is the only place in the workspace that is allowed to combine a
//! provider response with the database, and the rules it enforces are the
//! reason syncing can be interrupted at any moment:
//!
//! * The cursor advances only to the last day a fetch *completed* covering.
//!   A failure mid-window leaves the cursor where it was, so the next sync
//!   re-fetches and upserts over itself.
//! * Every window starts `overlap_days` before the cursor. A night's sleep is
//!   finalised hours after midnight and a provider may backfill an activity's
//!   lap data a day later; resuming at exactly the cursor would permanently
//!   miss both.
//! * Every run is logged to `sync_run` — start, status, record count, error —
//!   which is what the Settings sync log shows and what makes a user's "sync
//!   is broken" report diagnosable.

use runalytics_core::{Date, Provider, Timestamp};
use runalytics_store::Db;
use runalytics_store::repos::{
    ActivityRepo, FitnessRepo, HealthRepo, ProviderAccountRepo, SyncRunRepo,
};

use crate::contract::{ActivityDraft, HealthDraft, WearableProvider};
use crate::error::{ProviderError, Result};
use runalytics_core::{ACTIVITY_NAMESPACE, Activity, ActivityId, HealthDay, HealthDayId};

/// How the ingest path windows a sync.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SyncPolicy {
    /// Days re-fetched behind the cursor, to catch late-arriving data.
    pub overlap_days: i64,
    /// Days fetched when no cursor exists yet — the initial history pull.
    pub initial_backfill_days: i64,
    /// The largest window one fetch call may cover. Providers page internally
    /// and a month of COROS health data in one response is a timeout waiting
    /// to happen; the pipeline loops windows of this size.
    pub max_window_days: i64,
}

impl Default for SyncPolicy {
    fn default() -> Self {
        Self {
            // 3 days covers the observed late-arrival window for sleep and
            // lap backfill on both target platforms, without re-fetching
            // enough to matter against rate limits.
            overlap_days: 3,
            // A year of history is what an athlete expects to see on day one,
            // and what the injury model needs for its history factor.
            initial_backfill_days: 365,
            max_window_days: 14,
        }
    }
}

/// What one ingest call did.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SyncReport {
    pub data_class: String,
    /// Drafts upserted (activities and health days, not store-level row counts).
    pub records: u32,
    /// The cursor after this sync, when one was written.
    pub cursor_after: Option<Date>,
}

/// The inclusive `[from, to]` window a sync should fetch, given the cursor.
///
/// `to` is *today*: a provider cannot report a future activity, and capping at
/// today keeps the cursor monotone even if the athlete's clock is wrong.
fn window_for(cursor: Option<Date>, today: Date, policy: &SyncPolicy) -> (Date, Date) {
    let from = match cursor {
        Some(c) => c - chrono::Duration::days(policy.overlap_days),
        None => today - chrono::Duration::days(policy.initial_backfill_days),
    };
    (from, today)
}

/// Split `[from, to]` into consecutive windows of at most `max_window_days`.
fn windows(from: Date, to: Date, max_window_days: i64) -> Vec<(Date, Date)> {
    let mut out = Vec::new();
    let mut start = from;
    while start <= to {
        let end = (start + chrono::Duration::days(max_window_days - 1)).min(to);
        out.push((start, end));
        start = end + chrono::Duration::days(1);
    }
    out
}

/// Sync activities for one account. See the module docs for the contract.
///
/// # Errors
///
/// [`ProviderError`] from any fetch window. The cursor is left untouched when
/// any window fails, so the next sync resumes the whole range.
pub async fn ingest_activities(
    db: &Db,
    account_id: runalytics_core::ProviderAccountId,
    provider: &dyn WearableProvider,
    today: Date,
    fetched_at: Timestamp,
    policy: &SyncPolicy,
) -> Result<SyncReport> {
    let run = SyncRunRepo::start(db, account_id, "activities").map_err(store_err)?;
    let outcome = pull_activities(db, account_id, provider, today, fetched_at, policy).await;
    finish_run(db, run, &outcome, |r| r.records);
    outcome
}

async fn pull_activities(
    db: &Db,
    account_id: runalytics_core::ProviderAccountId,
    provider: &dyn WearableProvider,
    today: Date,
    fetched_at: Timestamp,
    policy: &SyncPolicy,
) -> Result<SyncReport> {
    let cursor = ProviderAccountRepo::cursor(db, account_id, "activities").map_err(store_err)?;
    let (from, to) = window_for(cursor, today, policy);

    let mut records = 0_u32;
    let mut last_complete: Option<Date> = None;
    for (wfrom, wto) in windows(from, to, policy.max_window_days) {
        let drafts = provider.fetch_activities(wfrom, wto).await?;
        for draft in &drafts {
            let activity = materialise_activity(draft, account_id, provider.provider(), fetched_at);
            ActivityRepo::upsert(db, &activity).map_err(store_err)?;
            records += 1;
        }
        last_complete = Some(wto);
    }

    if let Some(date) = last_complete {
        ProviderAccountRepo::set_cursor(db, account_id, "activities", advance_cursor(date, today))
            .map_err(store_err)?;
    }
    Ok(SyncReport {
        data_class: "activities".into(),
        records,
        cursor_after: last_complete.map(|d| advance_cursor(d, today)),
    })
}

/// Sync daily health for one account.
pub async fn ingest_health(
    db: &Db,
    account_id: runalytics_core::ProviderAccountId,
    provider: &dyn WearableProvider,
    today: Date,
    policy: &SyncPolicy,
) -> Result<SyncReport> {
    let run = SyncRunRepo::start(db, account_id, "health").map_err(store_err)?;
    let outcome = pull_health(db, account_id, provider, today, policy).await;
    finish_run(db, run, &outcome, |r| r.records);
    outcome
}

async fn pull_health(
    db: &Db,
    account_id: runalytics_core::ProviderAccountId,
    provider: &dyn WearableProvider,
    today: Date,
    policy: &SyncPolicy,
) -> Result<SyncReport> {
    let cursor = ProviderAccountRepo::cursor(db, account_id, "health").map_err(store_err)?;
    let (from, to) = window_for(cursor, today, policy);

    let mut records = 0_u32;
    let mut last_complete: Option<Date> = None;
    for (wfrom, wto) in windows(from, to, policy.max_window_days) {
        let drafts = provider.fetch_health(wfrom, wto).await?;
        for draft in &drafts {
            let day = materialise_health(draft, account_id);
            HealthRepo::upsert(db, &day).map_err(store_err)?;
            records += 1;
        }
        last_complete = Some(wto);
    }

    if let Some(date) = last_complete {
        ProviderAccountRepo::set_cursor(db, account_id, "health", advance_cursor(date, today))
            .map_err(store_err)?;
    }
    Ok(SyncReport {
        data_class: "health".into(),
        records,
        cursor_after: last_complete.map(|d| advance_cursor(d, today)),
    })
}

/// Sync provider fitness estimates for one account.
pub async fn ingest_fitness(
    db: &Db,
    account_id: runalytics_core::ProviderAccountId,
    provider: &dyn WearableProvider,
    today: Date,
    policy: &SyncPolicy,
) -> Result<SyncReport> {
    let run = SyncRunRepo::start(db, account_id, "fitness").map_err(store_err)?;
    let cursor = ProviderAccountRepo::cursor(db, account_id, "fitness").map_err(store_err)?;
    let (from, to) = window_for(cursor, today, policy);

    let mut records = 0_u32;
    let mut last_complete: Option<Date> = None;
    for (wfrom, wto) in windows(from, to, policy.max_window_days) {
        let drafts = provider.fetch_fitness(wfrom, wto).await?;
        for draft in &drafts {
            FitnessRepo::upsert(db, account_id, &draft.assessment).map_err(store_err)?;
            records += 1;
        }
        last_complete = Some(wto);
    }

    let outcome = (|| -> Result<SyncReport> {
        if let Some(date) = last_complete {
            ProviderAccountRepo::set_cursor(db, account_id, "fitness", advance_cursor(date, today))
                .map_err(store_err)?;
        }
        Ok(SyncReport {
            data_class: "fitness".into(),
            records,
            cursor_after: last_complete.map(|d| advance_cursor(d, today)),
        })
    })();
    finish_run(db, run, &outcome, |r| r.records);
    outcome
}

/// The cursor position after a completed window.
///
/// A window ending today must not park the cursor on today: today's data is
/// still being written (the athlete may run this evening, sleep is not over).
/// The cursor holds one day back so tomorrow's sync re-covers today.
fn advance_cursor(completed: Date, today: Date) -> Date {
    if completed >= today {
        today - chrono::Duration::days(1)
    } else {
        completed
    }
}

fn materialise_activity(
    draft: &ActivityDraft,
    account_id: runalytics_core::ProviderAccountId,
    provider: Provider,
    fetched_at: Timestamp,
) -> Activity {
    let key = format!("{}:{}", provider.as_str(), draft.provider_activity_id);
    Activity {
        id: ActivityId::from_external(&ACTIVITY_NAMESPACE, &key),
        account: account_id,
        provider_activity_id: draft.provider_activity_id.clone(),
        name: draft.name.clone(),
        started_at: draft.started_at,
        local_date: draft.local_date,
        summary: draft.summary.clone(),
        laps: draft.laps.clone(),
        // The derived intensity is scoring's job; until it runs, the provider's
        // label is the best available and `intensity_factor` may fall back to it.
        intensity: draft
            .provider_intensity_label
            .as_deref()
            .and_then(|s| s.parse().ok())
            .unwrap_or_default(),
        matched_session: None,
        fetched_at,
    }
}

fn materialise_health(
    draft: &HealthDraft,
    account_id: runalytics_core::ProviderAccountId,
) -> HealthDay {
    HealthDay {
        id: HealthDayId::new(),
        account: account_id,
        date: draft.date,
        resting_hr: draft.resting_hr,
        avg_stress: draft.avg_stress,
        high_stress_minutes: draft.high_stress_minutes,
        sleep: draft.sleep.clone(),
        steps: draft.steps,
        provider_readiness: draft.provider_readiness,
        basal_energy: draft.basal_energy,
    }
}

/// Map a store error into the provider error domain.
///
/// The store has its own error type; callers of ingest deal with one error
/// type, and a database failure is *our* failure, not the provider's — so it
/// arrives as a non-retryable transport-shaped error carrying the cause.
fn store_err(e: runalytics_store::StoreError) -> ProviderError {
    ProviderError::Transport {
        provider: "store".into(),
        source: anyhow::Error::new(e).context("local database"),
    }
}

fn finish_run<T>(
    db: &Db,
    run_id: runalytics_core::Uuid,
    outcome: &Result<T>,
    records: impl Fn(&T) -> u32,
) {
    let (status, count, error) = match outcome {
        Ok(report) => ("ok", i64::from(records(report)), None),
        Err(e) => ("error", 0, Some(e.to_string())),
    };
    // The sync log must never mask the real outcome: if writing the log fails,
    // that is a second, louder problem but the first one is the return value.
    if let Err(log_err) = SyncRunRepo::finish(db, run_id, status, count, error.as_deref()) {
        tracing::warn!(%log_err, "failed to record sync run outcome");
    }
}

/// Record a successful connect against the store, returning the account id.
///
/// The provider enum comes from the adapter, not the info: `AccountInfo`
/// describes what the *remote side* reported about the athlete, and which
/// platform we are talking to is local knowledge.
pub fn register_account(
    db: &Db,
    provider: Provider,
    info: &crate::contract::AccountInfo,
) -> Result<runalytics_core::ProviderAccountId> {
    ProviderAccountRepo::upsert(db, provider, &info.account_label, info.region.as_deref())
        .map_err(store_err)
}

/// Sync every data class the provider's capabilities promise, health first.
///
/// One failure does not stop the remaining classes: a COROS health outage must
/// not prevent today's run from appearing on the dashboard. The returned
/// reports are per class; the caller decides how loudly to surface failures.
pub async fn ingest_all(
    db: &Db,
    account_id: runalytics_core::ProviderAccountId,
    provider: &dyn WearableProvider,
    today: Date,
    fetched_at: Timestamp,
    policy: &SyncPolicy,
) -> Vec<std::result::Result<SyncReport, ProviderError>> {
    let mut out = Vec::new();
    for class in provider.capabilities().data_classes() {
        let result = match class {
            "health" => ingest_health(db, account_id, provider, today, policy).await,
            "activities" => {
                ingest_activities(db, account_id, provider, today, fetched_at, policy).await
            }
            "fitness" => ingest_fitness(db, account_id, provider, today, policy).await,
            other => Err(ProviderError::Unsupported {
                provider: provider.provider().display_name().into(),
                capability: other.into(),
            }),
        };
        out.push(result);
    }
    out
}
