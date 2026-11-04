use crate::{
    calendar,
    error::{AppError, Result},
    secure_store,
    state::AppState,
    store::Store,
};
use aws_lc_rs::digest::{digest, SHA256};
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::time::Duration;
use tauri::{AppHandle, State};

#[cfg(target_os = "ios")]
mod ios_auth;

const SERVICE: &str = "com.fradone.runalytics.strava";
const API: &str = "https://www.strava.com/api/v3";
#[cfg(any(target_os = "ios", test))]
const IOS_REDIRECT: &str = "com.fradone.runalytics://localhost/strava/callback";
static OPERATION: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
static AUTH_CANCEL: OnceLock<tokio::sync::Notify> = OnceLock::new();

fn operation() -> &'static tokio::sync::Mutex<()> {
    OPERATION.get_or_init(|| tokio::sync::Mutex::new(()))
}

#[tauri::command]
pub fn strava_cancel_connect() {
    AUTH_CANCEL
        .get_or_init(tokio::sync::Notify::new)
        .notify_waiters();
}

fn failure(message: &str) -> AppError {
    AppError::Other(message.into())
}

#[derive(Serialize, Deserialize)]
struct Credentials {
    client_id: String,
    client_secret: String,
    tokens: Option<Tokens>,
}

#[derive(Serialize, Deserialize)]
struct Tokens {
    access_token: String,
    refresh_token: String,
    expires_at: i64,
    athlete_id: i64,
}

fn credentials() -> Result<Option<Credentials>> {
    match secure_store::entry(SERVICE, "credentials")
        .map_err(|_| failure("Strava Keychain is unavailable."))?
        .get_password()
    {
        Ok(value) => serde_json::from_str(&value).map(Some).map_err(|_| {
            failure("Saved Strava credentials are unreadable. Configure Strava again.")
        }),
        Err(keyring_core::Error::NoEntry) => Ok(None),
        Err(_) => Err(failure(
            "Strava Keychain is unavailable. Unlock your device and try again.",
        )),
    }
}

fn save_credentials(value: &Credentials) -> Result<()> {
    secure_store::entry(SERVICE, "credentials")
        .map_err(|_| failure("Strava Keychain is unavailable."))?
        .set_password(&serde_json::to_string(value)?)
        .map_err(|_| failure("Could not save Strava credentials in Keychain."))
}

pub(crate) fn is_configured() -> Result<bool> {
    Ok(credentials()?.is_some())
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    configured: bool,
    client_id: String,
    connected: bool,
    athlete_id: Option<i64>,
    enabled: bool,
    enabled_at: Option<i64>,
    updated: usize,
    already_present: usize,
    pending: usize,
    needs_review: usize,
    last_message: Option<String>,
    last_attempt_at: Option<i64>,
}

fn status(store: &Store) -> Result<Status> {
    let Some(config) = credentials()? else {
        return Ok(Status::default());
    };
    let output = Status {
        configured: true,
        client_id: config.client_id,
        connected: config.tokens.is_some(),
        athlete_id: config.tokens.as_ref().map(|tokens| tokens.athlete_id),
        ..Status::default()
    };
    account_status(store, output)
}

fn account_status(store: &Store, mut output: Status) -> Result<Status> {
    if let Some(athlete) = output.athlete_id {
        let connection = store.conn.lock().unwrap();
        if let Some((enabled, enabled_at, message, attempted)) = connection.query_row(
            "SELECT enabled,enabled_at,last_message,last_attempt_at FROM strava_accounts WHERE athlete_id=?1",
            [athlete], |row| Ok((row.get::<_, bool>(0)?, row.get::<_, Option<i64>>(1)?, row.get::<_, Option<String>>(2)?, row.get::<_, Option<i64>>(3)?)),
        ).optional()? {
            output.enabled = enabled;
            output.enabled_at = enabled_at;
            output.last_message = message;
            output.last_attempt_at = attempted;
        }
        let mut statement = connection.prepare(
            "SELECT status,count(*) FROM strava_exports WHERE athlete_id=?1 GROUP BY status",
        )?;
        for entry in statement.query_map([athlete], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)? as usize))
        })? {
            let (state, count) = entry?;
            match state.as_str() {
                "updated" => output.updated += count,
                "already_present" => output.already_present += count,
                "needs_review" | "auth_required" => output.needs_review += count,
                _ => output.pending += count,
            }
        }
    }
    Ok(output)
}

#[tauri::command]
pub fn strava_status(state: State<'_, AppState>) -> Result<Status> {
    status(&state.store)
}

#[tauri::command]
pub async fn strava_configure(
    state: State<'_, AppState>,
    client_id: String,
    client_secret: Option<String>,
) -> Result<Status> {
    let _guard = operation().lock().await;
    let client_id = client_id.trim().to_string();
    if client_id.is_empty() || !client_id.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(failure(
            "Enter the numeric Client ID from your Strava API settings.",
        ));
    }
    let existing = credentials()?;
    let supplied = client_secret.filter(|secret| !secret.trim().is_empty());
    let unchanged = existing
        .as_ref()
        .is_some_and(|saved| saved.client_id == client_id)
        && supplied.as_ref().is_none_or(|secret| {
            existing
                .as_ref()
                .is_some_and(|saved| saved.client_secret == secret.trim())
        });
    let secret = supplied
        .map(|value| value.trim().to_string())
        .or_else(|| {
            existing
                .as_ref()
                .filter(|saved| saved.client_id == client_id)
                .map(|saved| saved.client_secret.clone())
        })
        .ok_or_else(|| failure("Enter your Strava Client Secret for this Client ID."))?;
    if !unchanged {
        if let Some(tokens) = existing.as_ref().and_then(|saved| saved.tokens.as_ref()) {
            state.store.conn.lock().unwrap().execute(
                "UPDATE strava_accounts SET enabled=0 WHERE athlete_id=?1",
                [tokens.athlete_id],
            )?;
        }
    }
    let tokens = if unchanged {
        existing.and_then(|saved| saved.tokens)
    } else {
        None
    };
    save_credentials(&Credentials {
        client_id,
        client_secret: secret,
        tokens,
    })?;
    status(&state.store)
}

#[tauri::command]
pub async fn strava_set_enabled(state: State<'_, AppState>, enabled: bool) -> Result<Status> {
    let _guard = operation().lock().await;
    let config = credentials()?.ok_or_else(|| failure("Configure Strava first."))?;
    let tokens = config
        .tokens
        .ok_or_else(|| failure("Connect Strava first."))?;
    state.store.conn.lock().unwrap().execute(
        "UPDATE strava_accounts SET enabled=?2,enabled_at=CASE WHEN ?2=1 THEN COALESCE(enabled_at,?3) ELSE enabled_at END WHERE athlete_id=?1",
        params![tokens.athlete_id, enabled, Utc::now().timestamp()],
    )?;
    status(&state.store)
}

#[tauri::command]
pub async fn strava_disconnect(state: State<'_, AppState>, forget: bool) -> Result<Status> {
    let _guard = operation().lock().await;
    if let Some(mut config) = credentials()? {
        if let Some(tokens) = config.tokens.take() {
            state.store.conn.lock().unwrap().execute(
                "UPDATE strava_accounts SET enabled=0 WHERE athlete_id=?1",
                [tokens.athlete_id],
            )?;
        }
        if forget {
            secure_store::entry(SERVICE, "credentials")
                .map_err(|_| failure("Strava Keychain is unavailable."))?
                .delete_credential()
                .map_err(|_| failure("Could not remove Strava credentials from Keychain."))?;
        } else {
            save_credentials(&config)?;
        }
    }
    status(&state.store)
}

fn http() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| failure("Could not initialize the Strava connection."))
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_at: i64,
    athlete: Option<RemoteAthlete>,
}

async fn token_request(config: &Credentials, grant: &str, value: &str) -> Result<TokenResponse> {
    let field = if grant == "authorization_code" {
        "code"
    } else {
        "refresh_token"
    };
    let response = http()?
        .post("https://www.strava.com/oauth/token")
        .form(&[
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("grant_type", grant),
            (field, value),
        ])
        .send()
        .await
        .map_err(|_| {
            failure("Strava token request failed. Check your connection and reconnect.")
        })?;
    if !response.status().is_success() {
        return Err(failure(
            "Strava authorization expired or API credentials were rejected. Reconnect Strava.",
        ));
    }
    response
        .json()
        .await
        .map_err(|_| failure("Strava returned an invalid authorization response."))
}

fn authorize_url(client_id: &str, redirect: &str, state: &str) -> Result<tauri::Url> {
    let endpoint = if cfg!(target_os = "ios") {
        "https://www.strava.com/oauth/mobile/authorize"
    } else {
        "https://www.strava.com/oauth/authorize"
    };
    let mut url = tauri::Url::parse(endpoint)
        .map_err(|_| failure("Invalid Strava authorization address."))?;
    url.query_pairs_mut().extend_pairs([
        ("client_id", client_id),
        ("redirect_uri", redirect),
        ("response_type", "code"),
        ("scope", "activity:read_all,activity:write"),
        ("approval_prompt", "force"),
        ("state", state),
    ]);
    Ok(url)
}

fn callback_code(callback: &tauri::Url, redirect: &str, expected_state: &str) -> Result<String> {
    let expected = tauri::Url::parse(redirect)
        .map_err(|_| failure("Invalid Strava callback configuration."))?;
    if callback.scheme() != expected.scheme()
        || callback.host_str() != expected.host_str()
        || callback.port() != expected.port()
        || callback.path() != expected.path()
    {
        return Err(failure("Strava returned an unexpected callback address."));
    }
    let pairs: std::collections::HashMap<_, _> = callback.query_pairs().into_owned().collect();
    if pairs.get("state").map(String::as_str) != Some(expected_state) {
        return Err(failure("Strava sign-in state did not match. Start again."));
    }
    if pairs.contains_key("error") {
        return Err(failure("Strava sign-in was declined."));
    }
    let scopes: Vec<_> = pairs
        .get("scope")
        .map(|scope| scope.split([',', ' ']).collect())
        .unwrap_or_default();
    if !scopes.contains(&"activity:read_all") || !scopes.contains(&"activity:write") {
        return Err(failure(
            "Allow activity read and write permissions to connect Strava.",
        ));
    }
    pairs
        .get("code")
        .filter(|code| !code.is_empty())
        .cloned()
        .ok_or_else(|| failure("Strava did not return an authorization code."))
}

#[cfg(not(target_os = "ios"))]
async fn authorize(app: &AppHandle, client_id: &str, state: &str) -> Result<(tauri::Url, String)> {
    use tauri_plugin_opener::OpenerExt;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let redirect = format!(
        "http://127.0.0.1:{}/strava/callback",
        listener.local_addr()?.port()
    );
    let url = authorize_url(client_id, &redirect, state)?;
    app.opener()
        .open_url(url.as_str(), None::<&str>)
        .map_err(|_| failure("Could not open Strava sign-in."))?;
    let callback = tokio::time::timeout(Duration::from_secs(300), async {
        loop {
            let (mut socket, _) = listener.accept().await?;
            let mut bytes = vec![0; 8192];
            let count = tokio::time::timeout(Duration::from_secs(5), socket.read(&mut bytes)).await
                .map_err(|_| failure("Strava callback timed out."))??;
            let request = String::from_utf8_lossy(&bytes[..count]);
            let target = request.lines().next().and_then(|line| line.strip_prefix("GET ")).and_then(|line| line.split_whitespace().next()).unwrap_or("");
            let parsed = tauri::Url::parse(&format!("http://127.0.0.1:{}{target}", listener.local_addr()?.port())).map_err(|_| failure("Invalid Strava callback."))?;
            if parsed.path() != "/strava/callback" {
                let _ = socket.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await;
                continue;
            }
            let body = "Return to Runalytics to finish connecting Strava.";
            let reply = format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            let _ = socket.write_all(reply.as_bytes()).await;
            return Ok::<_, AppError>(parsed);
        }
    }).await.map_err(|_| failure("Strava sign-in timed out. Try again."))??;
    Ok((callback, redirect))
}

#[cfg(target_os = "ios")]
async fn authorize(app: &AppHandle, client_id: &str, state: &str) -> Result<(tauri::Url, String)> {
    let redirect = IOS_REDIRECT.to_string();
    let url = authorize_url(client_id, &redirect, state)?;
    let callback = ios_auth::authorize(app, url.as_str()).await?;
    Ok((
        tauri::Url::parse(&callback).map_err(|_| failure("Invalid Strava callback."))?,
        redirect,
    ))
}

#[tauri::command]
pub async fn strava_connect(app: AppHandle, state: State<'_, AppState>) -> Result<Status> {
    let _guard = operation().lock().await;
    let mut config =
        credentials()?.ok_or_else(|| failure("Save your Strava API credentials first."))?;
    let nonce = uuid::Uuid::new_v4().to_string();
    let authorization = tokio::select! {
        result = authorize(&app, &config.client_id, &nonce) => result,
        _ = AUTH_CANCEL.get_or_init(tokio::sync::Notify::new).notified() => Err(failure("Strava sign-in was cancelled.")),
    };
    #[cfg(target_os = "ios")]
    ios_auth::cancel(&app)?;
    let (callback, redirect) = authorization?;
    let code = callback_code(&callback, &redirect, &nonce)?;
    let response = token_request(&config, "authorization_code", &code).await?;
    let athlete_id = response
        .athlete
        .ok_or_else(|| failure("Strava did not identify the connected athlete."))?
        .id;
    if let Some(previous) = config.tokens.as_ref() {
        state.store.conn.lock().unwrap().execute(
            "UPDATE strava_accounts SET enabled=0 WHERE athlete_id=?1",
            [previous.athlete_id],
        )?;
    }
    state.store.conn.lock().unwrap().execute("INSERT INTO strava_accounts(athlete_id,client_id) VALUES(?1,?2) ON CONFLICT(athlete_id) DO UPDATE SET client_id=excluded.client_id,enabled=0,last_message=NULL", params![athlete_id, config.client_id])?;
    config.tokens = Some(Tokens {
        access_token: response.access_token,
        refresh_token: response.refresh_token,
        expires_at: response.expires_at,
        athlete_id,
    });
    save_credentials(&config)?;
    state.store.conn.lock().unwrap().execute("UPDATE strava_exports SET status='pending',next_attempt_at=0 WHERE athlete_id=?1 AND status='auth_required'", [athlete_id])?;
    status(&state.store)
}

struct Job {
    key: String,
    start: i64,
    duration: f64,
    distance: f64,
    payload: String,
    reference: String,
    target: Option<i64>,
    attempts: u32,
}

fn enqueue(store: &Store, athlete: i64, cutoff: i64, now: i64) -> Result<()> {
    let activities = store.list_activities(-1)?;
    let events = calendar::events(store)?;
    let connection = store.conn.lock().unwrap();
    for event in events
        .into_iter()
        .filter(|event| event.workout_id.is_some() && event.activity_id.is_some())
    {
        let Some(activity) = activities
            .iter()
            .find(|activity| Some(activity.id) == event.activity_id)
        else {
            continue;
        };
        if activity.start_time.timestamp() < cutoff {
            continue;
        }
        let provider = serde_json::to_value(activity.provider)?
            .as_str()
            .unwrap_or_default()
            .to_string();
        let key = format!("{provider}:{}", activity.provider_record_id);
        connection.execute("INSERT OR IGNORE INTO strava_exports(athlete_id,source_key,workout_id,start_ts,duration_s,distance_m,payload,marker,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?9)",
            params![athlete, key, event.workout_id, activity.start_time.timestamp(), activity.duration_s, activity.distance_m, event.notes, marker(athlete, &key), now])?;
    }
    Ok(())
}

fn finish_job(
    store: &Store,
    athlete: i64,
    job: &Job,
    state: &str,
    message: &str,
    target: Option<i64>,
    retry_at: i64,
) -> Result<()> {
    store.conn.lock().unwrap().execute("UPDATE strava_exports SET status=?3,message=?4,target_id=?5,next_attempt_at=?6,attempts=attempts+1,updated_at=?7,payload=CASE WHEN ?3 IN ('updated','already_present') THEN '' ELSE payload END WHERE athlete_id=?1 AND source_key=?2",
        params![athlete, job.key, state, message, target, retry_at, Utc::now().timestamp()])?;
    Ok(())
}

struct ApiFailure {
    status: u16,
    retry_at: i64,
}

struct Client {
    http: reqwest::Client,
    base_url: String,
    token: String,
    remaining: usize,
    defer_until: i64,
}

impl Client {
    async fn request<T: serde::de::DeserializeOwned>(
        &mut self,
        method: reqwest::Method,
        endpoint: &str,
        body: Option<&str>,
    ) -> std::result::Result<T, ApiFailure> {
        let now = Utc::now().timestamp();
        if self.remaining == 0 || self.defer_until > now {
            return Err(ApiFailure {
                status: 429,
                retry_at: self.defer_until.max(now + 60),
            });
        }
        self.remaining -= 1;
        let mut request = self
            .http
            .request(method, format!("{}{endpoint}", self.base_url))
            .bearer_auth(&self.token);
        if let Some(description) = body {
            request = request.json(&serde_json::json!({"description":description}));
        }
        let response = request.send().await.map_err(|_| ApiFailure {
            status: 0,
            retry_at: now + 300,
        })?;
        for (limit_header, usage_header) in [
            ("x-ratelimit-limit", "x-ratelimit-usage"),
            ("x-readratelimit-limit", "x-readratelimit-usage"),
        ] {
            let parse = |name| {
                response
                    .headers()
                    .get(name)
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| {
                        let mut values = value
                            .split(',')
                            .filter_map(|item| item.trim().parse::<i64>().ok());
                        Some((values.next()?, values.next()?))
                    })
            };
            if let (Some(limit), Some(usage)) = (parse(limit_header), parse(usage_header)) {
                if usage.0 >= limit.0 {
                    self.defer_until = self.defer_until.max((now / 900 + 1) * 900 + 1);
                }
                if usage.1 >= limit.1 {
                    self.defer_until = self.defer_until.max((now / 86400 + 1) * 86400 + 1);
                }
            }
        }
        let status = response.status().as_u16();
        if !response.status().is_success() {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(0)
                .clamp(0, 86400);
            if status == 429 {
                self.defer_until = self
                    .defer_until
                    .max((now / 900 + 1) * 900 + 1)
                    .max(now + retry_after);
            }
            return Err(ApiFailure {
                status,
                retry_at: self
                    .defer_until
                    .max(now + retry_after)
                    .max(if status == 429 {
                        (now / 900 + 1) * 900 + 1
                    } else {
                        now + 300
                    }),
            });
        }
        response.json().await.map_err(|_| ApiFailure {
            status: 0,
            retry_at: now + 300,
        })
    }
}

async fn process_job(client: &mut Client, store: &Store, athlete: i64, job: &Job) -> Result<()> {
    let now = Utc::now().timestamp();
    let mut selected_target = job.target;
    let result: std::result::Result<(String, String, Option<i64>), ApiFailure> = async {
        let target = if let Some(target) = job.target { target } else {
            let mut candidates = Vec::new();
            let mut complete = false;
            for page in 1..=3 {
                let entries: Vec<RemoteActivity> = client.request(reqwest::Method::GET, &format!("/athlete/activities?after={}&before={}&per_page=100&page={page}", job.start - 121, job.start + 121), None).await?;
                complete = entries.len() < 100;
                candidates.extend(entries.into_iter().filter(|entry| matches_activity(job.start, job.duration, job.distance, entry)));
                if complete { break; }
            }
            if !complete || candidates.len() > 1 { return Ok(("needs_review".into(), "Multiple possible Strava activities; no description changed.".into(), None)); }
            let Some(candidate) = candidates.first() else { return Ok(("waiting".into(), "Waiting for the watch activity to arrive in Strava.".into(), None)); };
            let bound: bool = store.conn.lock().unwrap().query_row("SELECT EXISTS(SELECT 1 FROM strava_exports WHERE athlete_id=?1 AND target_id=?2 AND source_key<>?3)", params![athlete, candidate.id, job.key], |row| row.get(0)).map_err(|_| ApiFailure { status: 0, retry_at: now + 300 })?;
            if bound { return Ok(("needs_review".into(), "This Strava activity is already linked to another recording.".into(), None)); }
            candidate.id
        };
        selected_target = Some(target);
        let remote: RemoteActivity = client.request(reqwest::Method::GET, &format!("/activities/{target}"), None).await?;
        if remote.id != target || remote.athlete.as_ref().map(|owner| owner.id) != Some(athlete) || !matches_activity(job.start, job.duration, job.distance, &remote) {
            return Ok(("needs_review".into(), "Strava activity identity could not be verified.".into(), Some(target)));
        }
        match description_change(remote.description.as_deref().unwrap_or(""), &job.payload, &job.reference) {
            DescriptionChange::Present => Ok(("already_present".into(), "Description was already copied, possibly by another device.".into(), Some(target))),
            DescriptionChange::Review => Ok(("needs_review".into(), "An existing Runalytics description needs review; nothing changed.".into(), Some(target))),
            DescriptionChange::Append(description) => {
                let response: RemoteActivity = client.request(reqwest::Method::PUT, &format!("/activities/{target}"), Some(&description)).await?;
                if response.id == target && response.description.as_deref() == Some(&description) {
                    Ok(("updated".into(), "Paired description copied to Strava.".into(), Some(target)))
                } else { Err(ApiFailure { status: 0, retry_at: now + 300 }) }
            }
        }
    }.await;
    match result {
        Ok((state, message, target)) => {
            finish_job(store, athlete, job, &state, &message, target, now + 300)
        }
        Err(error) => {
            let (state, message) = match error.status {
                401 | 403 => ("auth_required", "Strava authorization needs attention. Reconnect and allow activity read/write permissions."),
                400 | 422 => ("needs_review", "Strava rejected the description. No automatic truncation was attempted."),
                404 => ("waiting", "The activity is not available in Strava yet."),
                429 => ("retry", "Strava request limit reached. Pending descriptions will retry after the cooldown."),
                _ => ("retry", "Strava could not confirm the update. The next attempt checks the description before writing."),
            };
            let backoff = 60 * 2_i64.pow(job.attempts.min(10));
            finish_job(
                store,
                athlete,
                job,
                state,
                message,
                selected_target,
                error.retry_at.max(now + backoff),
            )?;
            if error.status == 401 || error.status == 403 || error.status == 429 {
                client.remaining = 0;
            }
            Ok(())
        }
    }
}

pub async fn sync(store: &Store) -> Result<String> {
    let Ok(_guard) = operation().try_lock() else {
        return Ok(
            "Strava connection is busy. Pending descriptions will retry on the next sync.".into(),
        );
    };
    let result = sync_inner(store).await;
    if result.is_err() {
        if let Ok(Some(config)) = credentials() {
            if let Some(tokens) = config.tokens {
                store.conn.lock().unwrap().execute("UPDATE strava_accounts SET last_message='Strava could not finish. Check your connection or reconnect Strava.',last_attempt_at=?2 WHERE athlete_id=?1", params![tokens.athlete_id,Utc::now().timestamp()])?;
            }
        }
    }
    result
}

async fn sync_inner(store: &Store) -> Result<String> {
    housekeeping(store, Utc::now().timestamp())?;
    let Some(mut config) = credentials()? else {
        return Ok(String::new());
    };
    let Some(tokens) = config.tokens.as_ref() else {
        return Ok("Strava is not connected. Connect in Settings to update descriptions.".into());
    };
    let athlete = tokens.athlete_id;
    let cutoff: Option<i64> = store
        .conn
        .lock()
        .unwrap()
        .query_row(
            "SELECT enabled_at FROM strava_accounts WHERE athlete_id=?1 AND enabled=1",
            [athlete],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    let Some(cutoff) = cutoff else {
        return Ok("Strava description updates are paused.".into());
    };
    let now = Utc::now().timestamp();
    let queue_failed = enqueue(store, athlete, cutoff, now).is_err();
    let jobs = {
        let connection = store.conn.lock().unwrap();
        let mut statement = connection.prepare("SELECT source_key,start_ts,duration_s,distance_m,payload,marker,target_id,attempts FROM strava_exports WHERE athlete_id=?1 AND status IN ('pending','waiting','retry') AND next_attempt_at<=?2 ORDER BY next_attempt_at,start_ts LIMIT 3")?;
        let jobs = statement
            .query_map(params![athlete, now], |row| {
                Ok(Job {
                    key: row.get(0)?,
                    start: row.get(1)?,
                    duration: row.get(2)?,
                    distance: row.get(3)?,
                    payload: row.get(4)?,
                    reference: row.get(5)?,
                    target: row.get(6)?,
                    attempts: row.get(7)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        jobs
    };
    if jobs.is_empty() {
        return if queue_failed {
            Err(failure(
                "Could not prepare paired descriptions. Try another watch sync.",
            ))
        } else {
            record_sync_result(store, athlete, now)
        };
    }
    if tokens.expires_at <= now + 120 {
        let response = token_request(&config, "refresh_token", &tokens.refresh_token).await?;
        config.tokens = Some(Tokens {
            access_token: response.access_token,
            refresh_token: response.refresh_token,
            expires_at: response.expires_at,
            athlete_id: athlete,
        });
        save_credentials(&config)?;
    }
    let mut client = Client {
        http: http()?,
        base_url: API.into(),
        token: config.tokens.as_ref().unwrap().access_token.clone(),
        remaining: 12,
        defer_until: 0,
    };
    let deadline = tokio::time::Instant::now() + Duration::from_secs(40);
    for job in &jobs {
        if client.remaining == 0 || tokio::time::Instant::now() >= deadline {
            break;
        }
        match tokio::time::timeout_at(deadline, process_job(&mut client, store, athlete, job)).await
        {
            Ok(result) => result?,
            Err(_) => {
                finish_job(store, athlete, job, "retry", "Strava sync reached its time budget. The next attempt checks remote text first.", job.target, now + 300)?;
                break;
            }
        }
    }
    if client.defer_until > now {
        store.conn.lock().unwrap().execute("UPDATE strava_exports SET next_attempt_at=max(next_attempt_at,?2) WHERE athlete_id=?1 AND status IN ('pending','waiting','retry')", params![athlete,client.defer_until])?;
    }
    record_sync_result(store, athlete, now)
}

fn record_sync_result(store: &Store, athlete: i64, now: i64) -> Result<String> {
    let output = account_status(store, Status { athlete_id: Some(athlete), ..Status::default() })?;
    let mut message = format!(
        "Strava: {} copied, {} already present, {} pending, {} need attention.",
        output.updated, output.already_present, output.pending, output.needs_review
    );
    let connection = store.conn.lock().unwrap();
    let outstanding: Option<(String, i64)> = connection.query_row(
        "SELECT message,next_attempt_at FROM strava_exports WHERE athlete_id=?1 AND status IN ('pending','waiting','retry','auth_required','needs_review') ORDER BY CASE status WHEN 'auth_required' THEN 0 WHEN 'needs_review' THEN 1 ELSE 2 END,next_attempt_at,start_ts LIMIT 1",
        [athlete], |row| Ok((row.get(0)?, row.get(1)?)),
    ).optional()?;
    if let Some((detail, retry_at)) = outstanding {
        if !detail.is_empty() { message.push_str(&format!(" {detail}")); }
        if output.needs_review == 0 && retry_at > now {
            message.push_str(&format!(" Next retry in {} min.", (retry_at - now + 59) / 60));
        }
    }
    connection.execute(
        "UPDATE strava_accounts SET last_message=?2,last_attempt_at=?3 WHERE athlete_id=?1",
        params![athlete, message, now],
    )?;
    Ok(message)
}

#[tauri::command]
pub async fn strava_retry(state: State<'_, AppState>) -> Result<Status> {
    sync(&state.store).await?;
    status(&state.store)
}

fn housekeeping(store: &Store, now: i64) -> Result<()> {
    let connection = store.conn.lock().unwrap();
    connection.execute("UPDATE strava_exports SET payload='',status='needs_review',message='Pending description expired after 90 days; no Strava activity was changed.',updated_at=?1 WHERE payload<>'' AND retention_expires_at<=?1", [now])?;
    connection.execute("DELETE FROM strava_exports WHERE NOT EXISTS(SELECT 1 FROM activities WHERE strava_exports.source_key=activities.provider || ':' || activities.provider_record_id)", [])?;
    Ok(())
}

pub(crate) const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS strava_accounts (
    athlete_id INTEGER PRIMARY KEY,
    client_id TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 0 CHECK(enabled IN (0,1)),
    enabled_at INTEGER,
    last_message TEXT,
    last_attempt_at INTEGER,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE TABLE IF NOT EXISTS strava_exports (
    athlete_id INTEGER NOT NULL REFERENCES strava_accounts(athlete_id),
    source_key TEXT NOT NULL,
    workout_id TEXT NOT NULL,
    start_ts INTEGER NOT NULL,
    duration_s REAL NOT NULL,
    distance_m REAL NOT NULL,
    payload TEXT NOT NULL,
    marker TEXT NOT NULL,
    target_id INTEGER,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN
      ('pending','waiting','updated','already_present','needs_review','retry','auth_required')),
    attempts INTEGER NOT NULL DEFAULT 0,
    next_attempt_at INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    retention_expires_at INTEGER NOT NULL DEFAULT (unixepoch()+7776000),
    message TEXT,
    PRIMARY KEY(athlete_id,source_key),
    UNIQUE(athlete_id,target_id),
    CHECK(status NOT IN ('updated','already_present') OR target_id IS NOT NULL)
);
"#;

#[derive(Clone, Deserialize)]
struct RemoteActivity {
    id: i64,
    start_date: DateTime<Utc>,
    elapsed_time: f64,
    #[serde(default)]
    moving_time: Option<f64>,
    distance: f64,
    sport_type: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    athlete: Option<RemoteAthlete>,
}

#[derive(Clone, Deserialize)]
struct RemoteAthlete {
    id: i64,
}

fn marker(athlete: i64, source_key: &str) -> String {
    let hash = digest(&SHA256, format!("{athlete}:{source_key}").as_bytes());
    let encoded: String = hash
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!("[Runalytics sync v1:{encoded}]")
}

fn matches_activity(start: i64, duration: f64, distance: f64, remote: &RemoteActivity) -> bool {
    matches!(
        remote.sport_type.as_str(),
        "Run" | "TrailRun" | "VirtualRun"
    ) && duration.is_finite()
        && duration > 0.0
        && distance.is_finite()
        && distance > 0.0
        && (remote.start_date.timestamp() - start).abs() <= 120
        && (remote.elapsed_time - duration).abs().min(
            remote.moving_time.map_or(f64::INFINITY, |moving| (moving - duration).abs()),
        ) <= (duration * 0.02).max(60.0)
        && (remote.distance - distance).abs() <= (distance * 0.02).max(100.0)
}

#[derive(Debug, PartialEq)]
enum DescriptionChange {
    Present,
    Append(String),
    Review,
}

fn description_change(existing: &str, payload: &str, reference: &str) -> DescriptionChange {
    let normalized = existing.replace("\r\n", "\n");
    let block = payload.replace("\r\n", "\n");
    if block.lines().filter(|line| line.trim().starts_with("Made with Runalytics") || line.trim() == "Runalytics").count() > 1 {
        return DescriptionChange::Review;
    }
    if normalized.lines().any(|line| line.trim() == reference)
        || format!("\n\n{}\n\n", normalized.trim()).contains(&format!("\n\n{}\n\n", block.trim()))
    {
        return DescriptionChange::Present;
    }
    if normalized.contains("[Runalytics sync ")
        || normalized.contains("Runalytics\n\n")
        || normalized.contains("Made with Runalytics")
    {
        return DescriptionChange::Review;
    }
    let separator = if existing.is_empty() { "" } else { "\n\n" };
    DescriptionChange::Append(format!("{existing}{separator}{payload}\n\n{reference}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Store, Job) {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let job = Job {
            key: "coros:one".into(),
            start: 1_793_664_000,
            duration: 1800.0,
            distance: 5000.0,
            payload: "Made with Runalytics\n\nIntervals\n\nResults".into(),
            reference: marker(42, "coros:one"),
            target: None,
            attempts: 0,
        };
        store.conn.lock().unwrap().execute("INSERT INTO strava_accounts(athlete_id,client_id,enabled,enabled_at) VALUES(42,'123',1,1)", []).unwrap();
        store.conn.lock().unwrap().execute("INSERT INTO strava_exports(athlete_id,source_key,workout_id,start_ts,duration_s,distance_m,payload,marker,created_at,updated_at) VALUES(42,?1,'workout',?2,?3,?4,?5,?6,1,1)", params![job.key,job.start,job.duration,job.distance,job.payload,job.reference]).unwrap();
        (store, job)
    }

    fn remote(job: &Job, description: &str) -> serde_json::Value {
        serde_json::json!({"id":99,"start_date":DateTime::from_timestamp(job.start,0).unwrap(),"elapsed_time":1800,"distance":5000,"sport_type":"Run","description":description,"athlete":{"id":42}})
    }

    fn outcome(store: &Store) -> (String, Option<i64>, String, i64) {
        store.conn.lock().unwrap().query_row("SELECT status,target_id,payload,next_attempt_at FROM strava_exports WHERE athlete_id=42 AND source_key='coros:one'", [], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).unwrap()
    }

    #[test]
    fn deferred_queue_reports_reason_and_retry_without_staying_silent() {
        let (store, job) = fixture();
        let now = Utc::now().timestamp();
        finish_job(&store, 42, &job, "waiting", "Waiting for the watch activity to arrive in Strava.", None, now + 300).unwrap();
        let message = record_sync_result(&store, 42, now).unwrap();
        assert!(message.contains("1 pending"));
        assert!(message.contains("Waiting for the watch activity to arrive in Strava."));
        assert!(message.contains("Next retry in 5 min."));
        let output = account_status(&store, Status { athlete_id: Some(42), ..Status::default() }).unwrap();
        assert_eq!(output.last_message.as_deref(), Some(message.as_str()));
        assert_eq!(output.last_attempt_at, Some(now));
        assert_eq!(outcome(&store).0, "waiting");
        assert_eq!(outcome(&store).3, now + 300);
    }

    #[test]
    fn authorization_queue_reports_attention_instead_of_a_retry_promise() {
        let (store, job) = fixture();
        let now = Utc::now().timestamp();
        finish_job(&store, 42, &job, "auth_required", "Reconnect Strava and allow activity read/write permissions.", None, now + 300).unwrap();
        let message = record_sync_result(&store, 42, now).unwrap();
        assert!(message.contains("0 pending, 1 need attention"));
        assert!(message.contains("Reconnect Strava"));
        assert!(!message.contains("Next retry"));
    }

    async fn fake_api(
        responses: Vec<(String, u16, serde_json::Value)>,
    ) -> (Client, tokio::task::JoinHandle<Vec<serde_json::Value>>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let mut bodies = Vec::new();
            for (expected, status, body) in responses {
                let (mut socket, _) =
                    tokio::time::timeout(Duration::from_secs(5), listener.accept())
                        .await
                        .unwrap()
                        .unwrap();
                let mut request = Vec::new();
                let header_end = loop {
                    let mut chunk = [0; 4096];
                    let count = socket.read(&mut chunk).await.unwrap();
                    assert!(count > 0);
                    request.extend_from_slice(&chunk[..count]);
                    if let Some(end) = request.windows(4).position(|part| part == b"\r\n\r\n") {
                        break end + 4;
                    }
                };
                let headers = String::from_utf8_lossy(&request[..header_end]);
                assert!(
                    headers.starts_with(&expected),
                    "unexpected request method or path"
                );
                let length = headers
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|value| value.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                while request.len() < header_end + length {
                    let mut chunk = [0; 4096];
                    let count = socket.read(&mut chunk).await.unwrap();
                    assert!(count > 0);
                    request.extend_from_slice(&chunk[..count]);
                }
                if length > 0 {
                    bodies.push(
                        serde_json::from_slice(&request[header_end..header_end + length]).unwrap(),
                    );
                }
                if status == 0 {
                    continue;
                }
                let body = body.to_string();
                let reply = format!("HTTP/1.1 {status} Response\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());
                socket.write_all(reply.as_bytes()).await.unwrap();
            }
            bodies
        });
        (
            Client {
                http: http().unwrap(),
                base_url,
                token: "synthetic".into(),
                remaining: 12,
                defer_until: 0,
            },
            server,
        )
    }

    #[tokio::test]
    async fn export_preserves_existing_notes_and_only_writes_description() {
        let (store, job) = fixture();
        let DescriptionChange::Append(description) =
            description_change("My notes", &job.payload, &job.reference)
        else {
            panic!("expected append")
        };
        let (mut client, server) = fake_api(vec![
            (
                "GET /athlete/activities?".into(),
                200,
                serde_json::json!([remote(&job, "")]),
            ),
            ("GET /activities/99 ".into(), 200, remote(&job, "My notes")),
            (
                "PUT /activities/99 ".into(),
                200,
                remote(&job, &description),
            ),
        ])
        .await;
        process_job(&mut client, &store, 42, &job).await.unwrap();
        let bodies = server.await.unwrap();
        assert_eq!(bodies, vec![serde_json::json!({"description":description})]);
        let (state, target, payload, _) = outcome(&store);
        assert_eq!(
            (state.as_str(), target, payload.as_str()),
            ("updated", Some(99), "")
        );
    }

    #[tokio::test]
    async fn another_devices_marker_and_clipboard_copy_are_adopted_without_put() {
        for marker_copy in [true, false] {
            let (store, job) = fixture();
            let description = if marker_copy {
                format!("Other device text\n\n{}", job.reference)
            } else {
                format!("My notes\n\n{}", job.payload)
            };
            let (mut client, server) = fake_api(vec![
                (
                    "GET /athlete/activities?".into(),
                    200,
                    serde_json::json!([remote(&job, "")]),
                ),
                (
                    "GET /activities/99 ".into(),
                    200,
                    remote(&job, &description),
                ),
            ])
            .await;
            process_job(&mut client, &store, 42, &job).await.unwrap();
            assert!(server.await.unwrap().is_empty());
            assert_eq!(outcome(&store).0, "already_present");
        }
    }

    #[tokio::test]
    async fn uncertain_success_keeps_target_and_retry_adopts_remote_content() {
        let (store, mut job) = fixture();
        let DescriptionChange::Append(description) =
            description_change("My notes", &job.payload, &job.reference)
        else {
            panic!("expected append")
        };
        let (mut client, server) = fake_api(vec![
            (
                "GET /athlete/activities?".into(),
                200,
                serde_json::json!([remote(&job, "")]),
            ),
            ("GET /activities/99 ".into(), 200, remote(&job, "My notes")),
            ("PUT /activities/99 ".into(), 0, serde_json::Value::Null),
        ])
        .await;
        process_job(&mut client, &store, 42, &job).await.unwrap();
        assert_eq!(server.await.unwrap().len(), 1);
        assert_eq!(
            (outcome(&store).0.as_str(), outcome(&store).1),
            ("retry", Some(99))
        );
        job.target = Some(99);
        let (mut client, server) = fake_api(vec![(
            "GET /activities/99 ".into(),
            200,
            remote(&job, &description),
        )])
        .await;
        process_job(&mut client, &store, 42, &job).await.unwrap();
        assert!(server.await.unwrap().is_empty());
        assert_eq!(outcome(&store).0, "already_present");
    }

    #[tokio::test]
    async fn delayed_and_ambiguous_activity_matches_never_write() {
        for ambiguous in [false, true] {
            let (store, job) = fixture();
            let mut duplicate = remote(&job, "");
            duplicate["id"] = serde_json::json!(100);
            let candidates = if ambiguous {
                serde_json::json!([remote(&job, ""), duplicate])
            } else {
                serde_json::json!([])
            };
            let (mut client, server) =
                fake_api(vec![("GET /athlete/activities?".into(), 200, candidates)]).await;
            process_job(&mut client, &store, 42, &job).await.unwrap();
            assert!(server.await.unwrap().is_empty());
            assert_eq!(
                outcome(&store).0,
                if ambiguous { "needs_review" } else { "waiting" }
            );
        }
    }

    #[tokio::test]
    async fn permission_and_rate_limit_failures_pause_without_writing() {
        for status in [401, 403, 429] {
            let (store, job) = fixture();
            let (mut client, server) = fake_api(vec![(
                "GET /athlete/activities?".into(),
                status,
                serde_json::json!({}),
            )])
            .await;
            process_job(&mut client, &store, 42, &job).await.unwrap();
            assert!(server.await.unwrap().is_empty());
            assert_eq!(client.remaining, 0);
            assert_eq!(
                outcome(&store).0,
                if status == 429 {
                    "retry"
                } else {
                    "auth_required"
                }
            );
            assert!(outcome(&store).3 > Utc::now().timestamp());
            if status == 429 {
                assert!(client.defer_until > Utc::now().timestamp());
            }
        }
    }

    #[test]
    fn ios_oauth_uses_whitelisted_host_and_validates_the_custom_callback() {
        let url = authorize_url("12345", IOS_REDIRECT, "nonce").unwrap();
        let pairs: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        let redirect = tauri::Url::parse(&pairs["redirect_uri"]).unwrap();
        assert_eq!(redirect.host_str(), Some("localhost"));
        assert_eq!(redirect.scheme(), "com.fradone.runalytics");
        assert_eq!(redirect.path(), "/strava/callback");
        let callback = tauri::Url::parse(&format!(
            "{IOS_REDIRECT}?state=nonce&code=synthetic&scope=activity:read_all,activity:write"
        )).unwrap();
        assert_eq!(callback_code(&callback, IOS_REDIRECT, "nonce").unwrap(), "synthetic");
        assert!(callback_code(&callback, IOS_REDIRECT, "wrong").is_err());
        let mut unexpected = callback;
        unexpected.set_host(Some("strava")).unwrap();
        assert!(callback_code(&unexpected, IOS_REDIRECT, "nonce").is_err());
    }

    #[test]
    fn oauth_requires_exact_callback_state_and_both_granted_scopes() {
        let redirect = "http://127.0.0.1:12345/strava/callback";
        let callback = tauri::Url::parse(&format!(
            "{redirect}?state=nonce&code=synthetic&scope=activity:read_all,activity:write"
        ))
        .unwrap();
        assert_eq!(
            callback_code(&callback, redirect, "nonce").unwrap(),
            "synthetic"
        );
        assert!(callback_code(&callback, redirect, "wrong").is_err());
        assert!(
            callback_code(&callback, "http://127.0.0.1:54321/strava/callback", "nonce").is_err()
        );
        let refused = tauri::Url::parse(&format!(
            "{redirect}?state=nonce&code=synthetic&scope=activity:read_all"
        ))
        .unwrap();
        assert!(callback_code(&refused, redirect, "nonce").is_err());
    }

    #[test]
    fn queue_only_copies_new_paired_activities_and_freezes_calendar_text() {
        let (store, _) = fixture();
        let cutoff = Utc::now().timestamp() - 3600;
        {
            let connection = store.conn.lock().unwrap();
            connection
                .execute("DELETE FROM strava_exports", [])
                .unwrap();
            for (record, start, paired) in [
                ("old", cutoff - 1, true),
                ("new", cutoff + 1, true),
                ("unpaired", cutoff + 10, false),
            ] {
                connection.execute("INSERT INTO activities(provider,provider_record_id,sport,start_ts,duration_s,distance_m) VALUES('coros',?1,'running',?2,1800,5000)",params![record,start]).unwrap();
                if paired {
                    connection.execute("INSERT INTO calendar_completions(workout_id,activity_id,original_notes) VALUES(?1,?2,'Made with Runalytics\n\nIntervals\n\nPlan:\n5 km')",params![record,connection.last_insert_rowid()]).unwrap();
                }
            }
        }
        let expected = calendar::events(&store)
            .unwrap()
            .into_iter()
            .find(|event| event.workout_id.as_deref() == Some("new"))
            .unwrap()
            .notes;
        enqueue(&store, 42, cutoff, Utc::now().timestamp()).unwrap();
        {
            let connection = store.conn.lock().unwrap();
            let (count, payload): (u32, String) = connection
                .query_row("SELECT count(*),payload FROM strava_exports", [], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })
                .unwrap();
            assert_eq!(count, 1);
            assert_eq!(payload, expected);
            connection
                .execute(
                    "UPDATE calendar_completions SET original_notes='Changed after queueing'",
                    [],
                )
                .unwrap();
        }
        enqueue(&store, 42, cutoff, Utc::now().timestamp()).unwrap();
        let payload: String = store
            .conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT payload FROM strava_exports WHERE source_key='coros:new'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(payload, expected);
        store.clear_plan_data().unwrap();
        let count: u32 = store
            .conn
            .lock()
            .unwrap()
            .query_row("SELECT count(*) FROM strava_exports", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn expired_payloads_and_deleted_activity_history_are_cleaned_locally() {
        let (store, job) = fixture();
        store.conn.lock().unwrap().execute("INSERT INTO activities(provider,provider_record_id,sport,start_ts,duration_s,distance_m) VALUES('coros','one','running',?1,1800,5000)",[job.start]).unwrap();
        housekeeping(&store, Utc::now().timestamp() + 7_776_001).unwrap();
        let (state, _, payload, _) = outcome(&store);
        assert_eq!(state, "needs_review");
        assert!(payload.is_empty());
        store
            .conn
            .lock()
            .unwrap()
            .execute("DELETE FROM activities", [])
            .unwrap();
        housekeeping(&store, Utc::now().timestamp()).unwrap();
        let count: u32 = store
            .conn
            .lock()
            .unwrap()
            .query_row("SELECT count(*) FROM strava_exports", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn duplicated_export_payload_requires_review_without_writing() {
        let payload = "Made with Runalytics\n\nProgressive tempo\n\nMade with Runalytics\n\nProgressive tempo";
        assert_eq!(description_change("", payload, "reference"), DescriptionChange::Review);
    }

    #[test]
    fn appends_once_and_adopts_other_device_or_clipboard_copy() {
        let reference = marker(42, "coros:watch-record");
        let payload = "Runalytics\n\nIntervals\n\nResults";
        let DescriptionChange::Append(description) =
            description_change("My notes", payload, &reference)
        else {
            panic!("expected append")
        };
        assert!(description.starts_with("My notes\n\n"));
        assert_eq!(
            description_change(&description, "Changed prescription", &reference),
            DescriptionChange::Present
        );
        assert_eq!(
            description_change(&format!("My notes\n\n{payload}"), payload, &reference),
            DescriptionChange::Present
        );
        assert_eq!(
            description_change("[Runalytics sync v1:other]", payload, &reference),
            DescriptionChange::Review
        );
        assert_ne!(marker(43, "coros:watch-record"), reference);
    }

    #[test]
    fn matching_requires_close_time_distance_duration_and_running_sport() {
        let mut remote = RemoteActivity {
            id: 1,
            start_date: Utc::now(),
            elapsed_time: 1800.0,
            moving_time: None,
            distance: 5000.0,
            sport_type: "Run".into(),
            description: None,
            athlete: None,
        };
        let start = remote.start_date.timestamp();
        assert!(matches_activity(start + 120, 1800.0, 5000.0, &remote));
        assert!(!matches_activity(start + 121, 1800.0, 5000.0, &remote));
        assert!(!matches_activity(start, 1800.0, 5200.0, &remote));
        assert!(!matches_activity(start, 2000.0, 5000.0, &remote));
        remote.elapsed_time = 1898.0;
        remote.moving_time = Some(1820.0);
        assert!(matches_activity(start, 1820.0, 5000.0, &remote));
        assert!(!matches_activity(start, 1700.0, 5000.0, &remote));
        remote.sport_type = "Ride".into();
        assert!(!matches_activity(start, 1800.0, 5000.0, &remote));
    }

    #[test]
    fn ledger_migration_is_repeatable_and_enforces_target_uniqueness() {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        connection.execute_batch(SCHEMA).unwrap();
        connection.execute_batch(SCHEMA).unwrap();
        connection
            .execute(
                "INSERT INTO strava_accounts(athlete_id,client_id) VALUES(42,'123')",
                [],
            )
            .unwrap();
        let insert = "INSERT INTO strava_exports(athlete_id,source_key,workout_id,start_ts,duration_s,distance_m,payload,marker,target_id,status,created_at,updated_at) VALUES(42,?1,'workout',1,1800,5000,'notes','ref',99,'updated',1,1)";
        connection.execute(insert, ["coros:one"]).unwrap();
        assert!(connection.execute(insert, ["coros:two"]).is_err());
    }
}
