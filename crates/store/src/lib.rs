//! SQLite persistence for Runalytics.
//!
//! One connection, one migration table, typed repositories. The schema is
//! designed around three realities of a desktop sync app:
//!
//! 1. **Ingestion is idempotent.** Provider rows are keyed by
//!    `(account, provider_activity_id)` so a re-sync updates rather than
//!    duplicates.
//! 2. **Sync is resumable.** Every account carries a cursor per data class, so
//!    an interrupted pull resumes where it stopped instead of re-fetching a
//!    whole history.
//! 3. **Scores are reproducible.** Every score row records the formula version
//!    that produced it, so a formula change backfills cleanly and the UI can
//!    tell the user a number was recomputed.
//!
//! Timestamps are stored as UTC ISO-8601 text. Calendar days are stored as
//! naive `YYYY-MM-DD` because a training day is a local concept and must not
//! shift when the athlete flies.

#![forbid(unsafe_code)]

pub mod db;
pub mod error;
pub mod migrations;
pub mod models;
pub mod repos;

pub use db::Db;
pub use error::{Result, StoreError};
pub use models::*;
pub use repos::{
    ActivityRepo, FeedbackRepo, FitnessRepo, HealthRepo, PlanRepo, ProviderAccountRepo,
    ReadinessRepo, ScoreRepo, SyncRunRepo,
};

/// Schema version owned by this crate. Bumped together with `migrations.rs`.
pub const SCHEMA_VERSION: i64 = 2;
