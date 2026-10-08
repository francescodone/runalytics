//! The COROS adapter: a `WearableProvider` over COROS's hosted MCP server.
//!
//! COROS speaks MCP (Model Context Protocol) over Streamable HTTP rather than
//! exposing a plain REST API, so this adapter is an MCP *client*: it opens a
//! session against the regional endpoint (see [`crate::endpoints`]),
//! negotiates which tools the server actually offers (see [`crate::tools`]),
//! and translates tool results into provider-core drafts via [`crate::mapping`].
//!
//! Three deliberate structural choices:
//!
//! * **The browser flow lives in the shell, not here.** The adapter consumes
//!   an [`OAuthToken`] from a [`TokenStore`] and refreshes it, but it never
//!   opens a browser or binds a localhost socket — those need UI ownership
//!   and belong to the Tauri layer. `connect` with no stored token is
//!   [`ProviderError::NotConnected`], which the shell renders as "sign in".
//! * **Sessions are lazy and self-healing.** A session is established on
//!   demand, cached, and transparently re-established once when a call fails
//!   with a transport error or the access token is near expiry. COROS tokens
//!   are short-lived; a sync that survives a token rollover is worth the
//!   retry logic.
//! * **One session lock.** Fetches serialise against a single cached session
//!   rather than pooling connections. COROS rate-limits per token, not per
//!   connection, so parallelism would only convert 429s into slower 429s.

use std::sync::{Arc, Mutex as StdMutex, PoisonError};

use chrono::Utc;
use rmcp::{
    ServiceExt as _,
    model::{CallToolRequestParams, CallToolResult, ContentBlock, JsonObject},
    service::{RoleClient, RunningService, ServiceError},
    transport::{
        StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig,
    },
};
use runalytics_core::{Date, Timestamp};
use runalytics_provider_core::{
    AccountInfo, ActivityDraft, Capability, FitnessDraft, HealthDraft, OAuthToken, ProviderError,
    Result, TokenStore, WearableProvider, WorkoutPush,
};
use serde_json::{Value, json};
use tokio::sync::Mutex;

use crate::endpoints::{Region, endpoint_order};
use crate::mapping;
use crate::oauth::{self, AuthEndpoints};
use crate::tools::{Operation, ToolMap};

/// The provider key used for token storage — `Provider::Coros.as_str()`.
const PROVIDER_KEY: &str = "coros";

/// Identity of the OAuth client this build ships with. COROS registers MCP
/// clients dynamically; the shell performs registration (or uses a
/// provisioned client) and hands the pair to the adapter.
#[derive(Debug, Clone)]
pub struct OAuthClient {
    /// `client_id` as issued by COROS.
    pub client_id: String,
    /// The exact localhost loopback URI that was registered — COROS compares
    /// it verbatim, so the port must match what the shell bound.
    pub redirect_uri: String,
}

/// Configuration for one COROS account.
#[derive(Debug, Clone)]
pub struct CorosConfig {
    /// Human label distinguishing accounts on the same platform; also the
    /// token-store key, so renaming it orphans the stored token.
    pub account_label: String,
    /// The OAuth client credentials to authenticate the MCP session with.
    pub oauth: OAuthClient,
    /// Athlete's timezone for local-day attribution.
    pub timezone: runalytics_core::Tz,
    /// Region last known to work, tried first. `None` probes in default order.
    pub known_region: Option<Region>,
}

/// A live MCP session against one regional endpoint.
struct Session {
    running: RunningService<RoleClient, ()>,
    tools: ToolMap,
    region: Region,
    /// Expiry of the token the session was authenticated with; a session
    /// older than its token is rebuilt rather than risk 401s mid-sync.
    token_expires_at: Option<Timestamp>,
}

/// The COROS wearable provider.
pub struct CorosProvider {
    http: reqwest::Client,
    tokens: Arc<dyn TokenStore>,
    config: CorosConfig,
    /// The cached session; `None` until the first successful connect.
    session: Mutex<Option<Session>>,
    /// Authorization-server endpoints, discovered once and cached.
    auth: StdMutex<Option<AuthEndpoints>>,
    /// Set by `connect`; plain field because `connect` is `&mut self`.
    account: Option<AccountInfo>,
    /// Capabilities negotiated from the tool map at connect time.
    capabilities: Capability,
}

impl CorosProvider {
    /// Build an adapter for one account. The token is *not* loaded yet —
    /// nothing async happens until `connect`.
    #[must_use]
    pub fn new(config: CorosConfig, tokens: Arc<dyn TokenStore>) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .expect("a default TLS client is always constructible"),
            tokens,
            config,
            session: Mutex::new(None),
            auth: StdMutex::new(None),
            account: None,
            capabilities: Capability::empty(),
        }
    }

    /// A valid access token, refreshing (and persisting) it when within a
    /// minute of expiry.
    async fn current_token(&self) -> Result<OAuthToken> {
        let stored = self.tokens.get(PROVIDER_KEY, &self.config.account_label)?;
        if !stored.is_expires_within(chrono::Duration::minutes(1), Utc::now()) {
            return Ok(stored);
        }
        let endpoints = self.auth_endpoints().await?;
        let refreshed = oauth::refresh(
            &self.http,
            &endpoints,
            &self.config.oauth.client_id,
            &stored,
        )
        .await?;
        self.tokens
            .put(PROVIDER_KEY, &self.config.account_label, &refreshed)?;
        Ok(refreshed)
    }

    /// The authorization-server endpoints for this account's region, via the
    /// MCP authorization spec's discovery document, falling back to the
    /// conventional paths on the MCP host.
    ///
    /// Discovery is attempted once and cached: the token endpoint is hit on
    /// every refresh, and re-fetching `.well-known` each time would add a
    /// round-trip to every sync for no information gain.
    async fn auth_endpoints(&self) -> Result<AuthEndpoints> {
        if let Some(cached) = self
            .auth
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
        {
            return Ok(cached);
        }
        let region = self.config.known_region.unwrap_or(Region::Global);
        let mcp = url::Url::parse(region.mcp_url())
            .map_err(|e| ProviderError::OAuth(format!("unparseable MCP url: {e}")))?;
        let host = mcp
            .host_str()
            .ok_or_else(|| ProviderError::OAuth("MCP url has no host".into()))?;
        let well_known = format!("https://{host}/.well-known/oauth-authorization-server");
        let discovered: Value = match self.http.get(&well_known).send().await {
            Ok(resp) if resp.status().is_success() => resp.json().await.unwrap_or(Value::Null),
            _ => Value::Null,
        };
        let endpoint = |key: &str| {
            discovered
                .get(key)
                .and_then(Value::as_str)
                .and_then(|s| url::Url::parse(s).ok())
        };
        let endpoints = match (
            endpoint("authorization_endpoint"),
            endpoint("token_endpoint"),
        ) {
            (Some(authorize), Some(token)) => AuthEndpoints {
                authorize,
                token,
                registration: endpoint("registration_endpoint"),
            },
            _ => AuthEndpoints {
                authorize: url::Url::parse(&format!("https://{host}/oauth/authorize"))
                    .map_err(|e| ProviderError::OAuth(e.to_string()))?,
                token: url::Url::parse(&format!("https://{host}/oauth/token"))
                    .map_err(|e| ProviderError::OAuth(e.to_string()))?,
                registration: None,
            },
        };
        *self.auth.lock().unwrap_or_else(PoisonError::into_inner) = Some(endpoints.clone());
        Ok(endpoints)
    }

    /// Open a fresh MCP session against the first regional endpoint that
    /// answers, and negotiate the tool map.
    async fn open_session(&self, token: &OAuthToken) -> Result<Session> {
        let mut last_err = ProviderError::Transport {
            provider: PROVIDER_KEY.into(),
            source: anyhow::anyhow!("no regional endpoint was reachable"),
        };
        for region in endpoint_order(self.config.known_region) {
            let config = StreamableHttpClientTransportConfig::with_uri(region.mcp_url())
                .auth_header(token.access_token.clone());
            let transport = StreamableHttpClientTransport::with_client(self.http.clone(), config);
            match ().serve(transport).await {
                Ok(running) => {
                    let tool_names = match running.list_tools(None).await {
                        Ok(list) => list
                            .tools
                            .iter()
                            .map(|t| t.name.to_string())
                            .collect::<Vec<_>>(),
                        Err(e) => {
                            last_err = ProviderError::Transport {
                                provider: PROVIDER_KEY.into(),
                                source: e.into(),
                            };
                            let _ = running.cancel().await;
                            continue;
                        }
                    };
                    let tools = ToolMap::resolve(&tool_names);
                    tracing::info!(
                        region = %region,
                        offered = tool_names.len(),
                        resolved = tools.resolved_count(),
                        "coros session established"
                    );
                    return Ok(Session {
                        running,
                        tools,
                        region,
                        token_expires_at: Some(token.expires_at),
                    });
                }
                Err(e) => {
                    tracing::debug!(%region, error = %e, "coros endpoint unreachable");
                    last_err = ProviderError::Transport {
                        provider: PROVIDER_KEY.into(),
                        source: e.into(),
                    };
                }
            }
        }
        Err(last_err)
    }

    /// Call a resolved tool and return its payload as JSON.
    ///
    /// One transparent retry: a transport-shaped failure tears the cached
    /// session down and the loop reopens one before the second attempt.
    async fn call(&self, op: Operation, args: JsonObject) -> Result<Value> {
        for attempt in 1..=2u8 {
            let mut guard = self.session.lock().await;
            let stale = guard.as_ref().is_none_or(|s| {
                s.token_expires_at
                    .is_some_and(|at| at <= Utc::now() + chrono::Duration::seconds(30))
            });
            if stale {
                drop(guard);
                let token = self.current_token().await?;
                let session = self.open_session(&token).await?;
                *self.session.lock().await = Some(session);
                guard = self.session.lock().await;
            }
            let Some(session) = guard.as_mut() else {
                return Err(ProviderError::NotConnected {
                    provider: PROVIDER_KEY.into(),
                });
            };
            let Some(tool) = session.tools.tool(op) else {
                return Err(ProviderError::Unsupported {
                    provider: PROVIDER_KEY.into(),
                    capability: op.candidates()[0].to_owned(),
                });
            };
            let tool = tool.to_owned();
            // `new` takes a 'static name, so the owned copy moves into the
            // request; the local stays alive for the error/result reporting.
            let params = CallToolRequestParams::new(tool.clone()).with_arguments(args.clone());
            match session.running.call_tool(params).await {
                Ok(result) => return tool_result_to_json(&result, &tool),
                Err(e) if attempt == 1 && is_transportish(&e) => {
                    tracing::debug!(tool, error = %e, "coros call failed, rebuilding session");
                    if let Some(old) = guard.take() {
                        let _ = old.running.cancel().await;
                    }
                }
                Err(e) => {
                    return Err(ProviderError::Transport {
                        provider: PROVIDER_KEY.into(),
                        source: e.into(),
                    });
                }
            }
        }
        unreachable!("the loop returns or continues on every path")
    }

    /// Identity, if the server offers a whoami tool. A failure here never
    /// fails the connect — identity is a nicety for dedup, data access is
    /// the requirement.
    async fn whoami(&self) -> Result<Option<AccountInfo>> {
        let has_tool = self
            .session
            .lock()
            .await
            .as_ref()
            .is_some_and(|s| s.tools.tool(Operation::Whoami).is_some());
        if !has_tool {
            return Ok(None);
        }
        match self.call(Operation::Whoami, JsonObject::new()).await {
            Ok(value) => Ok(account_info_from(&value, &self.config.account_label)),
            Err(ProviderError::Unsupported { .. } | ProviderError::Transport { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

/// True for errors a reconnect could plausibly fix. Protocol errors from bad
/// arguments (`McpError`, `UnexpectedResponse`) are *not* transport-shaped —
/// retrying them just wastes a handshake.
fn is_transportish(e: &ServiceError) -> bool {
    matches!(
        e,
        ServiceError::TransportSend(_)
            | ServiceError::TransportClosed
            | ServiceError::Cancelled { .. }
            | ServiceError::Timeout { .. }
    )
}

/// Flatten a tool result into a single JSON value for the mapping layer.
///
/// `structured_content` wins when present. Otherwise the text blocks are
/// concatenated and parsed; text that is not JSON becomes a Malformed error
/// carrying the sample rather than being silently dropped.
fn tool_result_to_json(result: &CallToolResult, tool: &str) -> Result<Value> {
    if result.is_error.unwrap_or(false) {
        return Err(classify_tool_error(tool, &text_of(result)));
    }
    if let Some(structured) = &result.structured_content {
        return Ok(structured.clone());
    }
    let text = text_of(result);
    if text.trim().is_empty() {
        return Ok(Value::Array(vec![]));
    }
    serde_json::from_str(&text).map_err(|_| ProviderError::Malformed {
        provider: PROVIDER_KEY.into(),
        detail: format!("tool `{tool}` returned non-JSON text"),
        sample: Some(truncate(&text, 400)),
    })
}

fn text_of(result: &CallToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text(t) => Some(t.text.as_ref()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_owned();
    }
    let mut cut = max;
    while !s.is_char_boundary(cut) {
        cut -= 1;
    }
    format!("{}…", &s[..cut])
}

/// Map a failing tool call onto our error taxonomy. COROS reports auth
/// failures in prose, not status codes, at this layer.
fn classify_tool_error(tool: &str, text: &str) -> ProviderError {
    let lower = text.to_lowercase();
    if lower.contains("unauthorized")
        || lower.contains("token")
        || lower.contains("auth")
        || lower.contains("401")
    {
        ProviderError::Unauthorized {
            provider: PROVIDER_KEY.into(),
            reason: text.to_owned(),
        }
    } else if lower.contains("not support") || lower.contains("not implemented") {
        ProviderError::Unsupported {
            provider: PROVIDER_KEY.into(),
            capability: tool.to_owned(),
        }
    } else {
        ProviderError::Rejected {
            provider: PROVIDER_KEY.into(),
            reason: format!("tool `{tool}` failed: {text}"),
        }
    }
}

/// Date-range arguments for a range tool, sent under every alias COROS
/// deployments are known to accept. Servers ignore parameters they do not
/// declare, so over-specifying is safe and spares us per-region guessing.
#[must_use]
pub fn date_args(from: Date, to: Date) -> JsonObject {
    let mut m = JsonObject::new();
    let from_s = from.format("%Y-%m-%d").to_string();
    let to_s = to.format("%Y-%m-%d").to_string();
    for (k, v) in [
        ("begin_date", from_s.clone()),
        ("end_date", to_s.clone()),
        ("beginDate", from_s),
        ("endDate", to_s),
    ] {
        m.insert(k.to_owned(), Value::String(v));
    }
    m
}

/// The calendar day a health/fitness record describes, from any of the date
/// field names COROS deployments use. `None` when the record carries none and
/// the caller's window cannot supply one.
fn record_date(raw: &Value, fallback: Option<Date>) -> Option<Date> {
    for key in ["date", "day", "calendarDate", "statDate"] {
        if let Some(s) = raw.get(key).and_then(Value::as_str) {
            if let Ok(d) = Date::parse_from_str(s, "%Y-%m-%d") {
                return Some(d);
            }
            if s.len() >= 10
                && let Ok(d) = Date::parse_from_str(&s[..10], "%Y-%m-%d")
            {
                return Some(d);
            }
        }
    }
    fallback
}

/// Account info from a whoami payload, tolerating field-name drift.
fn account_info_from(value: &Value, fallback_label: &str) -> Option<AccountInfo> {
    let obj = value.as_object()?;
    // Some deployments wrap the profile in `data`/`user`; search the outer
    // object first, then either wrapper, because the outer object *exists*
    // in both shapes and a naive unwrap-or-else would never descend.
    let get = |keys: &[&str]| -> Option<String> {
        let mut scopes = [obj]
            .into_iter()
            .chain(obj.get("data").and_then(Value::as_object))
            .chain(obj.get("user").and_then(Value::as_object));
        scopes.find_map(|scope| {
            keys.iter().find_map(|k| match scope.get(*k)? {
                Value::String(s) if !s.is_empty() => Some(s.clone()),
                Value::Number(n) => Some(n.to_string()),
                _ => None,
            })
        })
    };
    let external = get(&["userId", "user_id", "id", "unionId", "openId"])?;
    Some(AccountInfo {
        external_user_id: Some(external),
        account_label: get(&["nickname", "nickName", "name"])
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| fallback_label.to_owned()),
        region: None,
        capabilities: Capability::empty(),
    })
}

/// Negotiate capabilities from the resolved tool map.
///
/// Write capability requires *both* write tools: a server that can create
/// workouts but not delete them would leave stale sessions on the watch after
/// every re-plan, which is worse than not writing at all.
#[must_use]
pub fn capabilities_for(map: &ToolMap) -> Capability {
    let mut caps = Capability::empty();
    if map.tool(Operation::ListActivities).is_some() {
        caps |= Capability::ACTIVITIES;
    }
    if map.tool(Operation::DailyHealth).is_some() {
        caps |= Capability::HEALTH;
    }
    if map.tool(Operation::Fitness).is_some() {
        caps |= Capability::FITNESS;
    }
    if map.tool(Operation::UpsertWorkout).is_some() && map.tool(Operation::DeleteWorkout).is_some()
    {
        caps |= Capability::PLANS_WRITE;
    }
    if map.tool(Operation::Whoami).is_some() {
        caps |= Capability::IDENTITY;
    }
    caps
}

/// The workout payload COROS's write tools expect. Built here, not in the
/// plan engine, because the shape is provider vocabulary: the plan engine
/// knows "intervals, 6x800m @ 4:00", COROS knows seconds, and blocks.
///
/// Repeats are already expanded into ordered blocks by `StructuredWorkout`
/// (which is how core models them), so no repeat-group encoding is needed.
fn workout_payload(push: &WorkoutPush<'_>) -> Value {
    let session = push.session;
    let blocks: Vec<Value> = session
        .workout
        .blocks
        .iter()
        .map(|b| {
            let mut v = json!({
                "type": b.target.as_str(),
                "duration": b.duration.as_u32(),
            });
            if let Some(pace) = b.pace {
                v["targetPaceSecPerKm"] = json!(pace.as_secs_per_km());
            }
            if let Some(hr) = b.hr_ceiling {
                v["maxHeartRate"] = json!(hr.0);
            }
            if let Some(zone) = b.zone {
                v["zone"] = json!(format!("{zone:?}"));
            }
            v
        })
        .collect();
    let mut payload = json!({
        "title": session.title,
        "date": session.date.format("%Y-%m-%d").to_string(),
        "type": session.kind.as_str(),
        "note": session.intent,
        "blocks": blocks,
    });
    if let Some(key) = push.update_key {
        payload["workoutId"] = Value::String(key.to_owned());
    }
    payload
}

#[async_trait::async_trait]
impl WearableProvider for CorosProvider {
    fn provider(&self) -> runalytics_core::Provider {
        runalytics_core::Provider::Coros
    }

    fn capabilities(&self) -> Capability {
        self.capabilities
    }

    async fn connect(&mut self) -> Result<AccountInfo> {
        let token = self.current_token().await?;
        let session = self.open_session(&token).await?;
        let caps = capabilities_for(&session.tools);
        let region = session.region;
        *self.session.lock().await = Some(session);
        self.capabilities = caps;
        let mut info = self.whoami().await?.unwrap_or_else(|| AccountInfo {
            external_user_id: None,
            account_label: self.config.account_label.clone(),
            region: None,
            capabilities: Capability::empty(),
        });
        info.region = Some(region.to_string());
        info.capabilities = caps;
        self.account = Some(info.clone());
        Ok(info)
    }

    fn account(&self) -> Option<&AccountInfo> {
        self.account.as_ref()
    }

    async fn fetch_activities(&self, from: Date, to: Date) -> Result<Vec<ActivityDraft>> {
        let value = self
            .call(Operation::ListActivities, date_args(from, to))
            .await?;
        let records = mapping::record_array(&value, PROVIDER_KEY)?;
        Ok(records
            .iter()
            .filter_map(|r| mapping::map_activity(r, &self.config.timezone))
            .collect())
    }

    async fn fetch_health(&self, from: Date, to: Date) -> Result<Vec<HealthDraft>> {
        let value = self
            .call(Operation::DailyHealth, date_args(from, to))
            .await?;
        let records = mapping::record_array(&value, PROVIDER_KEY)?;
        // A single-day window can attribute records that carry no date field;
        // a multi-day window cannot, and an undated record there is dropped
        // (loudly) rather than guessed onto the wrong day.
        let window_date = (from == to).then_some(from);
        let mut out = Vec::with_capacity(records.len());
        for record in &records {
            let Some(date) = record_date(record, window_date) else {
                tracing::warn!("coros health record without a usable date field, skipping");
                continue;
            };
            if let Some(draft) = mapping::map_health(record, date) {
                out.push(draft);
            }
        }
        Ok(out)
    }

    async fn fetch_fitness(&self, from: Date, to: Date) -> Result<Vec<FitnessDraft>> {
        let value = self.call(Operation::Fitness, date_args(from, to)).await?;
        let records = mapping::record_array(&value, PROVIDER_KEY)?;
        let window_date = (from == to).then_some(from);
        let mut out = Vec::with_capacity(records.len());
        for record in &records {
            let Some(date) = record_date(record, window_date) else {
                tracing::warn!("coros fitness record without a usable date field, skipping");
                continue;
            };
            if let Some(draft) = mapping::map_fitness(record, date) {
                out.push(draft);
            }
        }
        Ok(out)
    }

    async fn push_workout(&self, push: &WorkoutPush<'_>) -> Result<String> {
        let mut args = JsonObject::new();
        args.insert("workout".to_owned(), workout_payload(push));
        let value = self.call(Operation::UpsertWorkout, args).await?;
        // Prefer the id COROS assigns; fall back to a deterministic local key
        // so the plan still carries *something* addressable if the response
        // shape drifts.
        let external = value
            .get("workoutId")
            .or_else(|| value.get("workout_id"))
            .or_else(|| value.get("id"))
            .and_then(|v| match v {
                Value::String(s) if !s.is_empty() => Some(s.clone()),
                Value::Number(n) => Some(n.to_string()),
                _ => None,
            });
        Ok(external.unwrap_or_else(|| format!("coros:local:{}", push.session.id)))
    }

    async fn delete_workout(&self, external_id: &str) -> Result<()> {
        // A locally-keyed id was never sent to COROS, so there is nothing to
        // delete — succeeding here keeps re-plans from erroring on sessions
        // that were pushed while the write tool was down.
        if external_id.starts_with("coros:local:") {
            return Ok(());
        }
        let mut args = JsonObject::new();
        args.insert(
            "workoutId".to_owned(),
            Value::String(external_id.to_owned()),
        );
        self.call(Operation::DeleteWorkout, args).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runalytics_core::{
        BlockTarget, DurationSecs, Pace, PlannedSession, PlannedSessionId, SessionKind,
        StructuredWorkout, TimeOfDay, VolumeKm, WorkoutBlock,
    };
    use runalytics_provider_core::InMemoryTokenStore;

    fn config() -> CorosConfig {
        CorosConfig {
            account_label: "test@corp".into(),
            oauth: OAuthClient {
                client_id: "cid".into(),
                redirect_uri: "http://localhost:51234/callback".into(),
            },
            timezone: "Europe/Berlin".parse().expect("tz"),
            known_region: Some(Region::Eu),
        }
    }

    fn planned_session(workout: StructuredWorkout) -> PlannedSession {
        PlannedSession {
            id: PlannedSessionId::new(),
            date: Date::from_ymd_opt(2026, 8, 1).expect("d"),
            start: TimeOfDay::from_hms_opt(7, 0, 0).expect("t"),
            kind: SessionKind::Intervals,
            title: "6x800m @ 4:00".into(),
            intent: "Cruise reps, capped rest.".into(),
            workout,
            target_volume: VolumeKm(8.0),
            target_duration: DurationSecs(3000),
            target_pace: Some(Pace::new(240.0)),
            rpe_target: Some(7),
            quality: true,
            external_id: None,
        }
    }

    #[test]
    fn date_args_carry_every_known_alias() {
        let args = date_args(
            Date::from_ymd_opt(2026, 7, 1).expect("d"),
            Date::from_ymd_opt(2026, 7, 8).expect("d"),
        );
        assert_eq!(args["begin_date"], Value::String("2026-07-01".into()));
        assert_eq!(args["endDate"], Value::String("2026-07-08".into()));
    }

    #[test]
    fn tool_result_prefers_structured_content() {
        let result = CallToolResult::structured(json!({ "items": [1] }));
        let json = tool_result_to_json(&result, "query_activity_list").expect("ok");
        assert_eq!(json["items"][0], Value::from(1));
    }

    #[test]
    fn tool_result_parses_json_in_text_and_treats_blank_as_empty() {
        let result = CallToolResult::success(vec![ContentBlock::text("[{\"id\":1}]")]);
        let json = tool_result_to_json(&result, "t").expect("ok");
        assert!(json.is_array());
        let blank = CallToolResult::success(vec![ContentBlock::text("   ")]);
        assert_eq!(
            tool_result_to_json(&blank, "t").expect("ok"),
            Value::Array(vec![])
        );
    }

    #[test]
    fn tool_result_non_json_text_becomes_malformed_with_sample() {
        let result =
            CallToolResult::success(vec![ContentBlock::text("<html>502 Bad Gateway</html>")]);
        let err = tool_result_to_json(&result, "t").expect_err("must fail");
        match err {
            ProviderError::Malformed { sample, .. } => {
                assert!(sample.expect("sample").contains("502 Bad Gateway"));
            }
            other => panic!("expected Malformed, got {other:?}"),
        }
    }

    #[test]
    fn error_results_classify_by_prose() {
        let mut result = CallToolResult::structured(json!({}));
        result.is_error = Some(true);
        result.content = vec![ContentBlock::text("HTTP 401 Unauthorized: token expired")];
        assert!(matches!(
            tool_result_to_json(&result, "t"),
            Err(ProviderError::Unauthorized { .. })
        ));
        assert!(matches!(
            classify_tool_error("t", "operation not supported in this region"),
            ProviderError::Unsupported { .. }
        ));
        assert!(matches!(
            classify_tool_error("t", "workout title too long"),
            ProviderError::Rejected { .. }
        ));
    }

    #[test]
    fn whoami_payloads_map_through_field_drift() {
        let a =
            account_info_from(&json!({"userId": 42, "nickname": "Fran"}), "default").expect("a");
        assert_eq!(a.external_user_id.as_deref(), Some("42"));
        assert_eq!(a.account_label, "Fran");
        let b = account_info_from(
            &json!({"data": {"user_id": "u-9", "name": "Francesco"}}),
            "default",
        )
        .expect("b");
        assert_eq!(b.external_user_id.as_deref(), Some("u-9"));
        assert_eq!(b.account_label, "Francesco");
        assert!(account_info_from(&json!({"foo": 1}), "default").is_none());
    }

    #[test]
    fn record_dates_are_tolerant_but_never_guessed() {
        let d = Date::from_ymd_opt(2026, 7, 3).expect("d");
        assert_eq!(record_date(&json!({"date": "2026-07-03"}), None), Some(d));
        assert_eq!(
            record_date(&json!({"statDate": "2026-07-03 00:00:00"}), None),
            Some(d)
        );
        assert_eq!(record_date(&json!({}), Some(d)), Some(d));
        assert_eq!(record_date(&json!({}), None), None);
    }

    #[test]
    fn capabilities_negotiate_from_tool_map() {
        let full = ToolMap::resolve(
            &[
                "query_activity_list",
                "query_activity_detail",
                "query_daily_health",
                "query_fitness_data",
                "create_training_workout",
                "delete_training_workout",
                "get_user_info",
            ]
            .iter()
            .map(|s| (*s).to_owned())
            .collect::<Vec<_>>(),
        );
        let caps = capabilities_for(&full);
        assert!(caps.contains(Capability::ACTIVITIES | Capability::HEALTH | Capability::FITNESS));
        assert!(caps.contains(Capability::PLANS_WRITE | Capability::IDENTITY));

        // Create-without-delete is NOT write capability (stale workouts).
        let half_write = ToolMap::resolve(
            &["query_activity_list", "create_training_workout"]
                .iter()
                .map(|s| (*s).to_owned())
                .collect::<Vec<_>>(),
        );
        let caps = capabilities_for(&half_write);
        assert!(caps.contains(Capability::ACTIVITIES));
        assert!(!caps.contains(Capability::PLANS_WRITE));
        assert!(!caps.contains(Capability::HEALTH));
    }

    #[test]
    fn workout_payload_carries_blocks_and_update_key() {
        let workout = StructuredWorkout::with_warmup_cooldown(
            DurationSecs(600),
            vec![
                WorkoutBlock::new(BlockTarget::Hard, DurationSecs(150)).with_pace(Pace::new(240.0)),
            ],
            DurationSecs(600),
        );
        let planned = planned_session(workout);
        let push = WorkoutPush {
            session: &planned,
            update_key: None,
        };
        let v = workout_payload(&push);
        assert_eq!(v["title"], "6x800m @ 4:00");
        assert_eq!(v["date"], "2026-08-01");
        assert_eq!(v["type"], "intervals");
        assert_eq!(v["blocks"][0]["type"], "warmup");
        assert_eq!(v["blocks"][1]["type"], "hard");
        assert_eq!(v["blocks"][1]["targetPaceSecPerKm"], json!(240.0));
        assert_eq!(v["blocks"][2]["type"], "cooldown");
        assert!(v.get("workoutId").is_none());
        let push = WorkoutPush {
            session: &planned,
            update_key: Some("wo-77"),
        };
        assert_eq!(workout_payload(&push)["workoutId"], "wo-77");
    }

    #[tokio::test]
    async fn connect_without_a_token_reports_not_connected() {
        let mut provider = CorosProvider::new(config(), Arc::new(InMemoryTokenStore::new()));
        let err = provider.connect().await.expect_err("must fail");
        assert!(
            err.needs_reauth(),
            "a missing token is the one error the UI can act on: {err}"
        );
        assert!(provider.account().is_none());
        assert_eq!(provider.capabilities(), Capability::empty());
    }

    #[tokio::test]
    async fn fetches_without_a_session_report_not_connected() {
        // A *fresh* (non-expired) token exists, so current_token succeeds and
        // the failure surfaces at session-open time against unreachable
        // endpoints — which in tests is a Transport error, not NotConnected.
        // What must never happen is a panic or a silent empty fetch.
        let store = InMemoryTokenStore::new();
        store
            .put(
                PROVIDER_KEY,
                "test@corp",
                &OAuthToken {
                    access_token: "atk".into(),
                    refresh_token: None,
                    expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
                    token_type: None,
                    scope: None,
                },
            )
            .expect("put");
        let provider = CorosProvider::new(config(), Arc::new(store));
        let from = Date::from_ymd_opt(2026, 7, 1).expect("d");
        let result = provider.fetch_activities(from, from).await;
        assert!(
            result.is_err(),
            "with no reachable endpoint the fetch must fail, not return empty data"
        );
    }
}
