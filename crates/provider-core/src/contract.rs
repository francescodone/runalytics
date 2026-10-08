//! The adapter contract and the draft shapes adapters return.
//!
//! A *draft* is a provider record stripped of everything the local database
//! owns: ids, account keys, fetch timestamps, matched-session links. Adapters
//! that cannot set those fields cannot get them wrong, and a fake adapter in a
//! test can be a table of drafts with no database at all.

use async_trait::async_trait;
use runalytics_core::{
    ActivityLap, ActivitySummary, Date, FitnessAssessment, PlannedSession, SleepDay, Timestamp,
};

use crate::capability::Capability;
use crate::error::Result;

/// What the provider reported about the account behind a successful connect.
#[derive(Debug, Clone, PartialEq)]
pub struct AccountInfo {
    /// The provider's own user/account id, verbatim. Stored as
    /// `external_user_id` so a reconnect under a new local row still recognises
    /// the same athlete.
    pub external_user_id: Option<String>,
    /// A human-stable label for the account (usually the provider username or
    /// email) — the dedup key in `provider_account`.
    pub account_label: String,
    /// Which regional deployment served the connect (`cn`, `eu`, `us`).
    pub region: Option<String>,
    /// What this account can actually do, negotiated at connect time. A
    /// provider can serve less than its adapter advertises — a COROS account
    /// in a region without the health tools, for instance.
    pub capabilities: Capability,
}

/// An activity as the provider reported it, without local identity.
#[derive(Debug, Clone, PartialEq)]
pub struct ActivityDraft {
    /// The provider's activity id, verbatim. Dedup and re-fetch key on it.
    pub provider_activity_id: String,
    pub name: String,
    pub started_at: Timestamp,
    /// The local calendar day, already resolved in the *athlete's* time zone —
    /// the adapter owns that conversion because only it knows where the watch
    /// recorded the start.
    pub local_date: Date,
    pub summary: ActivitySummary,
    pub laps: Vec<ActivityLap>,
    /// The provider's own intensity label, when it publishes one. The ingest
    /// path keeps it only as a fallback; `runalytics-scoring` re-derives the
    /// real intensity from data.
    pub provider_intensity_label: Option<String>,
}

/// A day of health data as the provider reported it.
///
/// Mirrors `HealthDay` minus the fields the database owns (id, account,
/// fetch time) — an adapter that cannot set an id cannot set the wrong one.
/// The ingest path fills the rest in.
#[derive(Debug, Clone, PartialEq)]
pub struct HealthDraft {
    /// The recovery day this describes: for a night ending Monday morning,
    /// Monday. Adapters that receive provider nights keyed by *onset* date
    /// convert before returning, because the whole app treats a sleep record
    /// as belonging to the morning it ended.
    pub date: Date,
    pub resting_hr: Option<runalytics_core::HeartRate>,
    pub avg_stress: Option<f64>,
    pub high_stress_minutes: Option<u32>,
    pub sleep: Option<SleepDay>,
    pub steps: Option<u32>,
    /// The provider's own readiness-style score, when it publishes one.
    pub provider_readiness: Option<u8>,
    pub basal_energy: Option<f64>,
}

/// A provider fitness estimate, as reported.
#[derive(Debug, Clone, PartialEq)]
pub struct FitnessDraft {
    pub assessment: FitnessAssessment,
}

/// A planned session on its way to a watch.
///
/// Adapters receive the full [`PlannedSession`] — including the structured
/// blocks — and translate it into whatever their platform calls a workout.
/// `update_key` is the session's `external_id` when one exists, so a re-plan
/// updates the existing workout instead of creating a duplicate.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkoutPush<'a> {
    pub session: &'a PlannedSession,
    pub update_key: Option<&'a str>,
}

/// The outcome of pushing a week of sessions to a provider.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PushReport {
    /// Sessions now present on the provider, with their new/updated external
    /// ids. The caller writes these back into `planned_session.external_id`.
    pub pushed: Vec<(runalytics_core::PlannedSessionId, String)>,
    /// Sessions the provider refused, with the reason to show the user.
    pub failed: Vec<(runalytics_core::PlannedSessionId, String)>,
}

impl PushReport {
    #[must_use]
    pub fn all_succeeded(&self) -> bool {
        self.failed.is_empty()
    }
}

/// Everything a wearable adapter must be able to do.
///
/// Object-safe by construction (the app holds `Box<dyn WearableProvider>` per
/// connected account). Every read method takes `&self`: an adapter is
/// immutable after [`WearableProvider::connect`] and reconnects by being
/// replaced, which removes the whole class of "half re-authenticated" states.
#[async_trait]
pub trait WearableProvider: Send + Sync {
    /// Which platform this is.
    fn provider(&self) -> runalytics_core::Provider;

    /// What the *adapter* can do at best. The per-account truth arrives with
    /// [`Self::connect`].
    fn capabilities(&self) -> Capability;

    /// Authenticate (or resume from stored tokens) and negotiate capabilities.
    ///
    /// Returns the account this adapter is now bound to. Implementations must
    /// be idempotent: calling `connect` again with valid stored tokens must
    /// not re-prompt the user.
    async fn connect(&mut self) -> Result<AccountInfo>;

    /// The account bound by `connect`. Panicking-free: `None` before connect.
    fn account(&self) -> Option<&AccountInfo>;

    /// Activities started in `[from, to]`, inclusive, oldest first.
    ///
    /// Adapters page internally; the caller sees one call per window.
    async fn fetch_activities(&self, from: Date, to: Date) -> Result<Vec<ActivityDraft>>;

    /// Health days in `[from, to]`, inclusive. Days with no data at all are
    /// *absent*, not zero-filled — a missing day must stay missing or the
    /// readiness model starts trusting silence.
    async fn fetch_health(&self, from: Date, to: Date) -> Result<Vec<HealthDraft>>;

    /// Fitness estimates in `[from, to]`, inclusive.
    async fn fetch_fitness(&self, from: Date, to: Date) -> Result<Vec<FitnessDraft>>;

    /// Push one session to the provider. Returns the provider-side id to
    /// store as `external_id`.
    ///
    /// Only called when [`Capability::PLANS_WRITE`] is set; implementations
    /// for read-only platforms return [`crate::ProviderError::Unsupported`].
    async fn push_workout(&self, push: &WorkoutPush<'_>) -> Result<String>;

    /// Remove a previously pushed workout by its external id.
    async fn delete_workout(&self, external_id: &str) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_report_defaults_to_empty_and_succeeded() {
        let report = PushReport::default();
        assert!(report.all_succeeded());
        // assert_eq (not assert! + is_empty) so a failure prints the entries.
        assert_eq!(report.pushed, []);
    }

    /// The trait must stay object-safe; this compiles only if it is.
    #[test]
    fn trait_is_object_safe() {
        fn _takes_boxed(_: Box<dyn WearableProvider>) {}
    }
}
