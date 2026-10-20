mod assistant;
mod calendar;
mod calendar_publish;
mod commands;
mod error;
mod fit;
mod metrics;
mod plan;
mod proposal;
mod providers;
mod state;
mod store;
mod types;

use std::sync::Mutex;

use tauri::{Manager, RunEvent, WindowEvent};

use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let db_path = data_dir.join("runalytics.sqlite");
            let store = store::Store::open(&db_path)
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

            // Load settings (atomic JSON file, defaults on any problem).
            let settings_path = data_dir.join("settings.json");
            let settings = std::fs::read_to_string(&settings_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or(commands::Settings {
                    pmtiles_path: None,
                    last_provider: None,
                    theme: None,
                    llm_enabled: false,
                    llm_endpoint: None,
                    llm_model: None,
                    thresholds: Default::default(),
                });

            let providers = providers::build_providers();
            providers::coros::set_app_handle(app.handle().clone());

            app.manage(AppState {
                store,
                settings: Mutex::new(settings),
                providers,
                data_dir,
                syncing: std::sync::atomic::AtomicBool::new(false),
                sync_lock: tokio::sync::Mutex::new(()),
                calendar_lock: tokio::sync::Mutex::new(()),
            });

            // Single window with in-app navigation between the pages.
            let _win = tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("Runalytics")
            .inner_size(1180.0, 780.0)
            .min_inner_size(900.0, 600.0)
            .build()?;

            // Auto-sync on launch: pull fresh COROS data in the background
            // as soon as the app opens. The nav-bar spinner (driven by the
            // sync://started / sync://completed events) tells the user the
            // app is fetching, like the COROS iOS app's loading state.
            // Skipped entirely when no provider is connected — nothing to
            // pull, and the wizard owns the first-run experience.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = handle.state::<AppState>();
                let connected = state
                    .providers
                    .iter()
                    .any(|p| p.kind() == crate::types::ProviderKind::Coros && p.is_connected());
                if connected {
                    if let Err(e) =
                        commands::run_full_sync(&handle, &state, crate::types::ProviderKind::Coros)
                            .await
                    {
                        eprintln!("[auto-sync] failed: {e}");
                    }
                } else if let Err(error) = calendar_publish::refresh(&state).await {
                    eprintln!("[calendar] refresh failed: {error}");
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::provider_status,
            commands::check_provider_api,
            commands::connect_provider,
            commands::disconnect_provider,
            commands::sync_now,
            commands::sync_status,
            commands::dashboard,
            commands::heat_points,
            commands::all_activities,
            commands::activity_detail,
            commands::create_plan,
            commands::plan_defaults,
            commands::current_plan,
            commands::push_plan_to_provider,
            commands::abort_plan,
            commands::workouts_between,
            commands::ics_path,
            commands::calendar_events,
            commands::calendar_publishing,
            commands::save_calendar_publishing,
            commands::match_calendar_activity,
            commands::get_settings,
            commands::save_settings,
            commands::save_pain_entry,
            commands::delete_pain_entry,
            commands::resolve_pain,
            commands::mark_pain_followup,
            commands::pain_entries,
            commands::pain_days,
            commands::pain_by_region,
            commands::pending_proposal,
            commands::pending_pacing_proposal,
            commands::pacing_report,
            commands::proposals,
            commands::apply_proposal,
            commands::dismiss_proposal,
            commands::llm_status,
            commands::set_llm_api_key,
            commands::clear_llm_api_key,
            commands::ask_assistant,
            commands::pain_messages,
            commands::clear_pain_messages,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Runalytics")
        .run(|app, event| match event {
            RunEvent::WindowEvent {
                label,
                event: WindowEvent::CloseRequested { .. },
                ..
            } => {
                if label == "main" {
                    app.exit(0);
                }
            }
            _ => {}
        });
}
