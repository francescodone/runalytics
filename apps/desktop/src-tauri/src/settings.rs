//! Persisted app settings (SQLite `settings` table, JSON values).
//!
//! The desktop app's own preferences — timezone, calendar name, provider
//! connection settings — live here rather than in tauri-plugin-store so the
//! CLI/MCP server and the GUI read exactly one copy. Secrets never land here;
//! they go to the OS keychain (`secrets.rs`).

use runalytics_store::{Db, SettingsRepo, StoreError};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The settings key under which the app config is stored.
pub const CONFIG_KEY: &str = "app.config";

/// User-configurable app settings with defaults matching the MCP server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppConfig {
    /// IANA time zone for "today" and calendar times.
    pub timezone: String,
    /// Calendar.app calendar the plan is published into.
    pub calendar_name: String,
    /// COROS OAuth client id (from dynamic registration), kept so refreshes
    /// work across launches without re-registering.
    pub coros_client_id: Option<String>,
    /// COROS account label the last successful connect used.
    pub coros_account_label: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            timezone: "Europe/Berlin".to_string(),
            calendar_name: "Runalytics".to_string(),
            coros_client_id: None,
            coros_account_label: None,
        }
    }
}

fn config_error(e: serde_json::Error) -> StoreError {
    StoreError::CorruptJson(e)
}

/// Load the app config, applying defaults over whatever is stored (so adding
/// a field never breaks an existing install).
///
/// # Errors
/// [`StoreError`] on a database failure. A stored value that no longer parses
/// is reported, not silently replaced — the user should know their settings
/// are unreadable.
pub fn load_config(db: &Db) -> Result<AppConfig, StoreError> {
    let Some(raw) = SettingsRepo::get(db, CONFIG_KEY)? else {
        return Ok(AppConfig::default());
    };
    let mut stored: Value = serde_json::from_str(&raw).map_err(config_error)?;
    let mut defaults = serde_json::to_value(AppConfig::default()).map_err(config_error)?;
    if let (Some(base), Some(overlay)) = (defaults.as_object_mut(), stored.as_object_mut()) {
        base.append(overlay);
    }
    serde_json::from_value(defaults).map_err(config_error)
}

/// Persist the app config.
///
/// # Errors
/// [`StoreError`] on a database or serialisation failure.
pub fn save_config(db: &Db, config: &AppConfig) -> Result<(), StoreError> {
    SettingsRepo::set(
        db,
        CONFIG_KEY,
        &serde_json::to_string(config).map_err(config_error)?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_apply_when_nothing_is_stored() {
        let db = Db::in_memory().expect("db");
        assert_eq!(load_config(&db).expect("load"), AppConfig::default());
    }

    #[test]
    fn roundtrip_preserves_values() {
        let db = Db::in_memory().expect("db");
        let config = AppConfig {
            timezone: "America/New_York".into(),
            coros_client_id: Some("cid".into()),
            ..AppConfig::default()
        };
        save_config(&db, &config).expect("save");
        assert_eq!(load_config(&db).expect("load"), config);
    }

    #[test]
    fn missing_fields_in_stored_json_fall_back_to_defaults() {
        let db = Db::in_memory().expect("db");
        // Simulates a config written by an older version of the app.
        SettingsRepo::set(&db, CONFIG_KEY, r#"{"timezone":"Asia/Tokyo"}"#).expect("set");
        let config = load_config(&db).expect("load");
        assert_eq!(config.timezone, "Asia/Tokyo");
        assert_eq!(config.calendar_name, "Runalytics");
    }
}
