//! OAuth token storage behind a trait.
//!
//! The production implementation lives with the desktop shell, where the
//! macOS Keychain is available (`keyring` with `apple-native`). This crate
//! only defines the contract and an in-memory implementation for tests and
//! for the headless MCP server, which is given tokens by the desktop app
//! rather than owning any itself.

use std::collections::HashMap;
use std::sync::Mutex;

use runalytics_core::Timestamp;

use crate::error::{ProviderError, Result};

/// An OAuth access token plus what is needed to refresh it.
///
/// `refresh_token` is `Option` because some providers issue rotating refresh
/// tokens delivered only with the exchange response; when the next refresh
/// returns a new one, the store is updated with it and the old value is
/// worthless. Storing `None` over `Some` would strand the account.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthToken {
    pub access_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// UTC instant the access token stops being accepted. Providers that omit
    /// `expires_in` are stored far in the future and rely on a 401 to renew.
    pub expires_at: Timestamp,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

impl OAuthToken {
    /// Whether the access token is close enough to expiry to renew now.
    ///
    /// A 60 s margin: a token with 3 s left will be mid-flight-expired by the
    /// time an MCP handshake and a paged fetch finish.
    #[must_use]
    pub fn is_expires_within(&self, horizon: chrono::Duration, now: Timestamp) -> bool {
        self.expires_at - now <= horizon
    }
}

/// Where tokens live. Keys are `(provider, account_label)` — the same identity
/// `provider_account` uses, so a token can never be looked up for the wrong
/// athlete on a shared machine.
///
/// Implementations must treat stored values as secret: never log them, never
/// include them in `Debug` output beyond what [`OAuthToken`] already redacts.
pub trait TokenStore: Send + Sync {
    /// Fetch a token, or [`ProviderError::NotConnected`] if absent.
    fn get(&self, provider: &str, account_label: &str) -> Result<OAuthToken>;

    /// Store or replace a token.
    fn put(&self, provider: &str, account_label: &str, token: &OAuthToken) -> Result<()>;

    /// Forget a token — on disconnect, or when the provider rejects it and a
    /// fresh consent is the only way forward.
    fn remove(&self, provider: &str, account_label: &str) -> Result<()>;
}

/// A process-memory store for tests and for helper processes that receive
/// tokens from the desktop app over their transport.
#[derive(Debug, Default)]
pub struct InMemoryTokenStore {
    entries: Mutex<HashMap<(String, String), OAuthToken>>,
}

impl InMemoryTokenStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl TokenStore for InMemoryTokenStore {
    fn get(&self, provider: &str, account_label: &str) -> Result<OAuthToken> {
        let guard = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard
            .get(&(provider.to_owned(), account_label.to_owned()))
            .cloned()
            .ok_or_else(|| ProviderError::NotConnected {
                provider: provider.to_owned(),
            })
    }

    fn put(&self, provider: &str, account_label: &str, token: &OAuthToken) -> Result<()> {
        self.entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(
                (provider.to_owned(), account_label.to_owned()),
                token.clone(),
            );
        Ok(())
    }

    fn remove(&self, provider: &str, account_label: &str) -> Result<()> {
        self.entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&(provider.to_owned(), account_label.to_owned()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    fn ts(secs: i64) -> Timestamp {
        chrono::Utc.timestamp_opt(secs, 0).unwrap()
    }

    fn token(refresh: Option<&str>) -> OAuthToken {
        OAuthToken {
            access_token: "at".into(),
            refresh_token: refresh.map(str::to_owned),
            expires_at: ts(2_000_000_000),
            token_type: Some("Bearer".into()),
            scope: None,
        }
    }

    #[test]
    fn missing_token_is_not_connected() {
        let store = InMemoryTokenStore::new();
        let err = store.get("coros", "a@b.c").unwrap_err();
        assert!(err.needs_reauth());
    }

    #[test]
    fn accounts_do_not_share_tokens() {
        let store = InMemoryTokenStore::new();
        store.put("coros", "a@b.c", &token(None)).expect("put");
        assert!(store.get("coros", "z@y.x").is_err());
        assert!(store.get("garmin", "a@b.c").is_err());
    }

    #[test]
    fn expiry_horizon_renews_before_the_deadline() {
        let t = token(None);
        assert!(t.is_expires_within(Duration::seconds(60), ts(2_000_000_000 - 30)));
        assert!(!t.is_expires_within(Duration::seconds(60), ts(2_000_000_000 - 3600)));
    }

    #[test]
    fn remove_forgets_only_the_named_account() {
        let store = InMemoryTokenStore::new();
        store.put("coros", "a@b.c", &token(None)).expect("put");
        store.put("coros", "d@e.f", &token(None)).expect("put");
        store.remove("coros", "a@b.c").expect("remove");
        assert_eq!(store.len(), 1);
        assert!(store.get("coros", "d@e.f").is_ok());
    }
}
