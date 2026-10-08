//! Errors a provider adapter can produce.
//!
//! The distinction that matters to the app is not *what* went wrong but
//! *whether the user can fix it*: [`ProviderError::Unauthorized`] means "show
//! the reconnect button", everything else means "retry later or file a bug".

use thiserror::Error;

/// Result alias for provider operations.
pub type Result<T, E = ProviderError> = std::result::Result<T, E>;

/// A failure while talking to, or interpreting, a wearable provider.
#[derive(Debug, Error)]
pub enum ProviderError {
    /// The session is not authenticated or the token no longer works.
    ///
    /// The only variant the UI can act on directly, so it is deliberately
    /// narrow: a 500 from a degraded API must not be reported as "log in
    /// again" or users will revoke a working grant.
    #[error("not connected to {provider}: {reason}")]
    Unauthorized { provider: String, reason: String },

    /// The MCP server could not be reached or the transport failed.
    #[error("cannot reach {provider}: {source}")]
    Transport {
        provider: String,
        #[source]
        source: anyhow::Error,
    },

    /// A tool the adapter depends on is absent from the server's tool list.
    ///
    /// Raised at connect time, not per fetch, so a regional server that lacks
    /// health data degrades to "activities only" instead of failing every sync.
    #[error("{provider} does not provide {capability}")]
    Unsupported {
        provider: String,
        capability: String,
    },

    /// The provider answered, but the payload did not match the expected shape.
    ///
    /// Carries a fragment of the payload: provider schema drift is common and
    /// the difference between a fast fix and a blind guess is seeing the body.
    #[error("{provider} returned an unreadable response: {detail}")]
    Malformed {
        provider: String,
        detail: String,
        sample: Option<String>,
    },

    /// A write-back was rejected by the provider.
    #[error("{provider} rejected the workout: {reason}")]
    Rejected { provider: String, reason: String },

    /// Credentials are not in the keychain, or the keychain refused access.
    #[error("no stored credentials for {provider}")]
    NotConnected { provider: String },

    /// An OAuth flow could not be completed.
    #[error("authorization failed: {0}")]
    OAuth(String),
}

impl ProviderError {
    /// The provider's display name, where the variant carries one.
    #[must_use]
    pub fn provider(&self) -> Option<&str> {
        match self {
            Self::Unauthorized { provider, .. }
            | Self::Transport { provider, .. }
            | Self::Unsupported { provider, .. }
            | Self::Malformed { provider, .. }
            | Self::Rejected { provider, .. }
            | Self::NotConnected { provider } => Some(provider),
            Self::OAuth(_) => None,
        }
    }

    /// Whether re-authenticating is the fix. Drives the "Reconnect" action.
    #[must_use]
    pub fn needs_reauth(&self) -> bool {
        matches!(self, Self::Unauthorized { .. } | Self::NotConnected { .. })
    }

    /// Whether trying the same call again could plausibly succeed. Timeouts and
    /// 5xx are worth a retry; a rejected workout or a schema mismatch is not.
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Transport { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_auth_errors_ask_for_a_reconnect() {
        assert!(
            ProviderError::NotConnected {
                provider: "coros".into()
            }
            .needs_reauth()
        );
        assert!(
            !ProviderError::Rejected {
                provider: "coros".into(),
                reason: "date in the past".into()
            }
            .needs_reauth()
        );
    }

    #[test]
    fn transport_is_retryable_and_rejection_is_not() {
        let transport = ProviderError::Transport {
            provider: "coros".into(),
            source: anyhow::anyhow!("connection reset"),
        };
        assert!(transport.is_retryable());
        assert_eq!(transport.provider(), Some("coros"));
        assert!(
            !ProviderError::OAuth("state mismatch".into()).is_retryable(),
            "a CSRF state failure must not be retried"
        );
    }
}
