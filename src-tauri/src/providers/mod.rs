pub mod coros;
#[cfg(feature = "garmin")]
pub mod garmin;

use std::sync::Arc;

use async_trait::async_trait;

use crate::error::Result;
use crate::store::Store;
use crate::types::*;

/// Reports how far a long-running provider operation has come: (stage label,
/// 0..=100). Commands pass a closure that emits `plan://sync`; background
/// callers (sync re-push) pass `&None` to stay silent.
pub type Progress<'a> = Option<&'a (dyn Fn(&str, u8) + Send + Sync)>;

/// A wearable/cloud data provider. COROS is implemented over the official
/// MCP endpoint; Garmin is feature-gated (no official public API).
#[async_trait]
pub trait Provider: Send + Sync {
    fn kind(&self) -> ProviderKind;

    /// Whether usable credentials are available right now.
    fn is_connected(&self) -> bool;

    /// Interactive OAuth connect. Returns a human-readable status message.
    async fn connect(&self) -> Result<String>;

    /// Drop stored credentials.
    async fn disconnect(&self) -> Result<()>;

    /// Pull activities for `days_back` into the store.
    async fn sync_activities(&self, store: &Store, days_back: i64) -> Result<u32>;

    /// Pull daily health + sleep + HRV for `days_back` into the store.
    async fn sync_health(&self, store: &Store, days_back: i64) -> Result<u32>;

    /// Pull fitness assessment + training load into the store.
    async fn sync_fitness(&self, store: &Store) -> Result<()>;

    /// Download FIT files for recent activities lacking tracks and decode
    /// them into track points. `budget` caps downloads (COROS allows 50/24h).
    async fn sync_tracks(&self, store: &Store, budget: u32) -> Result<u32>;

    /// Push a generated plan's workouts onto the provider schedule.
    /// Returns the provider-side plan/workout identifier when available.
    /// When the plan already carries a pushed id the provider must update
    /// its existing schedule in place, never create a second one.
    /// `progress` receives (stage, 0..=100) as the call walks its steps.
    async fn push_plan(
        &self,
        plan: &TrainingPlan,
        workouts: &[PlanWorkout],
        progress: Progress<'_>,
    ) -> Result<Option<String>>;

    /// Remove a previously pushed plan from the provider schedule — the
    /// counterpart of `push_plan`, called when the runner aborts the plan.
    /// `pushed_id` is the id `push_plan` returned; when absent the provider
    /// locates its own in-progress plan. Idempotent: an already-gone plan
    /// is not an error.
    async fn cancel_plan(&self, pushed_id: Option<&str>, progress: Progress<'_>) -> Result<()>;

    /// Diagnostic: list the tool names the provider's API actually exposes.
    /// Used by the UI to verify endpoint existence and tool availability.
    async fn list_api_tools(&self) -> Result<Vec<String>>;
}

/// Build the set of providers compiled into this build.
pub fn build_providers() -> Vec<Arc<dyn Provider>> {
    #[allow(unused_mut)]
    let mut v: Vec<Arc<dyn Provider>> = vec![Arc::new(coros::CorosProvider::new())];
    #[cfg(feature = "garmin")]
    v.push(Arc::new(garmin::GarminProvider::new()));
    v
}
