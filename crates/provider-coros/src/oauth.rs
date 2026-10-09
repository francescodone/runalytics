//! OAuth 2.1 with PKCE for the COROS MCP server.
//!
//! COROS follows the MCP authorization spec: dynamic client registration,
//! authorization-code + PKCE, no client secret. This module owns the *flow
//! mechanics* — verifier/challenge, the consent URL, code exchange, refresh —
//! while the desktop shell owns the parts that need a UI and a socket: opening
//! the browser and receiving the redirect on `http://localhost:{port}/callback`.
//!
//! The separation is testable: every function here is either pure or a plain
//! form POST, and no test ever touches the network.

use base64::Engine as _;
use rand::Rng as _;
use runalytics_provider_core::{OAuthToken, ProviderError, Result};
use serde_json::Value;
use sha2::{Digest as _, Sha256};

use crate::endpoints::Region;

/// The PKCE pair for one consent round.
///
/// The verifier is single-use: reusing it across attempts lets an attacker who
/// saw one failed attempt replay it against the second. Callers must generate
/// a fresh [`PkcePair`] per consent, including retries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PkcePair {
    verifier: String,
    challenge: String,
}

impl PkcePair {
    /// A fresh S256 pair (RFC 7636: 43-128 chars from `[A-Za-z0-9-._~]`).
    #[must_use]
    pub fn new() -> Self {
        // 32 random bytes -> 43 base64url chars, inside the RFC's length band.
        let raw: [u8; 32] = rand::rng().random();
        let verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw);
        let challenge = s256(&verifier);
        Self {
            verifier,
            challenge,
        }
    }

    #[must_use]
    pub fn verifier(&self) -> &str {
        &self.verifier
    }

    #[must_use]
    pub fn challenge(&self) -> &str {
        &self.challenge
    }
}

impl Default for PkcePair {
    fn default() -> Self {
        Self::new()
    }
}

fn s256(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}

/// CSRF/state token binding the consent round-trip to this attempt.
#[must_use]
pub fn csrf_state() -> String {
    let raw: [u8; 16] = rand::rng().random();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw)
}

/// The endpoints discovered from COROS's authorization server metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthEndpoints {
    pub authorize: url::Url,
    pub token: url::Url,
    /// Where this client was registered (dynamic registration), if used.
    pub registration: Option<url::Url>,
}

/// Build the consent URL the desktop shell opens in the system browser.
///
/// `redirect_uri` must be byte-identical to the one registered/announced —
/// COROS compares strings, and a trailing-slash difference fails the exchange
/// with an opaque `invalid_grant`.
///
/// # Errors
///
/// [`ProviderError::OAuth`] if the endpoints or client id are unusable.
pub fn authorization_url(
    endpoints: &AuthEndpoints,
    client_id: &str,
    redirect_uri: &url::Url,
    pkce: &PkcePair,
    state: &str,
    scope: Option<&str>,
) -> Result<url::Url> {
    let mut url = endpoints.authorize.clone();
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("response_type", "code")
            .append_pair("client_id", client_id)
            .append_pair("redirect_uri", redirect_uri.as_str())
            .append_pair("code_challenge", pkce.challenge())
            .append_pair("code_challenge_method", "S256")
            .append_pair("state", state);
        if let Some(scope) = scope {
            q.append_pair("scope", scope);
        }
    }
    Ok(url)
}

/// Discover the authorization-server endpoints for one region via the MCP
/// authorization spec's discovery document, falling back to the conventional
/// paths on the MCP host.
///
/// The desktop shell calls this before starting a consent flow; the provider
/// caches its own copy for refreshes.
///
/// # Errors
///
/// [`ProviderError::OAuth`] if the regional host is unusable (it never should
/// be — the URLs are static constants).
pub async fn discover_endpoints(http: &reqwest::Client, region: Region) -> Result<AuthEndpoints> {
    let mcp = url::Url::parse(region.mcp_url())
        .map_err(|e| ProviderError::OAuth(format!("unparseable MCP url: {e}")))?;
    let host = mcp
        .host_str()
        .ok_or_else(|| ProviderError::OAuth("MCP url has no host".into()))?;
    let well_known = format!("https://{host}/.well-known/oauth-authorization-server");
    let discovered: Value = match http.get(&well_known).send().await {
        Ok(resp) if resp.status().is_success() => resp.json().await.unwrap_or(Value::Null),
        _ => Value::Null,
    };
    let endpoint = |key: &str| {
        discovered
            .get(key)
            .and_then(Value::as_str)
            .and_then(|s| url::Url::parse(s).ok())
    };
    match (
        endpoint("authorization_endpoint"),
        endpoint("token_endpoint"),
    ) {
        (Some(authorize), Some(token)) => Ok(AuthEndpoints {
            authorize,
            token,
            registration: endpoint("registration_endpoint"),
        }),
        _ => Ok(AuthEndpoints {
            authorize: url::Url::parse(&format!("https://{host}/oauth/authorize"))
                .map_err(|e| ProviderError::OAuth(e.to_string()))?,
            token: url::Url::parse(&format!("https://{host}/oauth/token"))
                .map_err(|e| ProviderError::OAuth(e.to_string()))?,
            registration: None,
        }),
    }
}

/// Register this app as an OAuth client (RFC 7591 dynamic registration),
/// returning the issued `client_id`.
///
/// COROS issues public clients (no secret) bound to the exact loopback
/// `redirect_uri` announced here — the same string must appear byte-identical
/// in the consent URL and the code exchange.
///
/// # Errors
///
/// [`ProviderError::OAuth`] on a non-2xx or unreadable registration response;
/// [`ProviderError::Transport`] if the network call fails.
pub async fn register_client(
    http: &reqwest::Client,
    registration_endpoint: &url::Url,
    redirect_uri: &url::Url,
) -> Result<String> {
    let body = serde_json::json!({
        "client_name": "Runalytics",
        "redirect_uris": [redirect_uri.as_str()],
        "grant_types": ["authorization_code", "refresh_token"],
        "response_types": ["code"],
        "token_endpoint_auth_method": "none",
    });
    let response = http
        .post(registration_endpoint.as_str())
        .json(&body)
        .send()
        .await
        .map_err(|e| ProviderError::Transport {
            provider: "coros".into(),
            source: anyhow::Error::new(e).context("registration endpoint"),
        })?;
    let status = response.status();
    let issued: Value = response.json().await.map_err(|e| {
        ProviderError::OAuth(format!(
            "registration endpoint returned unreadable body (HTTP {status}): {e}"
        ))
    })?;
    if !status.is_success() {
        return Err(ProviderError::OAuth(format!(
            "registration endpoint rejected the client with HTTP {status}: {issued}"
        )));
    }
    issued
        .get("client_id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            ProviderError::OAuth(format!(
                "registration response carries no client_id: {issued}"
            ))
        })
}

/// Exchange an authorization code for tokens.
///
/// # Errors
///
/// [`ProviderError::OAuth`] on a non-2xx response or an unreadable body;
/// [`ProviderError::Transport`] if the network call itself fails.
pub async fn exchange_code(
    http: &reqwest::Client,
    endpoints: &AuthEndpoints,
    client_id: &str,
    code: &str,
    redirect_uri: &url::Url,
    pkce: &PkcePair,
) -> Result<OAuthToken> {
    let form = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("client_id", client_id),
        ("redirect_uri", redirect_uri.as_str()),
        ("code_verifier", pkce.verifier()),
    ];
    request_token(http, endpoints, &form).await
}

/// Refresh an access token, preserving the refresh token when the provider
/// rotates it and keeping the old one when it does not.
///
/// # Errors
///
/// As [`exchange_code`].
pub async fn refresh(
    http: &reqwest::Client,
    endpoints: &AuthEndpoints,
    client_id: &str,
    current: &OAuthToken,
) -> Result<OAuthToken> {
    let Some(refresh_token) = current.refresh_token.as_deref() else {
        return Err(ProviderError::OAuth(
            "token has no refresh token; reconnect is required".into(),
        ));
    };
    let form = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", client_id),
    ];
    let fresh = request_token(http, endpoints, &form).await?;
    // A rotating provider returns a new refresh token; one that omits the
    // field means "the old one still works". Losing it would strand the
    // account at the next expiry.
    if fresh.refresh_token.is_none() {
        return Ok(OAuthToken {
            refresh_token: current.refresh_token.clone(),
            ..fresh
        });
    }
    Ok(fresh)
}

#[derive(serde::Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
    #[serde(default)]
    token_type: Option<String>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    error_description: Option<String>,
}

async fn request_token(
    http: &reqwest::Client,
    endpoints: &AuthEndpoints,
    form: &[(&str, &str)],
) -> Result<OAuthToken> {
    let response = http
        .post(endpoints.token.as_str())
        .form(form)
        .send()
        .await
        .map_err(|e| ProviderError::Transport {
            provider: "coros".into(),
            source: anyhow::Error::new(e).context("token endpoint"),
        })?;
    let status = response.status();
    let body: TokenResponse = response.json().await.map_err(|e| {
        ProviderError::OAuth(format!(
            "token endpoint returned unreadable body (HTTP {status}): {e}"
        ))
    })?;
    if let Some(error) = body.error {
        return Err(ProviderError::OAuth(format!(
            "{error}: {}",
            body.error_description
                .unwrap_or_else(|| "no description".into())
        )));
    }
    if !status.is_success() {
        return Err(ProviderError::OAuth(format!(
            "token endpoint rejected the request with HTTP {status}"
        )));
    }
    let expires_in = body.expires_in.unwrap_or(3600);
    Ok(OAuthToken {
        access_token: body.access_token,
        refresh_token: body.refresh_token,
        expires_at: chrono::Utc::now() + chrono::Duration::seconds(expires_in),
        token_type: body.token_type,
        scope: body.scope,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_matches_the_rfc7636_test_vector() {
        // The verifier->challenge transform is the one place a silent
        // implementation bug (wrong base64 variant, wrong hash) still
        // produces a *plausible-looking* URL that fails only at COROS.
        // Pin it against the RFC's worked example.
        let pair = PkcePair {
            verifier: "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk".into(),
            challenge: s256("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
        };
        assert_eq!(
            pair.challenge,
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn fresh_pairs_do_not_repeat() {
        assert_ne!(PkcePair::new().verifier(), PkcePair::new().verifier());
    }

    #[test]
    fn verifier_is_rfc_shaped() {
        let v = PkcePair::new().verifier().to_owned();
        assert!((43..=128).contains(&v.len()));
        assert!(
            v.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-._~".contains(&b))
        );
        assert!(!v.contains('='), "base64url must be unpadded");
    }

    fn endpoints() -> AuthEndpoints {
        AuthEndpoints {
            authorize: url::Url::parse("https://sso.coros.com/oauth/authorize").expect("url"),
            token: url::Url::parse("https://sso.coros.com/oauth/token").expect("url"),
            registration: None,
        }
    }

    #[test]
    fn consent_url_carries_every_required_parameter_verbatim() {
        let url = authorization_url(
            &endpoints(),
            "cid",
            &url::Url::parse("http://localhost:51234/callback").expect("url"),
            &PkcePair::new(),
            "st1te",
            Some("offline_access"),
        )
        .expect("built");
        let q: Vec<(String, String)> = url.query_pairs().into_owned().collect();
        let get = |k: &str| {
            q.iter()
                .find(|(key, _)| key == k)
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        };
        assert_eq!(get("response_type"), "code");
        assert_eq!(get("client_id"), "cid");
        assert_eq!(get("redirect_uri"), "http://localhost:51234/callback");
        assert_eq!(get("code_challenge_method"), "S256");
        assert_eq!(get("state"), "st1te");
        assert_eq!(get("scope"), "offline_access");
    }

    #[tokio::test]
    async fn refresh_without_a_refresh_token_demands_a_reconnect() {
        let current = OAuthToken {
            access_token: "a".into(),
            refresh_token: None,
            expires_at: chrono::Utc::now(),
            token_type: None,
            scope: None,
        };
        let err = refresh(&reqwest::Client::new(), &endpoints(), "cid", &current)
            .await
            .expect_err("must fail");
        assert!(matches!(err, ProviderError::OAuth(_)));
    }
}
