//! COROS provider over the official COROS MCP endpoint
//! (https://mcp.coros.com/mcp, OAuth 2.0 + PKCE, stateless streamable HTTP).
//!
//! Flow:
//! 1. `connect()` builds an `OAuthState`, starts authorization with a local
//!    loopback callback (127.0.0.1:<ephemeral>/callback), opens the system
//!    browser, waits for the redirect, exchanges the code, and persists the
//!    resulting tokens into the macOS keychain via a `CredentialStore`.
//! 2. Every operation builds a fresh MCP client over an `AuthClient`-backed
//!    streamable-HTTP transport (tokens auto-refresh through the manager).
//! 3. Tool results arrive as MCP content blocks; COROS returns a
//!    human-readable report inside a JSON-encoded text block (double-quoted
//!    with \n escapes), which `call_tool_text` decodes and the sync parsers
//!    read label-by-label. FIT files arrive as base64 blob resources.

use std::collections::HashMap;
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use chrono::{Datelike, Duration, NaiveDate, Utc};
use keyring::Entry;
use rmcp::model::CallToolResult;
use rmcp::service::{RoleClient, RunningService};
use rmcp::transport::auth::{AuthClient, AuthorizationManager, CredentialStore, StoredCredentials};
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::ServiceExt;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::Provider;
use crate::error::{AppError, Result};
use crate::fit;
use crate::store::Store;
use crate::types::*;

const COROS_MCP_URL: &str = "https://mcp.coros.com/mcp";
const KEYRING_SERVICE: &str = "com.fradone.runalytics.coros";
const KEYRING_USER: &str = "oauth";
/// Upper bound on waiting for the user to finish the browser authorization.
const AUTH_TIMEOUT_SECS: u64 = 300;

/// Keychain-backed rmcp credential store so tokens survive restarts.
struct KeyringCredentialStore;

#[async_trait]
impl CredentialStore for KeyringCredentialStore {
    async fn load(
        &self,
    ) -> std::result::Result<Option<StoredCredentials>, rmcp::transport::auth::AuthError> {
        let entry = Entry::new(KEYRING_SERVICE, KEYRING_USER)
            .map_err(|e| rmcp::transport::auth::AuthError::CredentialStoreError(e.to_string()))?;
        match entry.get_password() {
            Ok(json) => {
                let creds: StoredCredentials = serde_json::from_str(&json).map_err(|e| {
                    rmcp::transport::auth::AuthError::CredentialStoreError(e.to_string())
                })?;
                Ok(Some(creds))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(rmcp::transport::auth::AuthError::CredentialStoreError(
                e.to_string(),
            )),
        }
    }

    async fn save(
        &self,
        credentials: StoredCredentials,
    ) -> std::result::Result<(), rmcp::transport::auth::AuthError> {
        let json = serde_json::to_string(&credentials)
            .map_err(|e| rmcp::transport::auth::AuthError::CredentialStoreError(e.to_string()))?;
        let entry = Entry::new(KEYRING_SERVICE, KEYRING_USER)
            .map_err(|e| rmcp::transport::auth::AuthError::CredentialStoreError(e.to_string()))?;
        entry
            .set_password(&json)
            .map_err(|e| rmcp::transport::auth::AuthError::CredentialStoreError(e.to_string()))
    }

    async fn clear(&self) -> std::result::Result<(), rmcp::transport::auth::AuthError> {
        let entry = Entry::new(KEYRING_SERVICE, KEYRING_USER)
            .map_err(|e| rmcp::transport::auth::AuthError::CredentialStoreError(e.to_string()))?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(rmcp::transport::auth::AuthError::CredentialStoreError(
                e.to_string(),
            )),
        }
    }
}

pub struct CorosProvider {
    connected: AtomicBool,
    /// Serializes connect() so a double-click cannot spawn two callbacks.
    connecting: tokio::sync::Mutex<()>,
    /// Live tool names exposed by the COROS MCP server (fetched once per run).
    tools: tokio::sync::OnceCell<Vec<String>>,
}

impl CorosProvider {
    pub fn new() -> Self {
        Self {
            connected: AtomicBool::new(false),
            connecting: tokio::sync::Mutex::new(()),
            tools: tokio::sync::OnceCell::new(),
        }
    }

    fn has_credentials(&self) -> bool {
        Entry::new(KEYRING_SERVICE, KEYRING_USER)
            .and_then(|e| e.get_password())
            .is_ok()
    }

    /// Build an authorized MCP client. Cheap enough to do per operation:
    /// COROS MCP is stateless, so no session needs to be kept alive.
    async fn client(&self) -> Result<RunningService<RoleClient, ()>> {
        let mut manager = AuthorizationManager::new(COROS_MCP_URL)
            .await
            .map_err(|e| AppError::Provider(format!("auth manager: {e}")))?;
        manager.set_credential_store(KeyringCredentialStore);
        // Set the COROS metadata (issuer on the regional host) before
        // initialize_from_store, which otherwise runs rmcp's strict
        // issuer == base-URL discovery and rejects the split-host setup.
        let metadata = discover_coros_metadata().await?;
        manager.set_metadata(metadata);
        let loaded = manager
            .initialize_from_store()
            .await
            .map_err(|e| AppError::Provider(format!("load credentials: {e}")))?;
        if !loaded {
            return Err(AppError::NotConnected(
                "COROS credentials missing — connect first".into(),
            ));
        }
        // get_access_token validates/expiry-refreshes before the MCP handshake.
        manager
            .get_access_token()
            .await
            .map_err(|e| AppError::NotConnected(format!("token: {e} — reconnect")))?;

        let http = AuthClient::new(reqwest::Client::default(), manager);

        let config = StreamableHttpClientTransportConfig::with_uri(COROS_MCP_URL);
        let transport = StreamableHttpClientTransport::with_client(http, config);
        let service = ()
            .serve(transport)
            .await
            .map_err(|e| AppError::Provider(format!("mcp connect: {e}")))?;
        Ok(service)
    }

    /// Fetch (once) and cache the live tool list from the MCP server.
    async fn live_tools(&self, service: &RunningService<RoleClient, ()>) -> Vec<String> {
        if let Some(t) = self.tools.get() {
            return t.clone();
        }
        let names: Vec<String> = service
            .list_tools(None)
            .await
            .map(|r| r.tools.iter().map(|t| t.name.to_string()).collect())
            .unwrap_or_default();
        let _ = self.tools.set(names.clone());
        names
    }

    /// Map an assumed tool name onto the live tool list: exact match first,
    /// then case-insensitive, then a token-substring heuristic (all words of
    /// the assumed name appear in the live name). Falls back to the assumed
    /// name so the server itself reports unknown tools.
    async fn resolve_tool(
        &self,
        service: &RunningService<RoleClient, ()>,
        assumed: &str,
    ) -> String {
        let live = self.live_tools(service).await;
        if live.is_empty() {
            return assumed.to_string();
        }
        if live.iter().any(|t| t == assumed) {
            return assumed.to_string();
        }
        let lower = assumed.to_lowercase();
        if let Some(t) = live.iter().find(|t| t.to_lowercase() == lower) {
            return t.clone();
        }
        // Token heuristic: look for a live tool whose lowercase name contains
        // every fragment word of the assumed name (e.g. querySleepData ->
        // "query_sleep_data", "getSleepAssessment", …).
        let frags: Vec<&str> = match assumed {
            "querySportRecords" => vec!["sport", "record"],
            "queryDailyHealthData" => vec!["daily", "health"],
            "querySleepData" => vec!["sleep"],
            "querySleepHrv" => vec!["sleep", "hrv"],
            "queryRestingHeartRate" => vec!["resting"],
            "queryFitnessAssessmentOverview" => vec!["fitness"],
            "queryTrainingLoadAssessment" => vec!["load"],
            "downloadActivityFitFiles" => vec!["fit", "file"],
            "queryActivityFitFileDownloadUrls" => vec!["fit", "url"],
            "createTrainingPlan" => vec!["plan"],
            _ => {
                // Generic: split on camel-case boundaries, keep fragments
                // longer than 2 chars, drop the leading query/get/fetch verb.
                let mut out: Vec<&str> = Vec::new();
                let bytes: Vec<char> = assumed.chars().collect();
                let mut start = 0usize;
                for i in 1..bytes.len() {
                    if bytes[i].is_uppercase() {
                        let word: String = bytes[start..i].iter().collect();
                        out.push(Box::leak(word.into_boxed_str()));
                        start = i;
                    }
                }
                let last: String = bytes[start..].iter().collect();
                out.push(Box::leak(last.into_boxed_str()));
                out.retain(|w| {
                    w.len() > 2 && !matches!(*w, "query" | "get" | "fetch" | "download")
                });
                if out.is_empty() {
                    return assumed.to_string();
                }
                out
            }
        };
        let mut best: Option<(usize, String)> = None;
        for t in &live {
            let tl = t.to_lowercase();
            // "download" contains "load" — never treat a download tool as the
            // training-load assessment.
            if frags.contains(&&"load") && tl.contains("download") {
                continue;
            }
            // The plain sleep tool must not steal the sleep-HRV tool.
            if frags == ["sleep"] && tl.contains("hrv") {
                continue;
            }
            let hits = frags.iter().filter(|f| tl.contains(*f)).count();
            if hits == frags.len() && frags.len() > 0 {
                let score = tl.len();
                match &best {
                    Some((s, _)) if *s <= score => {}
                    _ => best = Some((score, t.clone())),
                }
            }
        }
        best.map(|(_, t)| t).unwrap_or_else(|| assumed.to_string())
    }

    /// Call a COROS tool (resolving its real name first) and return the raw
    /// MCP result (text + embedded resource blocks).
    async fn call_tool_raw(
        &self,
        client: &RunningService<RoleClient, ()>,
        name: &str,
        args: Value,
    ) -> Result<CallToolResult> {
        let name = self.resolve_tool(client, name).await;
        let params = rmcp::model::CallToolRequestParams::new(name.to_string())
            .with_arguments(serde_json::from_value(args).unwrap_or_default());
        let result: CallToolResult = client
            .call_tool(params)
            .await
            .map_err(|e| AppError::Provider(format!("{name}: {e}")))?;
        let text = Self::text_of(&result);
        if result.is_error.unwrap_or(false) || text.contains("Tool call anomalies detected") {
            return Err(AppError::Provider(format!(
                "{name}: {}",
                text.chars().take(200).collect::<String>()
            )));
        }
        Ok(result)
    }

    /// Concatenate the text blocks of a result. COROS double-encodes its
    /// human-readable report: the block holds a JSON string literal (quoted,
    /// with `\n` escapes), so decode it back to real text.
    fn text_of(result: &CallToolResult) -> String {
        let mut text = String::new();
        for block in &result.content {
            if let rmcp::model::ContentBlock::Text(t) = block {
                text.push_str(&t.text);
            }
        }
        let t = text.trim();
        if t.starts_with('"') {
            if let Ok(s) = serde_json::from_str::<String>(t) {
                return s;
            }
        }
        t.trim_matches('"').replace("\\n", "\n")
    }

    /// Call a tool and return its decoded human-readable report.
    async fn call_tool_text(
        &self,
        client: &RunningService<RoleClient, ()>,
        name: &str,
        args: Value,
    ) -> Result<String> {
        let r = self.call_tool_raw(client, name, args).await?;
        let text = Self::text_of(&r);
        eprintln!(
            "[coros] {} -> {} chars: {}",
            name,
            text.len(),
            text.chars().take(120).collect::<String>()
        );
        Ok(text)
    }

    /// Extract (uri, decoded bytes) from embedded blob resource blocks —
    /// this is how COROS delivers FIT files.
    fn blobs_of(result: &CallToolResult) -> Vec<(String, Vec<u8>)> {
        let mut out = Vec::new();
        for block in &result.content {
            if let rmcp::model::ContentBlock::Resource(e) = block {
                if let rmcp::model::ResourceContents::BlobResourceContents { uri, blob, .. } =
                    &e.resource
                {
                    match base64_decode(blob) {
                        Ok(bytes) => out.push((uri.clone(), bytes)),
                        Err(err) => eprintln!("[coros] bad base64 in {uri}: {err}"),
                    }
                }
            }
        }
        out
    }

    /// Number right after a label: "Avg HR: 148 bpm" -> 148.0. Handles
    /// thousands separators ("Steps: 18,176").
    fn num_after(text: &str, label: &str) -> Option<f64> {
        let i = text.find(label)? + label.len();
        let rest = text[i..].trim_start();
        let end = rest
            .find(|c: char| !(c.is_ascii_digit() || c == ',' || c == '.'))
            .unwrap_or(rest.len());
        if end == 0 {
            return None;
        }
        rest[..end].replace(',', "").parse().ok()
    }

    /// Alphanumeric token right after a label: "LabelId: 481003780305879140".
    fn token_after(text: &str, label: &str) -> Option<String> {
        let i = text.find(label)? + label.len();
        let rest = text[i..].trim_start();
        let end = rest
            .find(|c: char| !c.is_ascii_alphanumeric())
            .unwrap_or(rest.len());
        if end == 0 {
            return None;
        }
        Some(rest[..end].to_string())
    }

    /// Seconds from a clock value after a label: "Duration: 1:33:27 |" or
    /// "Threshold Pace: 3:53 /km".
    fn clock_after(text: &str, label: &str) -> Option<f64> {
        let i = text.find(label)? + label.len();
        let rest = text[i..].trim_start();
        let end = rest
            .find(|c: char| !(c.is_ascii_digit() || c == ':'))
            .unwrap_or(rest.len());
        if end == 0 {
            return None;
        }
        Self::parse_clock(&rest[..end])
    }

    /// "38:02" -> 2282 s, "1:33:27" -> 5607 s.
    fn parse_clock(s: &str) -> Option<f64> {
        let parts: Vec<f64> = s
            .split(':')
            .filter_map(|p| p.trim().parse::<f64>().ok())
            .collect();
        match parts.len() {
            2 => Some(parts[0] * 60.0 + parts[1]),
            3 => Some(parts[0] * 3600.0 + parts[1] * 60.0 + parts[2]),
            _ => None,
        }
    }

    /// Like [`Self::clock_after`], but the label must *begin* a line. Required
    /// wherever one label is a suffix of another: "Marathon Prediction:" also
    /// occurs inside "Half Marathon Prediction:", so a plain substring search
    /// hands the half-marathon time to the marathon field as well.
    fn clock_after_line(text: &str, label: &str) -> Option<f64> {
        for line in text.lines() {
            let Some(rest) = line.trim_start().strip_prefix(label) else {
                continue;
            };
            let rest = rest.trim_start();
            let end = rest
                .find(|c: char| !(c.is_ascii_digit() || c == ':'))
                .unwrap_or(rest.len());
            if end > 0 {
                return Self::parse_clock(&rest[..end]);
            }
        }
        None
    }

    /// Minutes from a duration phrase after a label: "Total: 8h 18min |" -> 498.
    fn min_after(text: &str, label: &str) -> Option<f64> {
        let i = text.find(label)? + label.len();
        let seg = text[i..].split('|').next().unwrap_or("");
        let seg = seg.trim();
        if let Some(hpos) = seg.find('h') {
            let h: f64 = seg[..hpos].trim().parse().ok()?;
            let rest = &seg[hpos + 1..];
            let m: f64 = match rest.find("min") {
                Some(mpos) => rest[..mpos].trim().parse().unwrap_or(0.0),
                None => 0.0,
            };
            Some(h * 60.0 + m)
        } else {
            let mpos = seg.find("min")?;
            seg[..mpos].trim().parse().ok()
        }
    }

    /// Find the first date (YYYY-MM-DD or YYYYMMDD) anywhere in a string.
    fn find_date(s: &str) -> Option<NaiveDate> {
        let chars: Vec<char> = s.chars().collect();
        if chars.len() >= 10 {
            for i in 0..=chars.len() - 10 {
                let w: String = chars[i..i + 10].iter().collect();
                if let Ok(d) = NaiveDate::parse_from_str(&w, "%Y-%m-%d") {
                    return Some(d);
                }
            }
        }
        if chars.len() >= 8 {
            for i in 0..=chars.len() - 8 {
                let w: String = chars[i..i + 8].iter().collect();
                if w.chars().all(|c| c.is_ascii_digit()) {
                    if let Ok(d) = NaiveDate::parse_from_str(&w, "%Y%m%d") {
                        return Some(d);
                    }
                }
            }
        }
        None
    }

    /// Map a COROS record header ("Outdoor Run") to the app's sport label.
    fn sport_label(raw: &str) -> String {
        let l = raw.to_lowercase();
        if l.contains("run") || l.contains("jog") || l.contains("walk") {
            "run".into()
        } else if l.contains("cycle") || l.contains("ride") || l.contains("bike") {
            "ride".into()
        } else if l.contains("swim") {
            "swim".into()
        } else if l.contains("strength") || l.contains("gym") {
            "strength".into()
        } else {
            l.split_whitespace().next().unwrap_or("run").to_string()
        }
    }

    /// COROS dubbo sport-type codes needed by the FIT download tool. Kept in
    /// sync with the labels `sport_label` produces; a code missing here means
    /// that sport's activities can never have their FIT file fetched.
    fn sport_code(sport: &str) -> Option<i64> {
        let l = sport.to_lowercase();
        if l.contains("run") || l.contains("jog") || l.contains("walk") {
            Some(100)
        } else if l.contains("hik") {
            Some(104)
        } else if l.contains("cycl") || l.contains("ride") || l.contains("bike") {
            Some(200)
        } else if l.contains("swim") {
            Some(300)
        } else if l.contains("strength") || l.contains("gym") {
            Some(402)
        } else {
            None
        }
    }

    fn empty_health(date: NaiveDate) -> HealthDaily {
        HealthDaily {
            date,
            steps: None,
            avg_hr: None,
            resting_hr: None,
            stress_avg: None,
            sleep_score: None,
            sleep_min: None,
            deep_min: None,
            rem_min: None,
            hrv_avg: None,
        }
    }

    /// Turn the `querySportRecords` report into activity rows. Records are
    /// numbered blocks separated by a blank line:
    /// "1. Outdoor Run — 2026-10-13\n   …Duration: 38:02 | Distance: 9.01 km…".
    /// A block that can't be read is skipped rather than failing the sync.
    fn parse_sport_records(text: &str) -> Vec<fit::DecodedActivity> {
        let mut out = Vec::new();
        for block in text.split("\n\n") {
            let header = match block.lines().next() {
                Some(h) => h.trim(),
                None => continue,
            };
            // Header like "3. Strength — 2026-10-08".
            let Some(dash) = header.find('—') else {
                continue;
            };
            let name_raw = header[..dash].trim();
            let name = name_raw
                .split_once(". ")
                .map(|(_, n)| n.trim())
                .unwrap_or(name_raw)
                .to_string();
            let date = match Self::find_date(&header[dash..]) {
                Some(d) => d,
                None => continue,
            };
            let label_id = match Self::token_after(block, "LabelId:") {
                Some(id) => id,
                None => continue,
            };
            let start_ts = Self::num_after(block, "startTimestamp=").unwrap_or_else(|| {
                date.and_hms_opt(0, 0, 0)
                    .map(|dt| dt.and_utc().timestamp() as f64)
                    .unwrap_or(0.0)
            });
            let duration_s = Self::clock_after(block, "Duration:").unwrap_or(0.0);
            let distance_m = Self::num_after(block, "Distance:").unwrap_or(0.0) * 1000.0;
            // Strength sessions report Sets instead of Distance, so a missing
            // distance is normal — only a record with neither distance nor
            // duration is unusable. Requiring both silently dropped every
            // strength workout.
            if distance_m <= 0.0 && duration_s <= 0.0 {
                continue;
            }
            out.push(fit::DecodedActivity {
                record_id: label_id,
                sport: Self::sport_label(&name),
                // The numeric code is what the FIT download tool wants back;
                // the label above is lossy ("Hike"/"Cycling" have no inverse).
                provider_sport_code: Self::num_after(block, "SportType:").map(|v| v as i64),
                start_ts: start_ts as i64,
                duration_s,
                distance_m,
                avg_hr: Self::num_after(block, "Avg HR:").map(|v| v as i64),
                max_hr: None,
                elevation_gain_m: Self::num_after(block, "Elevation Gain:")
                    .or_else(|| Self::num_after(block, "Elevation:")),
                points: vec![],
            });
        }
        out
    }
}

/// Pick the first present key out of a JSON object.
fn json_pick<'a>(v: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|k| v.get(*k)).filter(|x| !x.is_null())
}

/// Minimal loopback HTTP server: accepts one GET on /callback and returns the
/// (code, state, iss) triple. Serves a friendly HTML page on success.
async fn wait_for_callback(
    listener: TcpListener,
    timeout: std::time::Duration,
) -> Result<(String, String, Option<String>)> {
    let (mut sock, _) = tokio::net::TcpListener::from_std(listener)
        .map_err(|e| AppError::Other(format!("callback listener: {e}")))?
        .accept()
        .await
        .map_err(|e| AppError::Other(format!("callback accept: {e}")))?;

    let result = tokio::time::timeout(timeout, async {
        let mut buf = vec![0u8; 8192];
        let mut read = 0usize;
        loop {
            let n = sock
                .read(&mut buf[read..])
                .await
                .map_err(|e| AppError::Other(format!("callback read: {e}")))?;
            read += n;
            if read == 0 || buf[..read].windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }
        let request = String::from_utf8_lossy(&buf[..read]).to_string();
        let line = request
            .lines()
            .next()
            .ok_or_else(|| AppError::Other("empty callback request".into()))?;
        // "GET /callback?code=...&state=... HTTP/1.1"
        let path = line.split_whitespace().nth(1).unwrap_or("/");
        let query = path.split_once('?').map(|(_, q)| q).unwrap_or("");
        let mut code = None;
        let mut state = None;
        let mut iss = None;
        for pair in query.split('&') {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            let v = urlencoding(v);
            match k {
                "code" => code = Some(v),
                "state" => state = Some(v),
                "iss" => iss = Some(v),
                _ => {}
            }
        }
        let code = code.ok_or_else(|| AppError::Provider("no code in callback".into()))?;
        let state = state.ok_or_else(|| AppError::Provider("no state in callback".into()))?;
        let page = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\
                     Connection: close\r\n\r\n\
                     <html><body style=\"font-family:sans-serif;padding:2em\">\
                     <h2>Runalytics</h2><p>COROS connected. You can close this tab.</p>\
                     </body></html>";
        let _ = sock.write_all(page).await;
        let _ = sock.shutdown().await;
        Ok::<_, AppError>((code, state, iss))
    })
    .await
    .map_err(|_| AppError::Provider("authorization timed out".into()))??;
    Ok(result)
}

fn urlencoding(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("00");
                out.push(char::from(u8::from_str_radix(hex, 16).unwrap_or(0)));
                i += 3;
            }
            b'+' => {
                out.push(' ');
                i += 1;
            }
            c => {
                out.push(c as char);
                i += 1;
            }
        }
    }
    out
}

/// COROS fronts the MCP endpoint on a regional host (e.g. mcp.coros.com
/// proxies to the EU cluster mcpeu.coros.com, which hosts OAuth). rmcp's
/// own discovery validates issuer == base URL and rejects that split, so
/// we resolve the metadata ourselves from the 401 challenge and hand it
/// to the manager via set_metadata.
async fn discover_coros_metadata() -> Result<rmcp::transport::auth::AuthorizationMetadata> {
    let client = reqwest::Client::new();
    // 1. Probe the MCP endpoint; the 401 carries the resource-metadata URL.
    let probe = client
        .post(COROS_MCP_URL)
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .body("{}")
        .send()
        .await
        .map_err(|e| AppError::Provider(format!("probe COROS MCP: {e}")))?;
    let www = probe
        .headers()
        .get("www-authenticate")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let resource_meta = www
        .find("resource_metadata=\"")
        .map(|i| {
            let rest = &www[i + "resource_metadata=\"".len()..];
            rest.split('"').next().unwrap_or("").to_string()
        })
        .filter(|s| !s.is_empty())
        // Fall back to the conventional path on the MCP host itself.
        .unwrap_or_else(|| "https://mcp.coros.com/.well-known/oauth-protected-resource/mcp".into());

    // 2. Protected-resource metadata names the authorization server.
    let prm: Value = client
        .get(&resource_meta)
        .send()
        .await
        .map_err(|e| AppError::Provider(format!("resource metadata: {e}")))?
        .json()
        .await
        .map_err(|e| AppError::Provider(format!("resource metadata json: {e}")))?;
    let auth_server = json_pick(&prm, &["authorization_server", "authorization_servers"])
        .map(|v| match v {
            Value::String(s) => s.clone(),
            Value::Array(a) => a
                .first()
                .and_then(|x| x.as_str())
                .unwrap_or("https://mcpeu.coros.com")
                .to_string(),
            _ => "https://mcpeu.coros.com".to_string(),
        })
        .unwrap_or_else(|| "https://mcpeu.coros.com".to_string());
    let issuer = auth_server.trim_end_matches('/').to_string();

    // 3. Fetch the authorization-server document from the issuer itself,
    //    where the issuer field trivially matches.
    let asm: Value = client
        .get(format!("{issuer}/.well-known/oauth-authorization-server"))
        .send()
        .await
        .map_err(|e| AppError::Provider(format!("auth server metadata: {e}")))?
        .json()
        .await
        .map_err(|e| AppError::Provider(format!("auth server metadata json: {e}")))?;
    let need = |k: &str| -> Result<String> {
        asm.get(k)
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| AppError::Provider(format!("oauth metadata missing {k}")))
    };
    let mut metadata = rmcp::transport::auth::AuthorizationMetadata::default();
    metadata.authorization_endpoint = need("authorization_endpoint")?;
    metadata.token_endpoint = need("token_endpoint")?;
    metadata.registration_endpoint = asm
        .get("registration_endpoint")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    metadata.issuer = Some(issuer.clone());
    // Preserve flags rmcp reads out of additional_fields (e.g. RFC 9207 iss).
    if let Value::Object(map) = asm {
        metadata.additional_fields = map.into_iter().collect();
    }
    Ok(metadata)
}

#[async_trait]
impl Provider for CorosProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Coros
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed) || self.has_credentials()
    }

    async fn connect(&self) -> Result<String> {
        let _guard = self
            .connecting
            .try_lock()
            .map_err(|_| AppError::Other("connect already running".into()))?;

        let mut manager = AuthorizationManager::new(COROS_MCP_URL)
            .await
            .map_err(|e| AppError::Provider(format!("auth manager: {e}")))?;
        manager.set_credential_store(KeyringCredentialStore);
        // Set the COROS metadata (issuer on the regional host) before
        // initialize_from_store, which otherwise runs rmcp's strict
        // issuer == base-URL discovery and rejects the split-host setup.
        let metadata = discover_coros_metadata().await?;
        manager.set_metadata(metadata);
        // Reuse a previously registered client id when present.
        let _ = manager.initialize_from_store().await;

        // Ephemeral loopback port for the redirect.
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|e| AppError::Other(format!("bind callback port: {e}")))?;
        let port = listener.local_addr()?.port();
        let redirect = format!("http://127.0.0.1:{port}/callback");

        // Dynamic client registration unless a stored client id configured us.
        let configured = manager
            .get_credentials()
            .await
            .map(|(id, _)| !id.is_empty())
            .unwrap_or(false);
        if !configured {
            manager
                .register_client("Runalytics", &redirect, &[])
                .await
                .map_err(|e| AppError::Provider(format!("client registration: {e}")))?;
        }
        let url = manager
            .get_authorization_url(&[])
            .await
            .map_err(|e| AppError::Provider(format!("auth url: {e}")))?;

        // Open the system browser (no CSP involvement — this is an OS open).
        if let Some(handle) = APP_HANDLE.get() {
            use tauri_plugin_opener::OpenerExt;
            let _ = handle.opener().open_url(&url, None::<String>);
        }

        let (code, csrf, iss) =
            wait_for_callback(listener, std::time::Duration::from_secs(AUTH_TIMEOUT_SECS)).await?;
        let token = manager
            .exchange_code_for_token_with_issuer(&code, &csrf, iss.as_deref())
            .await
            .map_err(|e| AppError::Provider(format!("token exchange: {e}")))?;

        // Persist explicitly so a restart can rebuild the manager from the
        // keychain (initialize_from_store).
        let (client_id, _) = manager
            .get_credentials()
            .await
            .map_err(|e| AppError::Provider(format!("client id: {e}")))?;
        let scopes = manager.get_current_scopes().await;
        let creds = StoredCredentials::new(
            client_id,
            Some(token),
            scopes,
            Some(Utc::now().timestamp() as u64),
        );
        KeyringCredentialStore
            .save(creds)
            .await
            .map_err(|e| AppError::Provider(format!("save credentials: {e}")))?;
        self.connected.store(true, Ordering::Relaxed);
        Ok("COROS account connected".into())
    }

    async fn disconnect(&self) -> Result<()> {
        KeyringCredentialStore
            .clear()
            .await
            .map_err(|e| AppError::Provider(format!("clear credentials: {e}")))?;
        self.connected.store(false, Ordering::Relaxed);
        Ok(())
    }

    async fn sync_activities(&self, store: &Store, days_back: i64) -> Result<u32> {
        let service = self.client().await?;
        let from = (Utc::now() - Duration::days(days_back)).date_naive();
        // COROS wants yyyyMMdd and chokes on ISO dates; limit generously.
        let args = json!({
            "startDate": from.format("%Y%m%d").to_string(),
            "endDate": Utc::now().date_naive().format("%Y%m%d").to_string(),
            "limit": 100,
        });
        let text = self
            .call_tool_text(&service, "querySportRecords", args)
            .await?;
        let mut count = 0u32;
        for decoded in Self::parse_sport_records(&text) {
            if store
                .upsert_activity(ProviderKind::Coros, &decoded)?
                .is_some()
            {
                count += 1;
            }
        }
        store.set_last_sync(ProviderKind::Coros, Utc::now())?;
        Ok(count)
    }

    async fn sync_health(&self, store: &Store, days_back: i64) -> Result<u32> {
        let service = self.client().await?;
        let days_arg = json!({ "days": days_back.max(7) as i64 });
        let mut days: HashMap<NaiveDate, HealthDaily> = HashMap::new();

        // Daily overview: steps + sleep summary + sleep HR, "--- 20261012 ---".
        match self
            .call_tool_text(&service, "queryDailyHealthData", days_arg.clone())
            .await
        {
            Ok(text) => {
                for block in text.split("--- ") {
                    let Some(d) = Self::find_date(block.lines().next().unwrap_or("")) else {
                        continue;
                    };
                    let h = days.entry(d).or_insert_with(|| Self::empty_health(d));
                    h.steps = Self::num_after(block, "Steps:").map(|v| v as i64);
                    h.avg_hr = Self::num_after(block, "Sleep HR: Avg").map(|v| v as i64);
                    h.sleep_min = Self::min_after(block, "Total:").map(|v| v as i64);
                    h.deep_min = Self::min_after(block, "Deep:").map(|v| v as i64);
                    h.rem_min = Self::min_after(block, "REM:").map(|v| v as i64);
                }
            }
            Err(e) => eprintln!("[coros] daily health unavailable: {e}"),
        }

        // Sleep overview: score + main sleep duration (deep/REM via ratios).
        match self
            .call_tool_text(&service, "querySleepOverview", days_arg.clone())
            .await
        {
            Ok(text) => {
                for block in text.split("\n\n") {
                    let first = block.lines().next().unwrap_or("");
                    if !first.starts_with("20") {
                        continue;
                    }
                    let Some(d) = Self::find_date(first) else {
                        continue;
                    };
                    let h = days.entry(d).or_insert_with(|| Self::empty_health(d));
                    h.sleep_score = Self::num_after(block, "Sleep Score:").map(|v| v as i64);
                    let main = Self::min_after(block, "Main Sleep (asleep):");
                    if h.sleep_min.is_none() {
                        h.sleep_min = main.map(|v| v as i64);
                    }
                    if let (Some(main), Some(ratio)) =
                        (main, Self::num_after(block, "Deep Sleep Ratio:"))
                    {
                        h.deep_min = Some((main * ratio / 100.0) as i64);
                    }
                    if let (Some(main), Some(ratio)) = (main, Self::num_after(block, "REM Ratio:"))
                    {
                        h.rem_min = Some((main * ratio / 100.0) as i64);
                    }
                }
            }
            Err(e) => eprintln!("[coros] sleep overview unavailable: {e}"),
        }

        // Sleep HRV: "2026-10-13:\n  HRV Avg: 75 ms" (assessment section).
        match self
            .call_tool_text(&service, "querySleepHrv", days_arg.clone())
            .await
        {
            Ok(text) => {
                // Only the assessment section has daily averages; stop before
                // the raw time-series section.
                let assess = text
                    .split("Sleep HRV Time Series")
                    .next()
                    .unwrap_or(&text)
                    .to_string();
                let lines: Vec<&str> = assess.lines().collect();
                for (i, line) in lines.iter().enumerate() {
                    let Some(d) = Self::find_date(line) else {
                        continue;
                    };
                    if !line.trim_start().starts_with("20") {
                        continue;
                    }
                    // Look ahead a couple of lines for "HRV Avg: NN ms".
                    for l in lines[i..(i + 3).min(lines.len())].iter() {
                        if let Some(v) = Self::num_after(l, "HRV Avg:") {
                            days.entry(d)
                                .or_insert_with(|| Self::empty_health(d))
                                .hrv_avg = Some(v);
                            break;
                        }
                    }
                }
            }
            Err(e) => eprintln!("[coros] sleep HRV unavailable: {e}"),
        }

        // Resting HR: "2026-10-13: 43 bpm".
        match self
            .call_tool_text(&service, "queryRestingHeartRate", days_arg)
            .await
        {
            Ok(text) => {
                for line in text.lines() {
                    let t = line.trim_start();
                    if !t.starts_with("20") {
                        continue;
                    }
                    if let (Some(d), Some(v)) = (Self::find_date(t), Self::num_after(line, ": ")) {
                        let h = days.entry(d).or_insert_with(|| Self::empty_health(d));
                        if h.resting_hr.is_none() {
                            h.resting_hr = Some(v as i64);
                        }
                    }
                }
            }
            Err(e) => eprintln!("[coros] resting HR unavailable: {e}"),
        }

        // Daily overview also carries a header "Resting HR: NN bpm" — use it
        // only as a fallback when the trend tool gave nothing.
        let count = days.len() as u32;
        for (_, h) in days {
            store.upsert_health(ProviderKind::Coros, &h)?;
        }
        Ok(count)
    }

    async fn sync_fitness(&self, store: &Store) -> Result<()> {
        let service = self.client().await?;
        let today = Utc::now().date_naive();

        // "VO2max: 62 / Threshold Pace: 3:53 /km / 5 km Prediction: 18:34 …"
        match self
            .call_tool_text(&service, "queryFitnessAssessmentOverview", json!({}))
            .await
        {
            Ok(text) => {
                let f = FitnessAssessment {
                    date: today,
                    vo2max: Self::num_after(&text, "VO2max:"),
                    threshold_sec_km: Self::clock_after(&text, "Threshold Pace:"),
                    predict_5k_s: Self::clock_after_line(&text, "5 km Prediction:")
                        .map(|v| v as i64),
                    predict_10k_s: Self::clock_after_line(&text, "10 km Prediction:")
                        .map(|v| v as i64),
                    predict_half_s: Self::clock_after_line(&text, "Half Marathon Prediction:")
                        .map(|v| v as i64),
                    predict_marathon_s: Self::clock_after_line(&text, "Marathon Prediction:")
                        .map(|v| v as i64),
                };
                store.upsert_fitness(ProviderKind::Coros, &f)?;
            }
            Err(e) => eprintln!("[coros] fitness overview unavailable: {e}"),
        }

        // Daily blocks: "2026-10-13\nComment: Performance\nShort-Term Load: 43
        // Long-Term Load: 62\nLoad Ratio: 0.69".
        match self
            .call_tool_text(
                &service,
                "queryTrainingLoadAssessment",
                json!({ "days": 14 }),
            )
            .await
        {
            Ok(text) => {
                for block in text.split("\n\n") {
                    let first = block.lines().next().unwrap_or("").trim();
                    let Some(d) = Self::find_date(first) else {
                        continue;
                    };
                    if !first.starts_with("20") {
                        continue;
                    }
                    let l = TrainingLoad {
                        date: d,
                        short_term: Self::num_after(block, "Short-Term Load:"),
                        long_term: Self::num_after(block, "Long-Term Load:"),
                        ratio: Self::num_after(block, "Load Ratio:"),
                        comment: block
                            .lines()
                            .find_map(|x| x.strip_prefix("Comment:"))
                            .map(|s| s.trim().to_string()),
                    };
                    store.upsert_training_load(ProviderKind::Coros, &l)?;
                }
            }
            Err(e) => eprintln!("[coros] training load unavailable: {e}"),
        }
        Ok(())
    }

    async fn sync_tracks(&self, store: &Store, budget: u32) -> Result<u32> {
        let service = self.client().await?;
        // Activities with no track and no ledger entry, newest first. The store
        // query carries the provider sport code we need back; deriving it from
        // the sport label instead loses every sport the label mapper folds away
        // (hike, cycling, strength) and those tracks never download.
        let budget = budget.clamp(1, 10) as i64;
        let missing = store.activities_missing_track(budget)?;
        if missing.is_empty() {
            return Ok(0);
        }
        // FIT download works per activity: labelId + sportType, and the file
        // arrives as a base64 blob in an embedded resource block. The tool
        // allows max 10 files per 24h, so the budget above is capped at 10.
        let mut downloaded = 0u32;
        for (label_id, sport, code) in missing {
            let code = match code.or_else(|| Self::sport_code(&sport)) {
                Some(c) => c,
                None => {
                    // No code means we cannot ask for this file at all; say so
                    // instead of skipping it silently on every sync.
                    eprintln!("[coros] no sport code for {label_id} ({sport}) — track unavailable");
                    continue;
                }
            };
            let args = json!({ "labelId": label_id, "sportType": code });
            let result = match self
                .call_tool_raw(&service, "downloadActivityFitFiles", args)
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("[coros] fit download {label_id} failed: {e}");
                    continue;
                }
            };
            for (uri, bytes) in Self::blobs_of(&result) {
                // uri like "coros://activity-fit-files/<labelId>.fit".
                let rid = uri
                    .rsplit('/')
                    .next()
                    .and_then(|f| f.strip_suffix(".fit"))
                    .unwrap_or(&label_id)
                    .to_string();
                match self.ingest_fit(store, &rid, &bytes) {
                    Ok(true) => downloaded += 1,
                    Ok(false) => {}
                    Err(e) => {
                        // The blob can come back truncated; forgetting the
                        // ledger entry lets a later sync retry it.
                        eprintln!("[coros] fit decode {rid} failed: {e}");
                        let _ = store.forget_fit_downloaded(ProviderKind::Coros, &rid);
                    }
                }
            }
        }
        Ok(downloaded)
    }

    async fn push_plan(
        &self,
        plan: &TrainingPlan,
        workouts: &[PlanWorkout],
        progress: super::Progress<'_>,
    ) -> Result<Option<String>> {
        let report = |stage: &str, pct: u8| {
            if let Some(p) = progress {
                p(stage, pct);
            }
        };
        report("Connecting to COROS", 5);
        let service = self.client().await?;
        // A plan that was pushed before must be updated in place: COROS keeps
        // every plan it is handed, so blindly calling createTrainingPlan
        // again would leave the stale schedule on the watch beside the new
        // one. updateTrainingPlan is day-granular, so the update path also
        // rest-clears days that no longer hold a workout. The lookup runs
        // even without a stored pushed id: a push whose report did not name
        // the new plan id leaves status=pushed with pushed_id=NULL, and
        // skipping the lookup then would duplicate the plan on the watch on
        // every re-push. find_execution_plan takes any editable execution
        // row when there is no id to prefer.
        report("Looking up the plan on COROS", 15);
        if let Some(exec) = self
            .find_execution_plan(&service, plan.pushed_id.as_deref())
            .await?
        {
            report("Reading its schedule", 30);
            match self
                .update_plan(&service, &exec, plan, workouts, progress)
                .await
            {
                Ok(()) => return Ok(Some(exec)),
                Err(e) => {
                    // The execution record may have been quit or finished
                    // in the COROS app since we pushed; a fresh create is
                    // the only way to get the plan back onto the watch.
                    // Clear the old schedule first so the watch never
                    // holds two Runalytics plans at once.
                    eprintln!("[coros] plan update failed, pushing as new: {e}");
                    if let Err(e) = self.cancel_existing(&service, &exec, progress).await {
                        eprintln!("[coros] clearing the stale plan failed: {e}");
                    }
                }
            }
        }
        report("Uploading the plan", 60);
        self.create_plan(&service, plan, workouts).await
    }

    async fn cancel_plan(
        &self,
        pushed_id: Option<&str>,
        progress: super::Progress<'_>,
    ) -> Result<()> {
        let report = |stage: &str, pct: u8| {
            if let Some(p) = progress {
                p(stage, pct);
            }
        };
        report("Connecting to COROS", 5);
        let service = self.client().await?;
        report("Looking up the plan on COROS", 20);
        let Some(exec) = self.find_execution_plan(&service, pushed_id).await? else {
            report("Already cleared", 100);
            return Ok(()); // nothing running on COROS — already gone
        };
        self.cancel_existing(&service, &exec, progress).await
    }

    async fn list_api_tools(&self) -> Result<Vec<String>> {
        let service = self.client().await?;
        let result = service
            .list_tools(None)
            .await
            .map_err(|e| AppError::Provider(format!("tools/list: {e}")))?;
        let names = result
            .tools
            .iter()
            .map(|t| t.name.to_string())
            .collect::<Vec<_>>();
        Ok(names)
    }
}

/// What an existing COROS plan looks like: dayNo 0 is the plan start date,
/// `max_day` is its last day, `course_days` are the dayNos holding a course.
struct PlanState {
    start: NaiveDate,
    max_day: i64,
    course_days: Vec<i64>,
}

impl CorosProvider {
    /// The first half of push_plan: a brand-new createTrainingPlan call.
    /// createTrainingPlan takes natural Mon-Sun weeks: planInfo +
    /// phaseInfo.periodization + a courseList keyed by dayNo (0 = plan
    /// start date). planStartDate must fall within [today, today+14] and
    /// totalWeeks is 4..=16, so the pushed window starts today at the
    /// earliest and workouts outside the window are simply not pushed.
    async fn create_plan(
        &self,
        service: &RunningService<RoleClient, ()>,
        plan: &TrainingPlan,
        workouts: &[PlanWorkout],
    ) -> Result<Option<String>> {
        let today = Utc::now().date_naive();
        let first = workouts.iter().map(|w| w.date).min().unwrap_or(today);
        let start = first.max(today).min(today + Duration::days(14));
        let offset = start.weekday().num_days_from_monday() as i64;
        let day_no = |d: NaiveDate| (d - start).num_days();

        let mut courses: Vec<Value> = workouts
            .iter()
            .filter(|w| w.kind != WorkoutKind::Rest && w.distance_m.unwrap_or(0.0) > 0.0)
            .filter(|w| day_no(w.date) >= 0)
            .map(|w| {
                let details = crate::plan::session::details_for_plan(w, plan);
                let sections: Vec<Value> = details.steps.iter().map(Self::course_section).collect();
                json!({
                    "dayNo": day_no(w.date),
                    "sportType": 1, // running
                    "courseName": w.title,
                    "courseDescription": details.purpose,
                    "sections": sections,
                })
            })
            .collect();
        if courses.is_empty() {
            return Err(AppError::Other(
                "nothing to push: the plan has no upcoming workouts".into(),
            ));
        }
        // Plan length: whole weeks up to the last pushed course, clamped to
        // the API's 4..=16; courses beyond a 16-week window are dropped.
        let max_day = |cs: &Vec<Value>| {
            cs.iter()
                .filter_map(|c| c["dayNo"].as_i64())
                .max()
                .unwrap_or(0)
        };
        let total_weeks = ((max_day(&courses) + offset) / 7 + 1).clamp(4, 16);
        let max_valid = total_weeks * 7 - offset - 1;
        courses.retain(|c| c["dayNo"].as_i64().unwrap_or(0) <= max_valid);
        let total_weeks = ((max_day(&courses) + offset) / 7 + 1).clamp(4, total_weeks);
        let final_sunday = start + Duration::days(total_weeks * 7 - offset - 1);
        // Periodization must sum exactly to totalWeeks: base phases then a
        // one-week race phase (minimum 4-week plan => 3 + 1).
        let base_weeks = total_weeks - 1;
        // COROS has one phase vocabulary, and phaseType 5 is its race phase.
        // Only a build that ends at an effort gets one: a maintenance block is
        // all base phase, and claiming a race week the runner does not have
        // would show them a start line on the watch that exists nowhere.
        let has_race_phase = plan.kind.has_race();
        let milestone = match plan.kind {
            PlanKind::Race => format!(
                " to {} on {}",
                plan.distance.label(),
                plan.target_date.format("%d %b %Y")
            ),
            PlanKind::Improvement => format!(
                ", closing with a {} time trial on {}",
                plan.distance.label(),
                plan.target_date.format("%d %b %Y")
            ),
            PlanKind::Maintenance => String::new(),
        };
        let overview = format!(
            "{} {} plan{} built by Runalytics from your watch data.",
            plan.kind.label(),
            plan.distance.label(),
            milestone
        );
        let periodization = if has_race_phase {
            json!([
                { "phaseType": 2, "startDate": Self::yyyymmdd(start),
                  "endDate": Self::yyyymmdd(final_sunday - Duration::days(7)),
                  "durationWeeks": base_weeks },
                { "phaseType": 5, "startDate": Self::yyyymmdd(final_sunday - Duration::days(6)),
                  "endDate": Self::yyyymmdd(final_sunday), "durationWeeks": 1 },
            ])
        } else {
            json!([
                { "phaseType": 2, "startDate": Self::yyyymmdd(start),
                  "endDate": Self::yyyymmdd(final_sunday),
                  "durationWeeks": total_weeks },
            ])
        };
        let args = json!({
            "planInfo": {
                "planName": format!("Runalytics {} {}", plan.kind.label(), plan.distance.label()),
                "planOverview": overview,
                "planStartDate": Self::yyyymmdd(start),
                "totalWeeks": total_weeks,
            },
            "phaseInfo": {
                "planRationale": overview,
                "periodization": periodization,
            },
            "courseList": courses,
        });
        let text = self
            .call_tool_text(service, "createTrainingPlan", args)
            .await?;
        Self::ensure_ok("createTrainingPlan", &text)?;
        // The report mentions the created plan id in prose; grab the token
        // after any "planId"/"Plan ID" label.
        let id = Self::token_after(&text, "planId")
            .or_else(|| Self::token_after(&text, "Plan ID"))
            .or_else(|| Self::token_after(&text, "plan id"));
        Ok(id)
    }

    /// COROS reports API-side rejections (validation errors, plan-state
    /// conflicts) as ordinary text with isError=false — only the prose says
    /// the call failed. Every mutating call must run its report through this.
    fn ensure_ok(name: &str, text: &str) -> Result<()> {
        if text.contains("COROS API error") || text.contains("validation error") {
            return Err(AppError::Provider(format!(
                "{name}: {}",
                text.chars().take(300).collect::<String>()
            )));
        }
        Ok(())
    }

    /// Locate the in-progress execution record for a pushed plan — the row
    /// queryTrainingPlanLibrary marks recordRole=execution and editable.
    /// Prefer the stored pushed id; otherwise take any editable execution
    /// row, since the id we stored may be stale after the runner restarted
    /// the plan in the COROS app. None means nothing is running on COROS.
    async fn find_execution_plan(
        &self,
        service: &RunningService<RoleClient, ()>,
        preferred: Option<&str>,
    ) -> Result<Option<String>> {
        let text = self
            .call_tool_text(service, "queryTrainingPlanLibrary", json!({}))
            .await?;
        let mut fallback: Option<String> = None;
        for block in text.split("\n\n") {
            if !block.contains("Record role: execution") || !block.contains("Editable via MCP: yes")
            {
                continue;
            }
            let Some(id) = Self::token_after(block, "Plan ID:") else {
                continue;
            };
            if preferred == Some(id.as_str()) {
                return Ok(Some(id));
            }
            if fallback.is_none() {
                fallback = Some(id);
            }
        }
        Ok(fallback)
    }

    /// Read an existing plan's span and course days from
    /// queryTrainingPlanDetails. The tool windows at most 28 days per call,
    /// so page across the whole plan; the header ("Dates: a to b") on the
    /// first call gives the full span regardless of the default window.
    async fn fetch_plan_state(
        &self,
        service: &RunningService<RoleClient, ()>,
        plan_id: &str,
    ) -> Result<PlanState> {
        let header = self
            .call_tool_text(
                service,
                "queryTrainingPlanDetails",
                json!({ "planId": plan_id }),
            )
            .await?;
        let (start, end) = Self::parse_plan_dates(&header)?;
        let mut course_days: Vec<i64> = Vec::new();
        let mut from = start;
        while from <= end {
            let to = (from + Duration::days(27)).min(end);
            let text = self
                .call_tool_text(
                    service,
                    "queryTrainingPlanDetails",
                    json!({
                        "planId": plan_id,
                        "startDay": Self::yyyymmdd(from),
                        "endDay": Self::yyyymmdd(to),
                    }),
                )
                .await?;
            course_days.extend(Self::parse_course_days(&text));
            from = to + Duration::days(1);
        }
        course_days.sort_unstable();
        course_days.dedup();
        Ok(PlanState {
            start,
            max_day: (end - start).num_days(),
            course_days,
        })
    }

    /// Day-granular update of a plan already on COROS: replace every day the
    /// new workout set covers, rest-clear days that used to hold a course
    /// but no longer do, and leave the final week's stale courses alone when
    /// the new set has nothing there (COROS refuses to empty the final
    /// week). dayNo stays anchored to the plan's own start date.
    async fn update_plan(
        &self,
        service: &RunningService<RoleClient, ()>,
        plan_id: &str,
        plan: &TrainingPlan,
        workouts: &[PlanWorkout],
        progress: super::Progress<'_>,
    ) -> Result<()> {
        let report = |stage: &str, pct: u8| {
            if let Some(p) = progress {
                p(stage, pct);
            }
        };
        report("Reading the plan from COROS", 35);
        let state = self.fetch_plan_state(service, plan_id).await?;
        let day_no = |d: NaiveDate| (d - state.start).num_days();
        let new_courses: Vec<Value> = workouts
            .iter()
            .filter(|w| w.kind != WorkoutKind::Rest && w.distance_m.unwrap_or(0.0) > 0.0)
            .filter(|w| {
                let n = day_no(w.date);
                n >= 0 && n <= state.max_day
            })
            .map(|w| {
                let details = crate::plan::session::details_for_plan(w, plan);
                let sections: Vec<Value> = details.steps.iter().map(Self::course_section).collect();
                json!({
                    "dayNo": day_no(w.date),
                    "sportType": 1, // running
                    "courseName": w.title,
                    "courseDescription": details.purpose,
                    "sections": sections,
                })
            })
            .collect();
        if new_courses.is_empty() {
            return Err(AppError::Other(
                "nothing to push: the plan has no upcoming workouts".into(),
            ));
        }
        let new_days: Vec<i64> = new_courses
            .iter()
            .filter_map(|c| c["dayNo"].as_i64())
            .collect();
        let keeps_final_week = new_days.iter().any(|d| *d > state.max_day - 7);
        let mut courses = new_courses;
        for d in &state.course_days {
            if new_days.contains(d) {
                continue;
            }
            if !keeps_final_week && *d > state.max_day - 7 {
                continue;
            }
            courses.push(json!({
                "dayNo": d,
                "sportType": 4,
                "courseName": "Rest",
                "courseDescription": "Recovery day",
                "sections": [],
            }));
        }
        let text = self
            .call_tool_text(
                service,
                "updateTrainingPlan",
                json!({ "planInfo": { "planId": plan_id }, "courseList": courses }),
            )
            .await?;
        report("Sending updates to COROS", 85);
        Self::ensure_ok("updateTrainingPlan", &text)
    }

    /// Empty an existing COROS plan: every course day becomes rest except
    /// the last course in the final week, which is replaced by a free-mode
    /// marker saying the plan was cancelled. COROS cannot delete a plan and
    /// refuses to empty the final week, so this is the closest thing to
    /// quitting it that MCP offers. Verified against the live API 2026-10-20.
    async fn cancel_existing(
        &self,
        service: &RunningService<RoleClient, ()>,
        plan_id: &str,
        progress: super::Progress<'_>,
    ) -> Result<()> {
        let report = |stage: &str, pct: u8| {
            if let Some(p) = progress {
                p(stage, pct);
            }
        };
        report("Reading the plan from COROS", 35);
        let state = self.fetch_plan_state(service, plan_id).await?;
        if state.course_days.is_empty() {
            report("Already cleared", 100);
            return Ok(());
        }
        let final_week_start = state.max_day - 6;
        let marker = state
            .course_days
            .iter()
            .rev()
            .find(|d| **d >= final_week_start)
            .copied();
        let courses: Vec<Value> = state
            .course_days
            .iter()
            .map(|d| {
                if Some(*d) == marker {
                    json!({
                        "dayNo": d,
                        "sportType": 1,
                        "courseName": "Plan cancelled",
                        "courseDescription": "This Runalytics plan was cancelled. Nothing is scheduled.",
                        "sections": [{ "sectionType": 2, "targetType": 4 }],
                    })
                } else {
                    json!({
                        "dayNo": d,
                        "sportType": 4,
                        "courseName": "Rest",
                        "courseDescription": "Recovery day",
                        "sections": [],
                    })
                }
            })
            .collect();
        let text = self
            .call_tool_text(
                service,
                "updateTrainingPlan",
                json!({ "planInfo": { "planId": plan_id }, "courseList": courses }),
            )
            .await?;
        report("Clearing the schedule on COROS", 75);
        Self::ensure_ok("updateTrainingPlan", &text)?;
        report("Plan cleared", 100);
        Ok(())
    }

    /// The "Dates: 2026-10-26 to 2026-11-22" header of a plan-details report.
    /// Parse each side with a strict date parser rather than find_date, and
    /// trim first: chrono's %Y skips leading whitespace, so an untrimmed
    /// 10-char window " 2026-10-2" happily parses as 2026-10-02, and
    /// find_date's 8-digit fallback reads "2026-102" as 2026-10-02 too.
    fn parse_plan_dates(text: &str) -> Result<(NaiveDate, NaiveDate)> {
        fn date_in(s: &str) -> Option<NaiveDate> {
            let s = s.trim();
            if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                return Some(d);
            }
            let chars: Vec<char> = s.chars().collect();
            for i in 0..=chars.len().saturating_sub(10) {
                let w: String = chars[i..i + 10].iter().collect();
                if w.starts_with(|c: char| c.is_ascii_digit()) {
                    if let Ok(d) = NaiveDate::parse_from_str(&w, "%Y-%m-%d") {
                        return Some(d);
                    }
                }
            }
            None
        }
        for line in text.lines() {
            let Some(rest) = line.trim_start().strip_prefix("Dates:") else {
                continue;
            };
            let Some((a, b)) = rest.split_once(" to ") else {
                continue;
            };
            if let (Some(start), Some(end)) = (date_in(a), date_in(b)) {
                if start <= end {
                    return Ok((start, end));
                }
            }
        }
        Err(AppError::Provider(
            "could not read the plan dates from the COROS report".into(),
        ))
    }

    /// Day numbers that hold a course in a plan-details report: a "Day N -
    /// date" line followed by a "Course ..." line ("Course JSON:" repeats
    /// are ignored because the day is consumed on the first hit).
    fn parse_course_days(text: &str) -> Vec<i64> {
        let mut out = Vec::new();
        let mut day: Option<i64> = None;
        for line in text.lines() {
            let line = line.trim_start();
            if let Some(rest) = line.strip_prefix("Day ") {
                day = rest.split('-').next().and_then(|n| n.trim().parse().ok());
            } else if line.starts_with("Course ") {
                if let Some(d) = day.take() {
                    out.push(d);
                }
            }
        }
        out
    }
}

impl CorosProvider {
    /// Date as the integer yyyyMMdd the COROS tools expect.
    fn yyyymmdd(d: NaiveDate) -> i64 {
        d.format("%Y%m%d").to_string().parse().unwrap_or(0)
    }

    /// Absolute pace intensity for a section: COROS takes intensityType 2
    /// with an integer s/km range of 120..=1499; a step without a pace band
    /// gets no intensity at all (fields absent, never 0). An open-ended step
    /// runs from its floor to the slowest value COROS accepts.
    fn pace_intensity(
        low: Option<i64>,
        high: Option<i64>,
        open_ended: bool,
    ) -> serde_json::Map<String, Value> {
        let mut o = serde_json::Map::new();
        let high = if open_ended { low.map(|_| 1499) } else { high };
        if let (Some(low), Some(high)) = (low, high) {
            let low = low.clamp(120, 1499);
            let high = high.clamp(120, 1499).max(low);
            o.insert("intensityType".into(), json!(2));
            o.insert("intensityValueStart".into(), json!(low));
            o.insert("intensityValueEnd".into(), json!(high));
        }
        o
    }

    /// One plan step -> one COROS section: warm-up/cooldown keep their
    /// section types, a repeated set becomes an intervalGroup whose members
    /// carry the targets (containers must omit them), everything else is a
    /// plain training section with a distance target.
    fn course_section(step: &crate::plan::session::WorkoutStep) -> Value {
        let metres = |m: f64| (m - 1e-6).round().max(1.0) as i64;
        if step.repeats > 1 {
            let mut work = Self::pace_intensity(step.pace_lo_s_km, step.pace_hi_s_km, false);
            work.insert("sectionType".into(), json!(2));
            work.insert("targetType".into(), json!(1)); // distance, metres
            work.insert("targetValue".into(), json!(metres(step.distance_m)));
            let mut recovery = Self::pace_intensity(
                step.recovery_pace_lo_s_km,
                step.recovery_pace_hi_s_km,
                false,
            );
            recovery.insert("sectionType".into(), json!(3));
            recovery.insert("targetType".into(), json!(1));
            recovery.insert("targetValue".into(), json!(metres(step.recovery_m)));
            return json!({
                "intervalGroup": true,
                "repeats": step.repeats.min(20),
                "sets": [Value::Object(work), Value::Object(recovery)],
            });
        }
        let section_type = if step.label.starts_with("Warm-up") {
            1
        } else if step.label.starts_with("Cool-down") {
            4
        } else {
            2
        };
        let mut o = Self::pace_intensity(step.pace_lo_s_km, step.pace_hi_s_km, step.open_ended);
        o.insert("sectionType".into(), json!(section_type));
        o.insert("targetType".into(), json!(1));
        o.insert("targetValue".into(), json!(metres(step.distance_m)));
        Value::Object(o)
    }

    fn ingest_fit(&self, store: &Store, record_id: &str, bytes: &[u8]) -> Result<bool> {
        if record_id.is_empty() {
            return Ok(false);
        }
        let mut decoded = fit::decode_fit(record_id, bytes)?;
        decoded.record_id = record_id.to_string();
        let inserted = store.upsert_activity(ProviderKind::Coros, &decoded)?;
        store.mark_fit_downloaded(ProviderKind::Coros, record_id)?;
        Ok(inserted.is_some() || !decoded.points.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::CorosProvider;

    /// Verbatim shapes from the live COROS report (2026-10-17).
    const RECORDS: &str = "Sport Records — 2026-09-17 to 2026-10-17 (3 records)\n\
========================\n\n\
1. Outdoor Run — 2026-10-14\n\
   Location: Lisbon Run\n\
   Time Window: startTimestamp=1791999167 | endTimestamp=1792000998\n\
   Duration: 29:00 | Distance: 7.01 km\n\
   Average Pace: 4:08 /km | Avg HR: 152 bpm | Calories: 508 kcal\n\
   LabelId: 481036619086856995 | SportType: 100\n\n\
2. Strength — 2026-10-08\n\
   Location: Strength\n\
   Time Window: startTimestamp=1791442491 | endTimestamp=1791445995\n\
   Duration: 58:24 | Sets: 7\n\
   Avg HR: 86 bpm | Calories: 347 kcal\n\
   LabelId: 480887225899319300 | SportType: 402\n\n\
3. Hike — 2026-09-19\n\
   Time Window: startTimestamp=1790493264 | endTimestamp=1790493270\n\
   Duration: 4:52:59 | Distance: 10.56 km\n\
   Average Speed: 2.2 km/h | Avg HR: 106 bpm\n\
   LabelId: 480453805714997355 | SportType: 104\n";

    /// A strength session has no `Distance:` line at all. Requiring distance
    /// *and* duration dropped every one of them, so the dashboard showed no
    /// strength work even though COROS reported it.
    #[test]
    fn parse_keeps_strength_record_without_distance() {
        let recs = CorosProvider::parse_sport_records(RECORDS);
        assert_eq!(recs.len(), 3, "run + strength + hike all parse");
        let strength = recs
            .iter()
            .find(|r| r.sport == "strength")
            .expect("strength record must survive parsing");
        assert_eq!(strength.distance_m, 0.0);
        assert_eq!(strength.duration_s, 58.0 * 60.0 + 24.0);
        assert_eq!(strength.avg_hr, Some(86));
    }

    /// The FIT download tool wants `sportType` back, and the sport label is
    /// lossy — without the raw code, hike/cycling/strength tracks can never be
    /// fetched, which is why the heatmap was missing them.
    #[test]
    fn parse_captures_provider_sport_code() {
        let recs = CorosProvider::parse_sport_records(RECORDS);
        let code_of = |sport: &str| {
            recs.iter()
                .find(|r| r.sport == sport)
                .and_then(|r| r.provider_sport_code)
        };
        assert_eq!(code_of("run"), Some(100));
        assert_eq!(code_of("strength"), Some(402));
        assert_eq!(code_of("hike"), Some(104));
    }

    /// Distance is in km in the report and metres in the store.
    #[test]
    fn parse_converts_distance_units() {
        let recs = CorosProvider::parse_sport_records(RECORDS);
        let run = recs.iter().find(|r| r.sport == "run").expect("run");
        assert!(
            (run.distance_m - 7010.0).abs() < 1.0,
            "7.01 km must land as ~7010 m, got {}",
            run.distance_m
        );
    }

    /// "Marathon Prediction:" is a suffix of "Half Marathon Prediction:", so a
    /// plain substring search gave the marathon field the half-marathon time.
    #[test]
    fn marathon_prediction_is_not_the_half_marathon() {
        let text = "VO2max: 62\nThreshold Pace: 3:53 /km\n5 km Prediction: 18:34\n\
                    10 km Prediction: 38:17\nHalf Marathon Prediction: 1:24:09\n\
                    Marathon Prediction: 2:53:57\n";
        let half =
            CorosProvider::clock_after_line(text, "Half Marathon Prediction:").expect("half");
        let full = CorosProvider::clock_after_line(text, "Marathon Prediction:").expect("marathon");
        assert_eq!(half, 3600.0 + 24.0 * 60.0 + 9.0);
        assert_eq!(full, 2.0 * 3600.0 + 53.0 * 60.0 + 57.0);
        assert_ne!(half, full, "the two must not collapse to the same value");
        // The old, substring-based lookup is what produced the bug — kept here
        // so the difference in behaviour is explicit.
        assert_eq!(
            CorosProvider::clock_after(text, "Marathon Prediction:"),
            Some(half)
        );
    }

    /// Every label `sport_label` can emit must map back to a COROS code, or
    /// that sport's FIT file is unreachable.
    #[test]
    fn sport_label_round_trips_to_a_code() {
        for label in ["run", "ride", "swim", "strength", "hike", "cycling", "jog"] {
            assert!(
                CorosProvider::sport_code(label).is_some(),
                "sport {label:?} must have a COROS sport code"
            );
        }
        assert_eq!(CorosProvider::sport_code("run"), Some(100));
        assert_eq!(CorosProvider::sport_code("hike"), Some(104));
        assert_eq!(CorosProvider::sport_code("ride"), Some(200));
        assert_eq!(CorosProvider::sport_code("swim"), Some(300));
        assert_eq!(CorosProvider::sport_code("strength"), Some(402));
    }

    /// Verbatim plan-details header + day list (live probe plan, 2026-10-20).
    const PLAN_DETAIL: &str = "Training Plan Detail\n\
========================\n\
Plan ID: 481164258269380939\n\
Name: Runalytics sync probe\n\
Status: In progress\n\
Type: mcp\n\
Dates: 2026-10-26 to 2026-11-22\n\
Weeks: 4\n\
Editable via MCP: yes\n\n\
Days (dayNo 0-6)\n\
------------------------\n\
Day 0 - 2026-10-26\n\
Rest day\n\
Day 1 - 2026-10-27\n\
Course 1 [Not started, score 0.0]\n\
Course: Easy 8km\n\
Course JSON: {\"dayNo\":1,\"courseName\":\"Easy 8km\"}\n\
Day 2 - 2026-10-28\n\
Rest day\n\
Day 4 - 2026-10-30\n\
Course 1 [Not started, score 0.0]\n\
Course: Tempo 8km\n\
Course JSON: {\"dayNo\":4}\n";

    /// The update path anchors every dayNo on the plan's own start date, so
    /// the "Dates: a to b" header must parse into exactly that span.
    #[test]
    fn parse_plan_dates_reads_the_header_span() {
        let (start, end) = CorosProvider::parse_plan_dates(PLAN_DETAIL).expect("dates");
        assert_eq!(start.to_string(), "2026-10-26");
        assert_eq!(end.to_string(), "2026-11-22");
    }

    /// Days holding a course are the ones an update must consider clearing;
    /// "Course JSON:" lines must not double-count their day.
    #[test]
    fn parse_course_days_lists_each_course_day_once() {
        assert_eq!(CorosProvider::parse_course_days(PLAN_DETAIL), vec![1, 4]);
    }

    /// COROS reports validation failures as ordinary text (isError=false);
    /// only the prose says the call failed, so mutating calls must grep it.
    #[test]
    fn ensure_ok_rejects_api_error_prose() {
        assert!(CorosProvider::ensure_ok(
            "updateTrainingPlan",
            "Training plan updated successfully."
        )
        .is_ok());
        let err = CorosProvider::ensure_ok(
            "updateTrainingPlan",
            "COROS API error: The final displayed week must contain at least one workout.",
        )
        .expect_err("must fail");
        assert!(err.to_string().contains("final displayed week"));
    }
}

/// Tiny base64 decoder (standard alphabet, padding tolerant) — avoids pulling
/// another dependency for one call site.
fn base64_decode(s: &str) -> Result<Vec<u8>> {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    for c in s.bytes() {
        let c = match c {
            b'\n' | b'\r' | b' ' | b'=' => continue,
            b'-' => b'+',
            b'_' => b'/',
            x => x,
        };
        let Some(pos) = T.iter().position(|&x| x == c) else {
            return Err(AppError::Provider("bad base64".into()));
        };
        acc = (acc << 6) | pos as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Ok(out)
}

// Global handle so `connect()` (a plain async fn, not a command) can reach
// the opener plugin. Set once in setup(). AppHandle is Send + Sync.
pub(crate) static APP_HANDLE: std::sync::OnceLock<tauri::AppHandle> = std::sync::OnceLock::new();

pub(crate) fn set_app_handle(handle: tauri::AppHandle) {
    let _ = APP_HANDLE.set(handle);
}
