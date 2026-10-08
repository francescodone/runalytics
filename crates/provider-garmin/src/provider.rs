//! The Garmin adapter: a `WearableProvider` over a local MCP sidecar.
//!
//! Garmin Connect has no official MCP server and no public API for personal
//! data, so the integration is a *sidecar*: a community Python MCP server
//! (typically `garminconnect` + `mcp` under `uvx`) that we spawn as a child
//! process and speak MCP to over stdio. The sidecar owns Garmin's unofficial
//! auth (it keeps its own credential cache on disk); this adapter never sees
//! the athlete's Garmin password.
//!
//! Consequences baked into the design:
//!
//! * **Read-only.** The community servers expose no workout-write tools, so
//!   [`Capability::PLANS_WRITE`] is never advertised and `push_workout`
//!   returns [`ProviderError::Unsupported`] — the contract's documented
//!   behaviour for read-only platforms.
//! * **The child is ours to kill.** The transport is created per session and
//!   the child is torn down when the session is dropped or cancelled, so a
//!   provider leak cannot leave orphaned Python processes.
//! * **No token store.** Auth lives in the sidecar. `connect` spawns it and
//!   asks for tools; if the sidecar cannot serve them (unauthenticated,
//!   crashed on start), connect fails with a message naming the sidecar.

use std::process::Stdio;

use rmcp::{
    ServiceExt as _,
    model::{CallToolRequestParams, CallToolResult, ContentBlock, JsonObject},
    service::{RoleClient, RunningService, ServiceError},
    transport::TokioChildProcess,
};
use runalytics_core::{Date, Provider as CoreProvider, Tz};
use runalytics_provider_core::{
    AccountInfo, ActivityDraft, Capability, FitnessDraft, HealthDraft, ProviderError, Result,
    WearableProvider, WorkoutPush,
};
use serde_json::{Value, json};
use tokio::process::Command;
use tokio::sync::Mutex;

use crate::mapping;
use crate::tools::{Operation, ToolMap};

const PROVIDER_KEY: &str = "garmin";

/// How to launch the sidecar.
#[derive(Debug, Clone)]
pub struct SidecarConfig {
    /// Program to run, e.g. `uvx` or an absolute venv path.
    pub program: String,
    /// Fixed arguments identifying the server package, e.g.
    /// `["--from", "garmin-mcp", "garmin-mcp"]`.
    pub args: Vec<String>,
    /// Extra environment for the child (proxy settings, credential-cache
    /// location). Values are never logged.
    pub env: Vec<(String, String)>,
}

impl Default for SidecarConfig {
    fn default() -> Self {
        Self {
            program: "uvx".into(),
            args: vec!["garmin-mcp".into()],
            env: Vec::new(),
        }
    }
}

/// Configuration for one Garmin account.
#[derive(Debug, Clone)]
pub struct GarminConfig {
    /// Human label for the account; also the identity fallback when the
    /// sidecar offers no whoami tool.
    pub account_label: String,
    /// Sidecar launch recipe.
    pub sidecar: SidecarConfig,
    /// Athlete's timezone for local-day attribution.
    pub timezone: Tz,
}

/// A live stdio session: the running service plus the child handle kept
/// alive so the process stays up (rmcp kills it when the transport is
/// cancelled or dropped).
struct Session {
    running: RunningService<RoleClient, ()>,
    tools: ToolMap,
}

/// The Garmin wearable provider.
pub struct GarminProvider {
    config: GarminConfig,
    session: Mutex<Option<Session>>,
    account: Option<AccountInfo>,
}

impl GarminProvider {
    #[must_use]
    pub fn new(config: GarminConfig) -> Self {
        Self {
            config,
            session: Mutex::new(None),
            account: None,
        }
    }

    /// Spawn the sidecar and open an MCP session over its stdio.
    async fn open_session(&self) -> Result<Session> {
        let mut command = Command::new(&self.config.sidecar.program);
        command.args(&self.config.sidecar.args);
        command.envs(self.config.sidecar.env.iter().cloned());
        // The builder sets stdio itself (its defaults would override anything
        // set on the Command), so the null stderr goes through the builder.
        let (transport, _stderr) = TokioChildProcess::builder(command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|source| ProviderError::Transport {
                provider: PROVIDER_KEY.into(),
                source: anyhow::anyhow!(
                    "could not spawn Garmin sidecar `{} {:?}`: {source}",
                    self.config.sidecar.program,
                    self.config.sidecar.args
                ),
            })?;
        let running =
            ().serve(transport)
                .await
                .map_err(|source| ProviderError::Transport {
                    provider: PROVIDER_KEY.into(),
                    source: anyhow::anyhow!(
                        "Garmin sidecar started but did not complete the MCP handshake: {source}"
                    ),
                })?;
        let tool_names = match running.list_tools(None).await {
            Ok(list) => list
                .tools
                .iter()
                .map(|t| t.name.to_string())
                .collect::<Vec<_>>(),
            Err(e) => {
                let _ = running.cancel().await;
                return Err(ProviderError::Transport {
                    provider: PROVIDER_KEY.into(),
                    source: e.into(),
                });
            }
        };
        let tools = ToolMap::resolve(&tool_names);
        tracing::info!(
            offered = tool_names.len(),
            resolved = tools.resolved_count(),
            "garmin sidecar session established"
        );
        Ok(Session { running, tools })
    }

    /// Call a resolved tool, rebuilding the session once on a
    /// transport-shaped failure (the child died between calls).
    async fn call(&self, op: Operation, args: JsonObject) -> Result<Value> {
        for attempt in 1..=2u8 {
            let mut guard = self.session.lock().await;
            if guard.is_none() {
                let session = self.open_session().await?;
                *guard = Some(session);
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
            let params = CallToolRequestParams::new(tool.clone()).with_arguments(args.clone());
            match session.running.call_tool(params).await {
                Ok(result) => return tool_result_to_json(&result, &tool),
                Err(e) if attempt == 1 && is_transportish(&e) => {
                    tracing::debug!(tool, error = %e, "garmin call failed, respawning sidecar");
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

    /// Fetch one day-granular dataset, mapping each record onto its own
    /// reported date (or the requested day when the record is undated).
    async fn fetch_daily(
        &self,
        op: Operation,
        from: Date,
        to: Date,
        map: impl Fn(&Value, Date) -> Option<HealthDraft>,
    ) -> Result<Vec<HealthDraft>> {
        let mut out = Vec::new();
        let mut day = from;
        while day <= to {
            let mut args = mapping::date_args(day, day);
            args.insert(
                "date".into(),
                Value::String(day.format("%Y-%m-%d").to_string()),
            );
            match self.call(op, args).await {
                Ok(value) => {
                    for record in records_of(&value) {
                        let date = mapping::record_date(&record, day);
                        if let Some(draft) = map(&record, date) {
                            out.push(draft);
                        }
                    }
                }
                // A day with no data at all must stay missing, not zero —
                // the contract's rule for health days. A refused or
                // malformed day is the same shape: skip it, keep syncing.
                Err(
                    ProviderError::Rejected { .. }
                    | ProviderError::Malformed { .. }
                    | ProviderError::Unsupported { .. },
                ) => {
                    tracing::debug!(?op, %day, "garmin daily record unusable, skipping day");
                }
                Err(e) => return Err(e),
            }
            let Some(next) = day.succ_opt() else { break };
            day = next;
        }
        Ok(out)
    }

    /// Dispatch one raw record to the mapper for its operation.
    fn map_health_record(&self, op: Operation, raw: &Value, date: Date) -> Option<HealthDraft> {
        match op {
            Operation::DailySleep => mapping::map_sleep(raw, date, &self.config.timezone),
            Operation::DailyHeart => mapping::map_heart(raw, date),
            Operation::DailyStress => mapping::map_stress(raw, date),
            Operation::Steps => mapping::map_steps(raw, date),
            _ => None,
        }
    }
}

#[async_trait::async_trait]
impl WearableProvider for GarminProvider {
    fn provider(&self) -> CoreProvider {
        CoreProvider::Garmin
    }

    fn capabilities(&self) -> Capability {
        // Best case for community servers: everything read, nothing write.
        Capability::read_only()
    }

    async fn connect(&mut self) -> Result<AccountInfo> {
        let session = self.open_session().await?;
        let tools = session.tools.clone();
        *self.session.lock().await = Some(session);

        let mut capabilities = Capability::empty();
        if tools.tool(Operation::ListActivities).is_some() {
            capabilities |= Capability::ACTIVITIES;
        }
        if tools.tool(Operation::DailySleep).is_some()
            || tools.tool(Operation::DailyHeart).is_some()
        {
            capabilities |= Capability::HEALTH;
        }
        if tools.tool(Operation::Fitness).is_some() {
            capabilities |= Capability::FITNESS;
        }

        let mut account = AccountInfo {
            external_user_id: None,
            account_label: self.config.account_label.clone(),
            region: None,
            capabilities,
        };
        // Identity is a nicety: a sidecar without a whoami tool still
        // connects, it just cannot dedup across reconnects.
        if tools.tool(Operation::Whoami).is_some()
            && let Ok(value) = self.call(Operation::Whoami, JsonObject::new()).await
            && let Some(id) = mapping::str_field(&value, &["userName", "username", "id"])
                .map(str::to_owned)
                .or_else(|| {
                    mapping::num_field(&value, &["userName", "username", "id"])
                        .map(|n| n.to_string())
                })
        {
            account.external_user_id = Some(id);
            account.capabilities |= Capability::IDENTITY;
        }
        self.account = Some(account.clone());
        Ok(account)
    }

    fn account(&self) -> Option<&AccountInfo> {
        self.account.as_ref()
    }

    async fn fetch_activities(&self, from: Date, to: Date) -> Result<Vec<ActivityDraft>> {
        let args = mapping::date_args(from, to);
        let value = self.call(Operation::ListActivities, args).await?;
        let mut drafts = Vec::new();
        for record in records_of(&value) {
            if let Some(draft) = mapping::map_activity(&record, &self.config.timezone) {
                drafts.push(draft);
            }
        }
        drafts.sort_by_key(|d| d.started_at);
        Ok(drafts)
    }

    async fn fetch_health(&self, from: Date, to: Date) -> Result<Vec<HealthDraft>> {
        // The sidecar serves sleep / heart / stress / steps as separate
        // day-granular tools; merge them per day. A day with nothing from
        // any tool stays absent.
        let mut by_day: std::collections::BTreeMap<Date, HealthDraft> =
            std::collections::BTreeMap::default();
        let mut merge = |draft: HealthDraft| {
            let entry = by_day.entry(draft.date).or_insert_with(|| HealthDraft {
                date: draft.date,
                resting_hr: None,
                avg_stress: None,
                high_stress_minutes: None,
                steps: None,
                provider_readiness: None,
                basal_energy: None,
                sleep: None,
            });
            let HealthDraft {
                date: _,
                resting_hr,
                avg_stress,
                high_stress_minutes,
                sleep,
                steps,
                provider_readiness,
                basal_energy,
            } = draft;
            entry.resting_hr = entry.resting_hr.or(resting_hr);
            entry.avg_stress = entry.avg_stress.or(avg_stress);
            entry.high_stress_minutes = entry.high_stress_minutes.or(high_stress_minutes);
            entry.sleep = entry.sleep.take().or(sleep);
            entry.steps = entry.steps.or(steps);
            entry.provider_readiness = entry.provider_readiness.or(provider_readiness);
            entry.basal_energy = entry.basal_energy.or(basal_energy);
        };
        for op in [
            Operation::DailySleep,
            Operation::DailyHeart,
            Operation::DailyStress,
            Operation::Steps,
        ] {
            // A tool the sidecar never offered is not an error — the
            // capability set simply shrinks; fetch what exists.
            let have_tool = self
                .session
                .lock()
                .await
                .as_ref()
                .is_some_and(|s| s.tools.tool(op).is_some());
            if !have_tool {
                continue;
            }
            match self
                .fetch_daily(op, from, to, |raw, date| {
                    self.map_health_record(op, raw, date)
                })
                .await
            {
                Ok(drafts) => drafts.into_iter().for_each(&mut merge),
                Err(e) => {
                    tracing::debug!(?op, error = %e, "garmin health source failed, continuing");
                }
            }
        }
        Ok(by_day.into_values().collect())
    }

    async fn fetch_fitness(&self, from: Date, to: Date) -> Result<Vec<FitnessDraft>> {
        let mut out = Vec::new();
        let mut day = from;
        while day <= to {
            let mut args = mapping::date_args(day, day);
            args.insert(
                "date".into(),
                Value::String(day.format("%Y-%m-%d").to_string()),
            );
            match self.call(Operation::Fitness, args).await {
                Ok(value) => {
                    for record in records_of(&value) {
                        let date = mapping::record_date(&record, day);
                        if let Some(draft) = mapping::map_fitness(&record, date) {
                            out.push(draft);
                        }
                    }
                }
                Err(
                    ProviderError::Rejected { .. }
                    | ProviderError::Malformed { .. }
                    | ProviderError::Unsupported { .. },
                ) => {}
                Err(e) => return Err(e),
            }
            let Some(next) = day.succ_opt() else { break };
            day = next;
        }
        Ok(out)
    }

    async fn push_workout(&self, _push: &WorkoutPush<'_>) -> Result<String> {
        Err(ProviderError::Unsupported {
            provider: PROVIDER_KEY.into(),
            capability: "workout write (community Garmin MCP servers are read-only)".into(),
        })
    }

    async fn delete_workout(&self, _external_id: &str) -> Result<()> {
        Err(ProviderError::Unsupported {
            provider: PROVIDER_KEY.into(),
            capability: "workout delete (community Garmin MCP servers are read-only)".into(),
        })
    }
}

/// True for errors a respawn could plausibly fix (same rule as COROS).
fn is_transportish(e: &ServiceError) -> bool {
    matches!(
        e,
        ServiceError::TransportSend(_)
            | ServiceError::TransportClosed
            | ServiceError::Cancelled { .. }
            | ServiceError::Timeout { .. }
    )
}

/// Flatten a tool result into one JSON value (structured content wins;
/// text blocks parse as JSON; blank becomes an empty array).
fn tool_result_to_json(result: &CallToolResult, tool: &str) -> Result<Value> {
    if result.is_error == Some(true) {
        let text = text_of(result);
        return Err(classify_tool_error(tool, &text));
    }
    if let Some(structured) = &result.structured_content {
        return Ok(structured.clone());
    }
    let text = text_of(result);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(Value::Array(Vec::new()));
    }
    serde_json::from_str(trimmed).map_err(|_| ProviderError::Malformed {
        provider: PROVIDER_KEY.into(),
        detail: format!("tool {tool} returned non-JSON text"),
        sample: Some(trimmed.chars().take(400).collect()),
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

fn classify_tool_error(tool: &str, text: &str) -> ProviderError {
    let lower = text.to_lowercase();
    if lower.contains("unauthorized")
        || lower.contains("token")
        || lower.contains("auth")
        || lower.contains("401")
        || lower.contains("login")
    {
        return ProviderError::Unauthorized {
            provider: PROVIDER_KEY.into(),
            reason: format!("{tool}: {}", text.trim()),
        };
    }
    if lower.contains("not support") || lower.contains("not available") {
        return ProviderError::Unsupported {
            provider: PROVIDER_KEY.into(),
            capability: tool.to_owned(),
        };
    }
    ProviderError::Rejected {
        provider: PROVIDER_KEY.into(),
        reason: format!("{tool}: {}", text.trim()),
    }
}

/// Records out of a tool result: bare arrays pass through, wrapped shapes
/// unwrap, a lone object is a one-record list.
fn records_of(value: &Value) -> Vec<Value> {
    match value {
        Value::Array(a) => a.clone(),
        Value::Object(_) => mapping::record_array(value, PROVIDER_KEY).unwrap_or_else(|_| {
            // A single record (e.g. one day's sleep) is its own array of one.
            vec![value.clone()]
        }),
        _ => Vec::new(),
    }
}

/// Convenience: the default sidecar recipe as JSON, for the settings UI.
#[must_use]
pub fn default_sidecar_json() -> Value {
    json!({
        "program": SidecarConfig::default().program,
        "args": SidecarConfig::default().args,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_result_structured_wins() {
        let mut result = CallToolResult::success(vec![ContentBlock::text("{\"a\": 1}")]);
        result.structured_content = Some(json!({"a": 2}));
        assert_eq!(
            tool_result_to_json(&result, "t").expect("ok"),
            json!({"a": 2})
        );
    }

    #[test]
    fn tool_result_text_parses_or_malforms() {
        let ok = CallToolResult::success(vec![ContentBlock::text("[1,2]")]);
        assert_eq!(tool_result_to_json(&ok, "t").expect("ok"), json!([1, 2]));
        let blank = CallToolResult::success(vec![]);
        assert_eq!(tool_result_to_json(&blank, "t").expect("ok"), json!([]));
        let junk = CallToolResult::success(vec![ContentBlock::text("not json at all")]);
        assert!(matches!(
            tool_result_to_json(&junk, "t"),
            Err(ProviderError::Malformed { .. })
        ));
    }

    #[test]
    fn error_results_classify_like_coros() {
        let mut result = CallToolResult::success(vec![ContentBlock::text(
            "Error: login required, token expired",
        )]);
        result.is_error = Some(true);
        assert!(matches!(
            tool_result_to_json(&result, "get_activities"),
            Err(ProviderError::Unauthorized { .. })
        ));
    }

    #[test]
    fn records_of_handles_all_shapes() {
        assert_eq!(records_of(&json!([1, 2])).len(), 2);
        assert_eq!(records_of(&json!({"data": [1]})).len(), 1);
        assert_eq!(records_of(&json!({"sleepTimeSeconds": 1})).len(), 1);
        assert_eq!(records_of(&json!("junk")).len(), 0);
    }

    #[tokio::test]
    async fn connect_with_unspawnable_sidecar_fails_as_transport() {
        let config = GarminConfig {
            account_label: "test@x".into(),
            sidecar: SidecarConfig {
                program: "/nonexistent-sidecar-binary".into(),
                args: Vec::new(),
                env: Vec::new(),
            },
            timezone: "Europe/Berlin".parse().expect("tz"),
        };
        let mut provider = GarminProvider::new(config);
        let err = provider.connect().await.expect_err("must fail");
        assert!(matches!(err, ProviderError::Transport { .. }));
        assert!(err.to_string().contains("nonexistent-sidecar-binary"));
    }

    #[tokio::test]
    async fn push_is_unsupported_by_contract() {
        use runalytics_core::PlannedSession;
        let provider = GarminProvider::new(GarminConfig {
            account_label: "a@b".into(),
            sidecar: SidecarConfig::default(),
            timezone: "Europe/Berlin".parse().expect("tz"),
        });
        let session = PlannedSession {
            id: runalytics_core::PlannedSessionId::new(),
            date: Date::from_ymd_opt(2026, 10, 12).expect("d"),
            start: runalytics_core::TimeOfDay::MIN,
            kind: runalytics_core::SessionKind::Easy,
            title: "x".into(),
            intent: String::new(),
            workout: runalytics_core::StructuredWorkout::default(),
            target_volume: runalytics_core::VolumeKm::ZERO,
            target_duration: runalytics_core::DurationSecs::ZERO,
            target_pace: None,
            rpe_target: None,
            quality: false,
            external_id: None,
        };
        let push = WorkoutPush {
            session: &session,
            update_key: None,
        };
        assert!(matches!(
            provider.push_workout(&push).await,
            Err(ProviderError::Unsupported { .. })
        ));
    }

    #[test]
    fn capabilities_are_read_only() {
        let caps = GarminProvider::new(GarminConfig {
            account_label: "a@b".into(),
            sidecar: SidecarConfig::default(),
            timezone: "Europe/Berlin".parse().expect("tz"),
        })
        .capabilities();
        assert!(caps.contains(Capability::read_only()));
        assert!(!caps.contains(Capability::PLANS_WRITE));
    }
}
