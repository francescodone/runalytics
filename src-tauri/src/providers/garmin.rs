//! Garmin provider — feature-gated placeholder.
//!
//! Garmin Connect has no official public API for personal accounts; any
//! integration would rely on unofficial endpoints and is disabled by default.
//! Enable with `cargo build --features garmin` once a client is implemented.

use async_trait::async_trait;

use super::Provider;
use crate::error::{AppError, Result};
use crate::store::Store;
use crate::types::*;

pub struct GarminProvider;

impl GarminProvider {
    pub fn new() -> Self {
        Self
    }
}

fn unsupported() -> Result<()> {
    Err(AppError::Provider(
        "Garmin support is not implemented (no official public API)".into(),
    ))
}

#[async_trait]
impl Provider for GarminProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Garmin
    }
    fn is_connected(&self) -> bool {
        false
    }
    async fn connect(&self) -> Result<String> {
        unsupported()?;
        unreachable!()
    }
    async fn disconnect(&self) -> Result<()> {
        unsupported()
    }
    async fn sync_activities(&self, _store: &Store, _days_back: i64) -> Result<u32> {
        unsupported()?;
        Ok(0)
    }
    async fn sync_health(&self, _store: &Store, _days_back: i64) -> Result<u32> {
        unsupported()?;
        Ok(0)
    }
    async fn sync_fitness(&self, _store: &Store) -> Result<()> {
        unsupported()
    }
    async fn sync_tracks(&self, _store: &Store, _budget: u32) -> Result<u32> {
        unsupported()?;
        Ok(0)
    }
    async fn push_plan(
        &self,
        _plan: &TrainingPlan,
        _workouts: &[PlanWorkout],
        _progress: super::Progress<'_>,
    ) -> Result<Option<String>> {
        unsupported()?;
        unreachable!()
    }
    async fn cancel_plan(
        &self,
        _pushed_id: Option<&str>,
        _progress: super::Progress<'_>,
    ) -> Result<()> {
        unsupported()
    }
    async fn list_api_tools(&self) -> Result<Vec<String>> {
        unsupported()?;
        unreachable!()
    }
}
