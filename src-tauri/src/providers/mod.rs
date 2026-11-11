pub mod coros;
pub(crate) mod delivery;
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
pub trait ProviderConnection: Send + Sync {
    fn kind(&self) -> ProviderKind;

    /// Whether usable credentials are available right now.
    fn is_connected(&self) -> bool;

    /// Interactive OAuth connect. Returns a human-readable status message.
    async fn connect(&self) -> Result<String>;

    /// Drop stored credentials.
    async fn disconnect(&self) -> Result<()>;
}

#[async_trait]
pub trait ActivitySource: ProviderConnection {
    /// Pull activities for `days_back` into the store.
    async fn sync_activities(&self, store: &Store, days_back: i64) -> Result<u32>;

    /// Download FIT files for recent activities lacking tracks and decode
    /// them into track points. `budget` caps downloads (COROS allows 50/24h).
    async fn sync_tracks(&self, store: &Store, budget: u32) -> Result<u32>;
}

#[async_trait]
pub trait WorkoutDestination: ProviderConnection {
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

#[async_trait]
pub trait Provider: ActivitySource + WorkoutDestination {
    async fn sync_health(&self, store: &Store, days_back: i64) -> Result<u32>;
    async fn sync_fitness(&self, store: &Store) -> Result<()>;
}

#[derive(Default)]
pub struct ProviderRegistry {
    legacy: Vec<Arc<dyn Provider>>,
    sources: Vec<Arc<dyn ActivitySource>>,
    destinations: Vec<Arc<dyn WorkoutDestination>>,
}

impl ProviderRegistry {
    fn register_legacy(&mut self, provider: Arc<dyn Provider>) {
        self.register_source(provider.clone());
        self.register_destination(provider.clone());
        self.legacy.push(provider);
    }

    pub fn register_source(&mut self, source: Arc<dyn ActivitySource>) {
        self.sources
            .retain(|existing| existing.kind() != source.kind());
        self.sources.push(source);
    }

    pub fn register_destination(&mut self, destination: Arc<dyn WorkoutDestination>) {
        self.destinations
            .retain(|existing| existing.kind() != destination.kind());
        self.destinations.push(destination);
    }

    pub fn legacy(&self, kind: ProviderKind) -> Option<Arc<dyn Provider>> {
        self.legacy
            .iter()
            .find(|provider| provider.kind() == kind)
            .cloned()
    }

    pub fn source(&self, kind: ProviderKind) -> Option<Arc<dyn ActivitySource>> {
        self.sources
            .iter()
            .find(|source| source.kind() == kind)
            .cloned()
    }

    pub fn destination(&self, kind: ProviderKind) -> Option<Arc<dyn WorkoutDestination>> {
        self.destinations
            .iter()
            .find(|destination| destination.kind() == kind)
            .cloned()
    }

    pub fn destinations(&self) -> impl Iterator<Item = &Arc<dyn WorkoutDestination>> {
        self.destinations.iter()
    }
}

/// Build the set of providers compiled into this build.
pub fn build_providers() -> ProviderRegistry {
    let mut registry = ProviderRegistry::default();
    registry.register_legacy(Arc::new(coros::CorosProvider::new()));
    #[cfg(feature = "garmin")]
    registry.register_legacy(Arc::new(garmin::GarminProvider::new()));
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct CalendarOnlyDestination {
        calls: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl ProviderConnection for CalendarOnlyDestination {
        fn kind(&self) -> ProviderKind {
            ProviderKind::Coros
        }

        fn is_connected(&self) -> bool {
            true
        }

        async fn connect(&self) -> Result<String> {
            self.calls.lock().unwrap().push("connect".into());
            Ok("connected".into())
        }

        async fn disconnect(&self) -> Result<()> {
            self.calls.lock().unwrap().push("disconnect".into());
            Ok(())
        }
    }

    #[async_trait]
    impl WorkoutDestination for CalendarOnlyDestination {
        async fn push_plan(
            &self,
            plan: &TrainingPlan,
            workouts: &[PlanWorkout],
            progress: Progress<'_>,
        ) -> Result<Option<String>> {
            assert!(workouts.iter().all(|workout| workout.plan_id == plan.id));
            self.calls.lock().unwrap().push(format!("push:{}", plan.id));
            if let Some(report) = progress {
                report("calendar accepted", 100);
            }
            Ok(Some("remote-plan".into()))
        }

        async fn cancel_plan(&self, pushed_id: Option<&str>, _: Progress<'_>) -> Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("cancel:{}", pushed_id.unwrap_or("discover")));
            Ok(())
        }

        async fn list_api_tools(&self) -> Result<Vec<String>> {
            Ok(vec!["push_plan".into(), "cancel_plan".into()])
        }
    }

    #[tokio::test]
    async fn workout_delivery_does_not_require_activity_or_health_intake() {
        let destination = Arc::new(CalendarOnlyDestination::default());
        let mut registry = ProviderRegistry::default();
        registry.register_destination(destination.clone());
        assert!(registry.source(ProviderKind::Coros).is_none());
        assert!(registry.legacy(ProviderKind::Coros).is_none());
        let output = registry.destination(ProviderKind::Coros).unwrap();
        let start = chrono::Utc::now().date_naive();
        let target = start + chrono::Duration::days(56);
        let request: PlanRequest = serde_json::from_value(serde_json::json!({
            "distance": "10K",
            "kind": "maintenance",
            "startDate": start,
            "targetDate": target,
            "thresholdSecKm": 300.0,
            "weeklyFrequency": 3,
            "longrunDay": 7,
            "increase": "moderate",
            "currentWeeklyKm": 20.0
        }))
        .unwrap();
        let (plan, workouts) = crate::plan::engine::generate(&request, 20.0).unwrap();
        let progress = Mutex::new(Vec::new());
        let report = |stage: &str, percent: u8| {
            progress.lock().unwrap().push((stage.to_string(), percent));
        };

        assert!(output.is_connected());
        assert_eq!(output.kind(), ProviderKind::Coros);
        assert_eq!(output.connect().await.unwrap(), "connected");
        assert_eq!(
            output.list_api_tools().await.unwrap(),
            ["push_plan", "cancel_plan"]
        );
        let remote_id = output
            .push_plan(&plan, &workouts, Some(&report))
            .await
            .unwrap();
        assert_eq!(remote_id.as_deref(), Some("remote-plan"));
        output
            .cancel_plan(remote_id.as_deref(), None)
            .await
            .unwrap();
        output.cancel_plan(None, None).await.unwrap();
        output.disconnect().await.unwrap();

        assert_eq!(
            *progress.lock().unwrap(),
            [("calendar accepted".into(), 100)]
        );
        assert_eq!(
            *destination.calls.lock().unwrap(),
            [
                "connect".into(),
                format!("push:{}", plan.id),
                "cancel:remote-plan".into(),
                "cancel:discover".into(),
                "disconnect".into(),
            ]
        );
    }

    #[test]
    fn replacing_a_destination_does_not_duplicate_or_add_intake() {
        let mut registry = ProviderRegistry::default();
        let original = Arc::new(CalendarOnlyDestination::default());
        registry.register_destination(original.clone());
        let replacement: Arc<dyn WorkoutDestination> = Arc::new(CalendarOnlyDestination::default());
        registry.register_destination(replacement.clone());

        assert_eq!(registry.destinations().count(), 1);
        assert!(Arc::ptr_eq(
            &registry.destination(ProviderKind::Coros).unwrap(),
            &replacement
        ));
        assert!(registry.source(ProviderKind::Coros).is_none());
        assert!(registry.legacy(ProviderKind::Coros).is_none());
        assert!(registry.destination(ProviderKind::Garmin).is_none());
        assert!(original.calls.lock().unwrap().is_empty());
    }
}
