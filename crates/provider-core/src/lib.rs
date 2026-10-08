//! The contract every wearable adapter implements, plus the ingestion pipeline
//! that turns provider responses into store rows.
//!
//! Three rules shape this crate:
//!
//! 1. **Adapters never touch the database.** They return *drafts* — provider
//!    shapes without local identity. Ids, account keys and cursors belong to
//!    the store, and keeping them out of adapters is what makes a provider
//!    testable against a fixture file instead of a live account.
//! 2. **Sync is resumable and idempotent.** [`ingest`] owns the cursor dance:
//!    read cursor, subtract the overlap window, fetch, upsert, advance the
//!    cursor only for data that landed. A crash mid-sync re-fetches the
//!    overlap and upserts over itself.
//! 3. **Capabilities are declared, not guessed.** A provider says what it can
//!    do via [`Capability`]; the UI and the write-back path branch on that,
//!    never on `match provider { Coros => ..., Garmin => ... }`.

#![forbid(unsafe_code)]

pub mod capability;
pub mod contract;
pub mod error;
pub mod ingest;
pub mod tokens;

pub use capability::Capability;
pub use contract::{
    AccountInfo, ActivityDraft, FitnessDraft, HealthDraft, PushReport, WearableProvider,
    WorkoutPush,
};
pub use error::{ProviderError, Result};
pub use ingest::{
    SyncPolicy, SyncReport, ingest_activities, ingest_all, ingest_fitness, ingest_health,
    register_account,
};
pub use tokens::{InMemoryTokenStore, OAuthToken, TokenStore};
