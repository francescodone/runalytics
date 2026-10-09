//! Runalytics desktop shell (Tauri).
//!
//! The shell is deliberately thin: every domain operation lives in the same
//! `ops` layer the MCP server exposes, so the GUI, the CLI, and any agent
//! talking MCP see identical behaviour and one SQLite database. What is here
//! is *desktop-only* concern:
//!
//! * **State** — the ops context, the persisted [`AppConfig`], and the
//!   keychain-backed token store, behind one `tauri::State`.
//! * **Commands** — `#[tauri::command]` wrappers that map errors to a
//!   displayable string (the frontend renders it verbatim).
//! * **The COROS consent flow** — the part provider-coros deliberately leaves
//!   out: bind a loopback port, register the client, open the browser, await
//!   the redirect, exchange the code, store the token in the keychain.
//! * **Provider sync** — build providers from stored tokens/settings and run
//!   the provider-core ingest pipeline, emitting progress to the frontend.
//! * **Shell chrome** — tray, single-instance, autostart.

// Tauri's command macro requires `State<'_, _>` and `AppHandle` parameters to
// be taken by value — the dispatch it generates matches on exactly that
// signature, so `needless_pass_by_value` cannot be satisfied here.
#![allow(clippy::needless_pass_by_value)]

mod callback;
mod secrets;
mod settings;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use chrono_tz::Tz;
use runalytics_core::Provider;
use runalytics_mcp::ops;
use runalytics_provider_core::{
    SyncPolicy, TokenStore, WearableProvider, ingest_all, register_account,
};
use runalytics_provider_coros::{CorosConfig, CorosProvider, OAuthClient, Region, oauth};
use runalytics_provider_garmin::{GarminConfig, GarminProvider, SidecarConfig};
use runalytics_store::{Db, ProviderAccountRepo, repos::ProviderAccount};
use serde::Deserialize;
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter as _, Manager, State};
use tauri_plugin_opener::OpenerExt as _;

pub use secrets::KeychainTokenStore;
pub use settings::AppConfig;

/// The event channel the frontend subscribes to for sync/connect progress.
pub const EVENT_SYNC: &str = "runalytics://sync";

/// Shared shell state: one database, one config, one token store.
pub struct AppState {
    /// The ops context — identical to what the MCP server would serve.
    /// A mutex because `set_settings` swaps it (timezone/calendar changes
    /// rebuild the config); reads clone it, which is cheap (`Db` is an
    /// `Arc<Connection>`).
    pub ops: Mutex<ops::Context>,
    /// Persisted user settings (mirrors the `settings` table).
    pub config: Mutex<AppConfig>,
    /// OAuth tokens, in the OS keychain.
    pub tokens: Arc<dyn TokenStore>,
}

impl AppState {
    /// A snapshot of the current config. A poisoned lock means a command
    /// panicked mid-write; recovering the inner value is safe because every
    /// write replaces the whole struct.
    fn config(&self) -> AppConfig {
        self.config
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// The current ops context (cloned; `Db` clone is an Arc bump).
    fn ctx(&self) -> ops::Context {
        self.ops
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// Map any command error to a string the frontend can show.
fn cmd_error<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

/// Parse a timezone string, rejecting anything the OS cannot resolve.
fn parse_tz(value: &str) -> Result<Tz, String> {
    value
        .parse()
        .map_err(|_| format!("'{value}' is not a valid IANA time zone"))
}

// ---------------------------------------------------------------------------
// Settings commands
// ---------------------------------------------------------------------------

/// Read the app settings.
#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> AppConfig {
    state.config()
}

/// Patch the app settings (timezone, calendar name). Writes through to the
/// `settings` table and rebuilds the ops context so the next command sees it.
#[tauri::command]
fn set_settings(state: State<'_, AppState>, patch: Value) -> Result<AppConfig, String> {
    let mut config = state.config();
    if let Some(tz) = patch.get("timezone").and_then(Value::as_str) {
        parse_tz(tz)?; // validate before persisting
        config.timezone = tz.to_string();
    }
    if let Some(name) = patch.get("calendarName").and_then(Value::as_str) {
        if name.trim().is_empty() {
            return Err("calendarName must not be empty".into());
        }
        config.calendar_name = name.to_string();
    }
    settings::save_config(&state.ctx().db, &config).map_err(cmd_error)?;
    {
        let mut guard = state.config.lock().unwrap_or_else(PoisonError::into_inner);
        guard.clone_from(&config);
    }
    {
        let mut guard = state.ops.lock().unwrap_or_else(PoisonError::into_inner);
        guard.config.timezone = parse_tz(&config.timezone)?;
        guard.config.calendar_name.clone_from(&config.calendar_name);
    }
    Ok(config)
}

// ---------------------------------------------------------------------------
// Domain commands (thin wrappers over ops, identical to MCP tool behaviour)
// ---------------------------------------------------------------------------

/// Today's dashboard: plan, metrics, readiness, injury risk, accounts.
#[tauri::command]
fn get_status(state: State<'_, AppState>) -> Result<Value, String> {
    let ctx = state.ctx();
    ops::get_status(&ctx, ops::GetStatusIn {}).map_err(cmd_error)
}

/// A fillable PlanRequest skeleton for the plan-creation form.
#[tauri::command]
fn plan_request_template(state: State<'_, AppState>) -> Result<Value, String> {
    let ctx = state.ctx();
    ops::plan_request_template(&ctx).map_err(cmd_error)
}

/// Generate a plan from a full PlanRequest and store it as a draft.
#[tauri::command]
fn generate_plan(
    state: State<'_, AppState>,
    request: Value,
    save: Option<bool>,
) -> Result<Value, String> {
    let ctx = state.ctx();
    ops::generate_plan(
        &ctx,
        ops::GeneratePlanIn {
            request,
            save: save.unwrap_or(true),
        },
    )
    .map_err(cmd_error)
}

/// List stored plans, newest first.
#[tauri::command]
fn list_plans(state: State<'_, AppState>) -> Result<Value, String> {
    let ctx = state.ctx();
    ops::list_plans(&ctx, ops::ListPlansIn {}).map_err(cmd_error)
}

/// Full plan JSON (active by default, or by `planId`).
#[tauri::command]
fn get_plan(state: State<'_, AppState>, plan_id: Option<String>) -> Result<Value, String> {
    let ctx = state.ctx();
    ops::get_plan(&ctx, ops::GetPlanIn { plan_id }).map_err(cmd_error)
}

/// Make a plan active (pauses the previous one).
#[tauri::command]
fn activate_plan(state: State<'_, AppState>, plan_id: String) -> Result<Value, String> {
    let ctx = state.ctx();
    ops::activate_plan(&ctx, &ops::ActivatePlanIn { plan_id }).map_err(cmd_error)
}

/// Partially update one planned session (move, retitle, re-target).
#[tauri::command]
fn update_session(state: State<'_, AppState>, patch: Value) -> Result<Value, String> {
    let input: ops::UpdateSessionIn =
        serde_json::from_value(patch).map_err(|e| format!("patch is not an UpdateSession: {e}"))?;
    let ctx = state.ctx();
    ops::update_session(&ctx, input).map_err(cmd_error)
}

/// Sessions scheduled from `from` (today by default) for `days` ahead.
#[tauri::command]
fn upcoming_sessions(
    state: State<'_, AppState>,
    from: Option<String>,
    days: Option<i64>,
) -> Result<Value, String> {
    let ctx = state.ctx();
    ops::upcoming_sessions(&ctx, &ops::UpcomingSessionsIn { from, days }).map_err(cmd_error)
}

/// Publish the active plan to Calendar.app (and the ICS file when configured).
#[tauri::command]
async fn sync_calendar(state: State<'_, AppState>) -> Result<Value, String> {
    let ctx = state.ctx();
    ops::sync_calendar(&ctx, ops::SyncCalendarIn {})
        .await
        .map_err(cmd_error)
}

/// The raw ICS feed for the active plan.
#[tauri::command]
fn get_calendar_feed(state: State<'_, AppState>) -> Result<Value, String> {
    let ctx = state.ctx();
    ops::get_calendar_feed(&ctx, ops::GetCalendarFeedIn {}).map_err(cmd_error)
}

/// Compute + persist readiness, injury risk, and load for one day.
#[tauri::command]
fn score_day(state: State<'_, AppState>, date: Option<String>) -> Result<Value, String> {
    let ctx = state.ctx();
    ops::score_day(&ctx, &ops::ScoreDayIn { date }).map_err(cmd_error)
}

/// Match a completed activity to its planned session and score it.
#[tauri::command]
fn score_session(state: State<'_, AppState>, patch: Value) -> Result<Value, String> {
    let input: ops::ScoreSessionIn =
        serde_json::from_value(patch).map_err(|e| format!("patch is not a ScoreSession: {e}"))?;
    let ctx = state.ctx();
    ops::score_session_tool(&ctx, &input).map_err(cmd_error)
}

/// Record the athlete's subjective feedback for a session.
#[tauri::command]
fn record_feedback(state: State<'_, AppState>, patch: Value) -> Result<Value, String> {
    let input: ops::RecordFeedbackIn =
        serde_json::from_value(patch).map_err(|e| format!("patch is not a RecordFeedback: {e}"))?;
    let ctx = state.ctx();
    ops::record_feedback(&ctx, input).map_err(cmd_error)
}

// ---------------------------------------------------------------------------
// Provider: accounts, COROS consent, sync
// ---------------------------------------------------------------------------

/// Connected wearable accounts.
#[tauri::command]
fn list_accounts(state: State<'_, AppState>) -> Result<Value, String> {
    let ctx = state.ctx();
    let accounts = ProviderAccountRepo::list(&ctx.db).map_err(cmd_error)?;
    Ok(json!({ "accounts": accounts }))
}

/// Input for [`connect_coros`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectCorosIn {
    /// Human label for this COROS account (keychain + dedup key).
    pub account_label: String,
    /// Region hint (`global`, `cn`, `eu`, `us`); probes in default order when absent.
    #[serde(default)]
    pub region: Option<String>,
}

/// Run the full COROS OAuth 2.1 + PKCE consent flow.
///
/// Binds a loopback port, registers the client (or reuses the stored id),
/// opens the system browser, and waits up to five minutes for the redirect.
/// On success the token is in the keychain, the account row exists, and the
/// config remembers the client id for future refreshes.
#[tauri::command]
async fn connect_coros(
    app: AppHandle,
    state: State<'_, AppState>,
    input: ConnectCorosIn,
) -> Result<Value, String> {
    let region = input
        .region
        .as_deref()
        .map(|label| {
            Region::parse(label)
                .ok_or_else(|| format!("unknown region '{label}' (global, cn, eu, us)"))
        })
        .transpose()?;
    let http = reqwest::Client::new();
    let endpoints = oauth::discover_endpoints(&http, region.unwrap_or(Region::Global))
        .await
        .map_err(cmd_error)?;
    let listener = callback::CallbackListener::bind()
        .await
        .map_err(cmd_error)?;
    let redirect_uri = url::Url::parse(&listener.redirect_uri()).map_err(|e| e.to_string())?;
    let stored_client_id = state.config().coros_client_id;
    let client_id = if let Some(id) = stored_client_id {
        id
    } else {
        let registration = endpoints.registration.as_ref().ok_or_else(|| {
            "COROS offered no dynamic registration endpoint; cannot register a client".to_string()
        })?;
        oauth::register_client(&http, registration, &redirect_uri)
            .await
            .map_err(cmd_error)?
    };
    let pkce = oauth::PkcePair::new();
    let csrf = oauth::csrf_state();
    let auth_url =
        oauth::authorization_url(&endpoints, &client_id, &redirect_uri, &pkce, &csrf, None)
            .map_err(cmd_error)?;
    app.opener()
        .open_url(auth_url.as_str(), None::<&str>)
        .map_err(|e| format!("could not open the browser: {e}"))?;
    let cb = listener
        .wait(std::time::Duration::from_secs(300))
        .await
        .map_err(cmd_error)?;
    if cb.state.as_deref() != Some(csrf.as_str()) {
        return Err("the callback state did not match — possible CSRF, flow aborted".into());
    }
    let code = cb.into_code().map_err(cmd_error)?;
    let token = oauth::exchange_code(&http, &endpoints, &client_id, &code, &redirect_uri, &pkce)
        .await
        .map_err(cmd_error)?;
    state
        .tokens
        .put("coros", &input.account_label, &token)
        .map_err(cmd_error)?;

    // Prove the token works and negotiate capabilities before declaring success.
    let ctx = state.ctx();
    let config = CorosConfig {
        account_label: input.account_label.clone(),
        oauth: OAuthClient {
            client_id: client_id.clone(),
            redirect_uri: redirect_uri.to_string(),
        },
        timezone: parse_tz(&state.config().timezone)?,
        known_region: region,
    };
    let mut provider = CorosProvider::new(config, state.tokens.clone());
    let info = provider.connect().await.map_err(cmd_error)?;
    let account_id = register_account(&ctx.db, Provider::Coros, &info).map_err(cmd_error)?;

    let mut config_row = state.config();
    config_row.coros_client_id = Some(client_id);
    config_row.coros_account_label = Some(input.account_label.clone());
    settings::save_config(&ctx.db, &config_row).map_err(cmd_error)?;
    {
        let mut guard = state.config.lock().unwrap_or_else(PoisonError::into_inner);
        *guard = config_row;
    }
    let payload = json!({
        "type": "connected",
        "provider": "coros",
        "account": info.account_label,
    });
    let _ = app.emit(EVENT_SYNC, &payload);
    Ok(json!({
        "accountId": account_id.to_string(),
        "account": info.account_label,
        "region": info.region,
        "capabilities": info.capabilities.data_classes(),
    }))
}

/// Forget a COROS account: keychain token removed, account row marked offline.
#[tauri::command]
fn disconnect_coros(state: State<'_, AppState>, account_label: String) -> Result<Value, String> {
    state
        .tokens
        .remove("coros", &account_label)
        .map_err(cmd_error)?;
    let ctx = state.ctx();
    if let Some(account) =
        ProviderAccountRepo::find_by_provider(&ctx.db, Provider::Coros).map_err(cmd_error)?
    {
        ProviderAccountRepo::mark_connected(&ctx.db, account.id, false).map_err(cmd_error)?;
    }
    Ok(json!({ "disconnected": account_label }))
}

/// Sync every connected wearable account (health, activities, fitness).
///
/// One account failing does not stop the others; one data class failing does
/// not stop the remaining classes (provider-core's contract). Progress lands
/// on the `runalytics://sync` event channel.
#[tauri::command]
async fn sync_providers(app: AppHandle, state: State<'_, AppState>) -> Result<Value, String> {
    let ctx = state.ctx();
    let accounts = ProviderAccountRepo::list(&ctx.db)
        .map_err(cmd_error)?
        .into_iter()
        .filter(|a| a.connected)
        .collect::<Vec<_>>();
    if accounts.is_empty() {
        return Err("no connected accounts — connect COROS first".into());
    }
    let today = ctx.today();
    let now = ctx.now();
    let policy = SyncPolicy::default();
    let mut reports = Vec::new();
    for account in accounts {
        let mut provider = match build_provider(&state, &account) {
            Ok(p) => p,
            Err(e) => {
                let payload = json!({
                    "account": account.account_label,
                    "provider": account.provider.as_str(),
                    "error": e,
                    "needsReauth": true,
                });
                let _ = app.emit(EVENT_SYNC, &payload);
                reports.push(payload);
                continue;
            }
        };
        if let Err(e) = provider.connect().await {
            let payload = json!({
                "account": account.account_label,
                "provider": account.provider.as_str(),
                "error": e.to_string(),
                "needsReauth": e.needs_reauth(),
            });
            let _ = app.emit(EVENT_SYNC, &payload);
            reports.push(payload);
            continue;
        }
        let outcomes =
            ingest_all(&ctx.db, account.id, provider.as_ref(), today, now, &policy).await;
        let classes = outcomes
            .into_iter()
            .map(|outcome| match outcome {
                Ok(report) => json!({
                    "class": report.data_class,
                    "records": report.records,
                    "cursor": report.cursor_after.map(|d| d.format("%Y-%m-%d").to_string()),
                }),
                Err(e) => json!({
                    "class": "unknown",
                    "error": e.to_string(),
                    "needsReauth": e.needs_reauth(),
                }),
            })
            .collect::<Vec<_>>();
        let _ = ProviderAccountRepo::record_sync(&ctx.db, account.id, now);
        let payload = json!({
            "account": account.account_label,
            "provider": account.provider.as_str(),
            "classes": classes,
        });
        let _ = app.emit(EVENT_SYNC, &payload);
        reports.push(payload);
    }
    Ok(json!({ "reports": reports }))
}

/// Build a live provider for a stored account, from keychain + settings.
fn build_provider(
    state: &State<'_, AppState>,
    account: &ProviderAccount,
) -> Result<Box<dyn WearableProvider>, String> {
    let timezone = parse_tz(&state.config().timezone)?;
    match account.provider {
        Provider::Coros => {
            let client_id =
                state.config().coros_client_id.clone().ok_or_else(|| {
                    "no stored COROS client id — reconnect the account".to_string()
                })?;
            Ok(Box::new(CorosProvider::new(
                CorosConfig {
                    account_label: account.account_label.clone(),
                    oauth: OAuthClient {
                        client_id,
                        // Only the code exchange compares this byte-for-byte,
                        // and that happens in `connect_coros` with the live
                        // listener URI; refresh ignores the field entirely.
                        redirect_uri: "http://localhost:0/callback".into(),
                    },
                    timezone,
                    known_region: account.region.as_deref().and_then(Region::parse),
                },
                state.tokens.clone(),
            )))
        }
        Provider::Garmin => Ok(Box::new(GarminProvider::new(GarminConfig {
            account_label: account.account_label.clone(),
            sidecar: SidecarConfig::default(),
            timezone,
        }))),
    }
}

// ---------------------------------------------------------------------------
// MCP server registration helper
// ---------------------------------------------------------------------------

/// The config snippet the user pastes into their agent (Copilot, Claude, …)
/// to talk to Runalytics over MCP, plus whether the sidecar binary was found
/// next to the app.
#[tauri::command]
fn mcp_registration(state: State<'_, AppState>) -> Result<Value, String> {
    let exe = std::env::current_exe().map_err(cmd_error)?;
    let sidecar = exe
        .parent()
        .map(|dir| dir.join("runalytics-mcp"))
        .filter(|p| p.exists());
    let command = sidecar
        .as_ref()
        .map_or_else(|| "runalytics-mcp".into(), |p| p.display().to_string());
    let mut args = Vec::new();
    if let Some(path) = &state.ctx().config.db_path {
        args.push("--db".to_string());
        args.push(path.display().to_string());
    }
    Ok(json!({
        "found": sidecar.is_some(),
        "config": {
            "mcpServers": {
                "runalytics": { "command": command, "args": args }
            }
        }
    }))
}

// ---------------------------------------------------------------------------
// App wiring
// ---------------------------------------------------------------------------

/// Open (or create) the app database and build the shared state. The app
/// config is read from the same database before the ops context is built, so
/// the timezone is correct from the first command.
///
/// # Errors
/// A message when the database cannot be opened or migrated, or the stored
/// timezone is unparseable.
pub fn open_state(db_path: PathBuf, tokens: Arc<dyn TokenStore>) -> Result<AppState, String> {
    if let Some(dir) = db_path.parent() {
        std::fs::create_dir_all(dir).map_err(cmd_error)?;
    }
    let db = Db::open(&db_path).map_err(cmd_error)?;
    let config = settings::load_config(&db).unwrap_or_default();
    let timezone = parse_tz(&config.timezone)?;
    let ops_config = ops::ServerConfig {
        db_path: Some(db_path),
        timezone,
        calendar_name: config.calendar_name.clone(),
        ics_path: None,
    };
    let ctx = ops::Context {
        db: db.clone(),
        config: ops_config,
    };
    Ok(AppState {
        ops: Mutex::new(ctx),
        config: Mutex::new(config),
        tokens,
    })
}

/// Build the tray menu. Closing the window keeps the menu-bar presence —
/// "Show" brings it back, "Quit" is the only exit.
fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::TrayIconBuilder;

    let show = MenuItem::with_id(app, "show", "Show Runalytics", true, None::<&str>)?;
    let sync = MenuItem::with_id(app, "sync", "Sync Now", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Runalytics", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &sync, &quit])?;
    let mut builder = TrayIconBuilder::with_id("runalytics-tray")
        .menu(&menu)
        .tooltip("Runalytics")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.unminimize();
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            // The frontend owns sync (it renders progress); the tray pokes it.
            "sync" => {
                let _ = app.emit("runalytics://tray-sync", ());
            }
            "quit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// Run the desktop app. `main.rs` calls this; the DB lives in the platform
/// app-data directory (`~/Library/Application Support/io.runalytics.desktop`).
///
/// # Errors
/// A message when state construction or tray setup fails at startup.
/// # Panics
/// On plugin-registration failures that Tauri reports via `expect` internally.
pub fn run() -> tauri::Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // A second launch focuses the existing window instead of opening
            // a second database connection.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_settings,
            get_status,
            plan_request_template,
            generate_plan,
            list_plans,
            get_plan,
            activate_plan,
            update_session,
            upcoming_sessions,
            sync_calendar,
            get_calendar_feed,
            score_day,
            score_session,
            record_feedback,
            list_accounts,
            connect_coros,
            disconnect_coros,
            sync_providers,
            mcp_registration
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let data_dir = handle.path().app_data_dir()?;
            let state = open_state(data_dir.join("runalytics.db"), Arc::new(KeychainTokenStore))
                .map_err(std::io::Error::other)?;
            app.manage(state);
            build_tray(&handle)?;
            // Keep the app alive when the window closes: the tray is the
            // resident surface; Quit (tray or ⌘Q) is the real exit.
            Ok(())
        })
        .build(tauri::generate_context!())?
        .run(|app, event| {
            // macOS convention: closing the last window must not quit an app
            // with a tray presence; re-show instead when the dock icon is
            // clicked again.
            if let tauri::RunEvent::Reopen {
                has_visible_windows: false,
                ..
            } = event
                && let Some(window) = app.get_webview_window("main")
            {
                let _ = window.show();
                let _ = window.set_focus();
            }
        });
    Ok(())
}
