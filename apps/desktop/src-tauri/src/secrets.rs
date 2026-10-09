//! OS-keychain-backed OAuth token store.
//!
//! Tokens are the one credential class that must never sit in the SQLite
//! file (it is read by the MCP server, backups, and Debug tooling), so they
//! live in the macOS keychain under service `io.runalytics.desktop`, one
//! item per `(provider, account_label)`. The value is the JSON of
//! [`OAuthToken`], which keeps expiry and refresh-token rotation in one
//! atomic write.

use keyring::Entry;
use runalytics_provider_core::{OAuthToken, ProviderError, Result, TokenStore};

/// The keychain service name — also the bundle identifier, so keychain
/// access prompts name the app the user actually launched.
const SERVICE: &str = "io.runalytics.desktop";

/// A [`TokenStore`] backed by the platform keychain (Keychain Services on
/// macOS).
#[derive(Debug, Default)]
pub struct KeychainTokenStore;

impl KeychainTokenStore {
    fn entry(provider: &str, account_label: &str) -> Result<Entry> {
        Entry::new(SERVICE, &format!("{provider}:{account_label}"))
            .map_err(|e| ProviderError::OAuth(format!("keychain unavailable: {e}")))
    }
}

impl TokenStore for KeychainTokenStore {
    fn get(&self, provider: &str, account_label: &str) -> Result<OAuthToken> {
        let entry = Self::entry(provider, account_label)?;
        let raw = entry.get_password().map_err(|e| match e {
            keyring::Error::NoEntry => ProviderError::NotConnected {
                provider: provider.to_string(),
            },
            other => ProviderError::OAuth(format!("keychain read failed: {other}")),
        })?;
        serde_json::from_str(&raw)
            .map_err(|e| ProviderError::OAuth(format!("stored token is unreadable: {e}")))
    }

    fn put(&self, provider: &str, account_label: &str, token: &OAuthToken) -> Result<()> {
        let entry = Self::entry(provider, account_label)?;
        let raw = serde_json::to_string(token)
            .map_err(|e| ProviderError::OAuth(format!("token cannot be stored: {e}")))?;
        entry
            .set_password(&raw)
            .map_err(|e| ProviderError::OAuth(format!("keychain write failed: {e}")))
    }

    fn remove(&self, provider: &str, account_label: &str) -> Result<()> {
        let entry = Self::entry(provider, account_label)?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(other) => Err(ProviderError::OAuth(format!(
                "keychain delete failed: {other}"
            ))),
        }
    }
}
