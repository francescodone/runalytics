//! Storage errors.

use thiserror::Error;

/// Failures surfaced by the persistence layer.
#[derive(Debug, Error)]
pub enum StoreError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("migration {version} failed: {message}")]
    Migration { version: i64, message: String },

    #[error("database schema is at version {found}, this build needs {wanted}")]
    SchemaTooOld { found: i64, wanted: i64 },

    #[error("no {entity} with id {id}")]
    NotFound { entity: &'static str, id: String },

    #[error("json column is malformed: {0}")]
    CorruptJson(#[from] serde_json::Error),

    #[error("invalid stored value for {field}: {value}")]
    InvalidValue { field: &'static str, value: String },

    #[error(transparent)]
    Domain(#[from] runalytics_core::DomainError),
}

pub type Result<T> = std::result::Result<T, StoreError>;
