//! Tauri commands exposed to the frontend. All heavy lifting happens here;
//! the frontend never talks to providers directly.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use tauri::{Emitter, State};

use crate::assistant;
use crate::calendar;
use crate::calendar_publish::{self, Publishing};
use crate::error::{AppError, Result};
use crate::metrics;
use crate::plan::{adaptive, engine, pacing};
use crate::proposal;
use crate::providers::Provider;
use crate::state::AppState;
use crate::store::provider_name;
use crate::types::*;

fn get_provider(state: &AppState, kind: ProviderKind) -> Result<Arc<dyn Provider>> {
    state
        .providers
        .iter()
        .find(|p| p.kind() == kind)
        .cloned()
        .ok_or_else(|| AppError::Other(format!("provider {} not built", provider_name(kind))))
}

/// The runner's configured decision boundaries, normalised. Read once per
/// command and passed down, so a scoring function never reaches into app state
/// and stays testable with any thresholds.
fn thresholds(state: &AppState) -> Thresholds {
    state.settings.lock().unwrap().thresholds()
}

// ---- status / sync ---------------------------------------------------------

#[tauri::command]
pub async fn provider_status(state: State<'_, AppState>) -> Result<Vec<ProviderStatus>> {
    let mut out = Vec::new();
    for p in state.providers.iter() {
        out.push(ProviderStatus {
            provider: p.kind(),
            connected: p.is_connected(),
            last_sync: state.store.last_sync(p.kind())?,
            detail: String::new(),
        });
    }
    Ok(out)
}

/// Verify the provider's API endpoint is reachable and authorized by listing
/// the tools it actually exposes. Surfaces real tool names in the UI so the
/// user (and we) can see whether the integration's expected tools exist.
#[tauri::command]
pub async fn check_provider_api(
    state: State<'_, AppState>,
    provider: ProviderKind,
) -> Result<Vec<String>> {
    get_provider(&state, provider)?.list_api_tools().await
}

#[tauri::command]
pub async fn connect_provider(
    state: State<'_, AppState>,
    provider: ProviderKind,
) -> Result<String> {
    let p = get_provider(&state, provider)?;
    p.connect().await
}

#[tauri::command]
pub async fn disconnect_provider(state: State<'_, AppState>, provider: ProviderKind) -> Result<()> {
    get_provider(&state, provider)?.disconnect().await
}

/// Full sync: activities, health, fitness, then tracks + adaptive review.
/// Emits `sync://started` / `sync://completed` so every open page can show
/// the spinner in the nav bar (like the COROS iOS app's loading state).
/// Concurrent calls collapse: if a sync is already running, this waits for
/// it and reports the same result.
#[tauri::command]
pub async fn sync_now(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    provider: ProviderKind,
) -> Result<SyncReport> {
    run_full_sync(&app, state.inner(), provider).await
}

/// Whether a sync is in flight right now (used by pages that mount while
/// the startup auto-sync is already running).
#[tauri::command]
pub fn sync_status(state: State<'_, AppState>) -> bool {
    state.syncing.load(std::sync::atomic::Ordering::SeqCst)
}

pub async fn run_full_sync(
    app: &tauri::AppHandle,
    state: &AppState,
    provider: ProviderKind,
) -> Result<SyncReport> {
    use std::sync::atomic::Ordering;
    // Only one sync at a time; a second caller gets the first one's result
    // by simply waiting on the same lock below.
    let _guard = state.sync_lock.lock().await;
    state.syncing.store(true, Ordering::SeqCst);
    let _ = app.emit("sync://started", ());
    let mut out = run_full_sync_inner(app, state, provider).await;
    // Every sync ends by republishing the calendar: regenerate the .ics
    // (after reconciling planned vs completed runs) and upload it when
    // publishing is configured. Report the outcome in the sync summary so
    // the user can see the calendar actually moved, not just that nothing
    // crashed.
    let calendar_message = match calendar_publish::refresh(state).await {
        Err(error) => Some(format!("calendar refresh failed: {error}")),
        Ok(_) => {
            let config = calendar_publish::load(&state.data_dir).ok();
            match config.as_ref().and_then(|config| config.last_error.clone()) {
                Some(error) => Some(format!("calendar publish failed: {error}")),
                None if config.as_ref().is_some_and(|config| config.enabled) => {
                    Some("calendar published".into())
                }
                None => None,
            }
        }
    };
    if let Some(message) = calendar_message {
        if let Ok(report) = &mut out {
            report.messages.push(message);
        } else {
            eprintln!("[calendar] {message}");
        }
    }
    state.syncing.store(false, Ordering::SeqCst);
    let _ = app.emit("sync://completed", serde_json::json!({ "ok": out.is_ok() }));
    out
}

async fn run_full_sync_inner(
    _app: &tauri::AppHandle,
    state: &AppState,
    provider: ProviderKind,
) -> Result<SyncReport> {
    let p = get_provider(state, provider)?;
    let mut messages = Vec::new();
    let activities_new = p.sync_activities(&state.store, 30).await?;
    {
        let _guard = state.calendar_lock.lock().await;
        calendar::reconcile(&state.store)?;
    }
    let health_days = p.sync_health(&state.store, 14).await?;
    p.sync_fitness(&state.store).await?;
    let tracks = p.sync_tracks(&state.store, 20).await.unwrap_or_else(|e| {
        messages.push(format!("track sync skipped: {e}"));
        0
    });
    // Adaptive plan review after fresh data.
    match adaptive::evaluate(&state.store, &thresholds(state)) {
        Ok(adjustments) => {
            for a in &adjustments {
                if state
                    .store
                    .soften_workout(&a.workout_id, a.new_kind, &a.new_title)
                    .is_ok()
                {
                    messages.push(format!("plan adjusted: {}", a.reason));
                }
            }
            if let Some((plan, workouts)) = state.store.current_plan()? {
                // Keep the provider side in sync: if the plan was pushed and
                // we just adjusted it (or new data arrived), re-push so the
                // watch schedule always mirrors the local plan.
                if !adjustments.is_empty() && plan.status == PlanStatus::Pushed {
                    match p.push_plan(&plan, &workouts, None).await {
                        Ok(Some(id)) => {
                            let _ = state.store.set_plan_pushed_id(&plan.id, &id);
                            messages.push(format!("plan re-synced to COROS ({id})"));
                        }
                        Ok(None) => messages.push("plan re-synced to COROS".into()),
                        Err(e) => messages.push(format!("plan re-sync failed: {e}")),
                    }
                }
            }
        }
        Err(e) => messages.push(format!("adaptive review failed: {e}")),
    }
    // Pacing review: with fresh tracks in the database, grade recent quality
    // sessions against their prescribed ranges and, if a run of them all landed
    // the same way outside, put an offer up for the runner to decide. Unlike
    // the adaptive softener this never changes the plan by itself.
    if let Some((plan, _)) = state.store.current_plan()? {
        match pacing::build(&state.store, &plan, &thresholds(state)) {
            Ok(Some(offer)) => {
                if state.store.insert_proposal(&offer).is_ok() {
                    messages.push(format!("pacing review: {}", offer.summary));
                }
            }
            Ok(None) => {}
            Err(e) => messages.push(format!("pacing review failed: {e}")),
        }
    }
    Ok(SyncReport {
        activities_new,
        health_days,
        fitness_updated: true,
        tracks_downloaded: tracks,
        messages,
    })
}

// ---- dashboard ---------------------------------------------------------------

#[tauri::command]
pub async fn dashboard(state: State<'_, AppState>, range: TimeRange) -> Result<DashboardPayload> {
    let days = range.days();
    let th = thresholds(&state);
    let stats = state.store.range_stats(days)?;
    let health = state.store.recent_health(days)?;
    let (risk_factors, risk_total) = metrics::compute_risk_factors(&state.store, days, &th)?;
    let mut scores = metrics::compute_scores(&state.store, &th)?;
    // Blend the range-aware heuristic sum into the headline risk percentage.
    let range_risk = (10.0 + risk_total * 55.0).clamp(0.0, 100.0).round() as u32;
    scores.injury_risk = ((scores.injury_risk + range_risk) / 2).clamp(0, 100);
    // Built from the blended scores so the cards' newest point matches the
    // headline numbers exactly (struct fields move, so compute before them).
    let score_history = metrics::score_history(&state.store, days, &scores, &th)?;
    let load = state.store.latest_training_load()?;
    // The training-status chart is a trend, so it keeps a readable window even
    // when the dashboard range is "Today": one point cannot show a boundary.
    let history_days = days.clamp(7, 30);
    let today = Utc::now().date_naive();
    let load_history = state
        .store
        .load_history(today - chrono::Duration::days(history_days - 1), today)?;
    // Derived before `load` moves into the payload below. The previous day's
    // ratio is passed so the shared Resuming/Performance band can be split the
    // way COROS splits it — by trend into the band, not by ratio inside it.
    // History is oldest→newest, so the second-from-the-end ratio is "yesterday".
    let ratios: Vec<f64> = load_history.iter().filter_map(|p| p.ratio).collect();
    let prev_ratio = if ratios.len() >= 2 {
        Some(ratios[ratios.len() - 2])
    } else {
        None
    };
    let training_status = metrics::training_status(&load, prev_ratio, &th);
    let recovery_band = metrics::recovery_band(scores.recovery_score, &th);
    let load_recommendation =
        metrics::load_recommendation(&load, &state.store.weekly_volumes(8)?, &th);
    Ok(DashboardPayload {
        scores,
        score_history,
        fitness: state.store.latest_fitness()?,
        load,
        load_history,
        training_status,
        thresholds: th,
        recovery_band: Some(recovery_band),
        load_recommendation,
        activities: state.store.list_activities(20)?,
        range,
        stats,
        health,
        risk_factors,
        active_pain: state.store.latest_pain()?,
        pending_proposal: state.store.pending_proposal()?,
    })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardPayload {
    pub scores: DashboardScores,
    /// Per-day sub-score trend for the readiness cards, oldest→newest,
    /// spanning the selected range.
    pub score_history: Vec<ScorePoint>,
    pub fitness: Option<FitnessAssessment>,
    pub load: Option<TrainingLoad>,
    /// Daily acute/chronic load for the training-status chart, oldest→newest.
    pub load_history: Vec<LoadPoint>,
    /// Which side of the boundaries recent training falls on.
    pub training_status: Option<TrainingStatus>,
    /// The boundaries the zones above were computed with, so the chart draws
    /// the same bands the runner configured in Settings.
    pub thresholds: Thresholds,
    /// Which recovery band the recovery score falls in.
    pub recovery_band: Option<RecoveryBand>,
    /// This week's 7-day load target from the 3:1 cycle.
    pub load_recommendation: Option<LoadRecommendation>,
    pub activities: Vec<Activity>,
    pub range: TimeRange,
    pub stats: RangeStats,
    pub health: Vec<HealthDaily>,
    pub risk_factors: Vec<RiskFactor>,
    /// The most recent unresolved pain entry, for the Today-tab banner.
    pub active_pain: Option<PainEntry>,
    /// An unanswered plan-change offer, surfaced with Accept right there.
    pub pending_proposal: Option<PlanProposal>,
}

#[tauri::command]
pub async fn heat_points(state: State<'_, AppState>) -> Result<Vec<HeatPoint>> {
    state.store.heat_points(3)
}

/// Full synced activity history (newest first). Used by the Achievements
/// page to compute streaks and lifetime badge progress.
#[tauri::command]
pub async fn all_activities(state: State<'_, AppState>) -> Result<Vec<Activity>> {
    state.store.list_activities(i64::MAX)
}

/// Everything for the activity detail sheet: GPS track, session data, the
/// recovery signals for that day, and the injury-risk heuristics evaluated
/// as of that day.
#[tauri::command]
pub async fn activity_detail(
    state: State<'_, AppState>,
    activity_id: i64,
    range: TimeRange,
) -> Result<ActivityDetail> {
    let activity = state
        .store
        .activity_by_id(activity_id)?
        .ok_or_else(|| AppError::Other(format!("activity {activity_id} not found")))?;
    let day = activity.start_time.date_naive();
    let days = range.days();
    let th = thresholds(&state);
    let (risk_factors, _risk_total) = metrics::compute_risk_factors_as_of(
        &state.store,
        day,
        days,
        &th,
        Some(activity.start_time.timestamp()),
    )?;
    // The same blended headline the dashboard hero shows, as of the run's
    // day — not just the factor-sum half of it — so the sheet, the calendar
    // description, and the dashboard agree on one number.
    let injury_risk = metrics::risk_headline_as_of(
        &state.store,
        day,
        days,
        &th,
        Some(activity.start_time.timestamp()),
    )?;
    Ok(ActivityDetail {
        track: state.store.track_points(activity_id, 2000)?,
        health: state.store.health_on(day)?,
        load: state.store.training_load_on_or_before(day)?,
        fitness: state.store.fitness_on_or_before(day)?,
        risk_factors,
        injury_risk,
        activity,
    })
}

// ---- plans ----------------------------------------------------------------

/// One run's frozen score snapshot with its injury-risk impact — how many
/// points this session moved the runner's risk (risk with the run minus risk
/// without it) — exposed so the plan page can mark each done session.
/// `workout_id` is set when the run matched a plan session, which is how the
/// plan page keys the map.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunScore {
    pub activity_id: i64,
    pub workout_id: Option<String>,
    pub load: Option<f64>,
    pub risk: u32,
    pub risk_delta: i64,
}

#[tauri::command]
pub async fn run_scores(state: State<'_, AppState>) -> Result<Vec<RunScore>> {
    let workout_by_activity: std::collections::HashMap<i64, String> = state
        .store
        .calendar_completions()?
        .into_iter()
        .map(|(workout_id, activity_id, _)| (activity_id, workout_id))
        .collect();
    Ok(state
        .store
        .activity_scores()?
        .into_iter()
        .map(|(activity_id, load, risk, risk_delta)| RunScore {
            activity_id,
            workout_id: workout_by_activity.get(&activity_id).cloned(),
            load,
            risk,
            risk_delta,
        })
        .collect())
}

#[tauri::command]
pub async fn create_plan(
    state: State<'_, AppState>,
    request: PlanRequest,
) -> Result<(TrainingPlan, Vec<PlanWorkout>)> {
    // Only one plan may own the calendar at a time: the current plan must be
    // aborted before a new one can be created.
    if state.store.current_plan()?.is_some() {
        return Err(AppError::Other(
            "there is already an active plan — abort it before creating a new one".into(),
        ));
    }
    // Baseline for a plan that doesn't state its own mileage: the mean of the
    // runner's recent complete weeks. 15 km is a floor for a brand-new runner
    // with nothing synced, not a substitute for real data.
    let mileage = state.store.mileage_stats()?;
    let baseline = mileage
        .weekly_avg_m
        .map(|m| m / 1000.0)
        .unwrap_or(15.0)
        .max(10.0);
    let (plan, workouts) = engine::generate(&request, baseline)?;
    state.store.insert_plan(&plan)?;
    state.store.insert_workouts(&workouts)?;
    calendar_publish::refresh(&state).await?;
    Ok((plan, workouts))
}

/// Device-derived starting values for the create-plan form: current weekly
/// mileage from synced activities and threshold pace / race predictions
/// from the latest fitness assessment the watch reported.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanDefaults {
    pub current_weekly_km: Option<f64>,
    pub threshold_sec_km: Option<f64>,
    pub predict_5k_s: Option<i64>,
    pub predict_10k_s: Option<i64>,
    pub predict_half_s: Option<i64>,
    pub predict_marathon_s: Option<i64>,
    /// Distance logged in the week currently in progress (km).
    pub this_week_km: f64,
    /// Every running kilometre synced so far (km) — the odometer.
    pub total_km: f64,
    /// Complete weeks the average is built from (1..=4); 0 means no history.
    pub weeks_observed: i64,
    /// When the watch last synced, so the form can say how fresh this is.
    pub last_sync: Option<DateTime<Utc>>,
    /// Unresolved pain, so the create-plan form can warn before committing
    /// to a plan built on top of an injury.
    pub active_pain: Option<PainEntry>,
}

#[tauri::command]
pub async fn plan_defaults(state: State<'_, AppState>) -> Result<PlanDefaults> {
    let mileage = state.store.mileage_stats()?;
    let fitness = state.store.latest_fitness()?;
    Ok(PlanDefaults {
        current_weekly_km: mileage.weekly_avg_m.map(|m| m / 1000.0),
        this_week_km: mileage.this_week_m / 1000.0,
        total_km: mileage.total_m / 1000.0,
        weeks_observed: mileage.weeks_observed,
        last_sync: mileage.last_sync,
        threshold_sec_km: fitness.as_ref().and_then(|f| f.threshold_sec_km),
        predict_5k_s: fitness.as_ref().and_then(|f| f.predict_5k_s),
        predict_10k_s: fitness.as_ref().and_then(|f| f.predict_10k_s),
        predict_half_s: fitness.as_ref().and_then(|f| f.predict_half_s),
        predict_marathon_s: fitness.as_ref().and_then(|f| f.predict_marathon_s),
        active_pain: state.store.latest_pain()?,
    })
}

#[tauri::command]
pub async fn current_plan(
    state: State<'_, AppState>,
) -> Result<Option<(TrainingPlan, Vec<PlanWorkout>)>> {
    state.store.current_plan()
}

#[tauri::command]
pub async fn push_plan_to_provider(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    provider: ProviderKind,
) -> Result<String> {
    let (plan, workouts) = state
        .store
        .current_plan()?
        .ok_or_else(|| AppError::Other("no active plan".into()))?;
    let p = get_provider(&state, provider)?;
    // The COROS round-trip takes tens of seconds; report each stage so the
    // Plan page can show a live percentage instead of a dead button.
    let emit = |stage: &str, percent: u8| {
        let _ = app.emit(
            "plan://sync",
            PlanSyncProgress {
                op: "push".into(),
                percent,
                stage: stage.into(),
            },
        );
    };
    let id = p.push_plan(&plan, &workouts, Some(&emit)).await?;
    if let Some(id) = &id {
        state.store.set_plan_pushed_id(&plan.id, id)?;
    }
    state.store.set_plan_status(&plan.id, PlanStatus::Pushed)?;
    Ok(id.unwrap_or_else(|| "scheduled".into()))
}

#[tauri::command]
pub async fn abort_plan(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<()> {
    if let Some((plan, _)) = state.store.current_plan()? {
        // A plan that was pushed to a watch must be cancelled there first:
        // COROS keeps every plan it is handed, so dropping only the local
        // copy would leave the schedule live on the watch. A provider
        // failure must not trap the runner in a plan they want gone — the
        // local delete proceeds and the message says the watch may still
        // show the schedule.
        let mut provider_note = None;
        if plan.status == PlanStatus::Pushed {
            match get_provider(&state, ProviderKind::Coros) {
                Ok(p) => {
                    let emit = |stage: &str, percent: u8| {
                        let _ = app.emit(
                            "plan://sync",
                            PlanSyncProgress {
                                op: "cancel".into(),
                                percent,
                                stage: stage.into(),
                            },
                        );
                    };
                    if let Err(e) = p.cancel_plan(plan.pushed_id.as_deref(), Some(&emit)).await {
                        eprintln!("[coros] plan cancel failed: {e}");
                        provider_note = Some(format!(
                            "the plan was removed locally, but COROS could not be updated ({e}) — \
                             quit it in the COROS app if it still shows there"
                        ));
                    }
                }
                Err(e) => eprintln!("[coros] provider unavailable for cancel: {e}"),
            }
        }
        // Delete the plan outright: cascading removes its workouts, which is
        // what actually clears them from the calendar view and the .ics file.
        state.store.delete_plan(&plan.id)?;
        calendar_publish::refresh(&state).await?;
        if let Some(note) = provider_note {
            return Err(AppError::Other(note));
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn workouts_between(
    state: State<'_, AppState>,
    from: String,
    to: String,
) -> Result<Vec<PlanWorkout>> {
    state.store.workouts_between(&from, &to)
}

// ---- calendar export -------------------------------------------------------

/// Read-only: where the .ics lives. Deliberately does not regenerate or
/// upload — opening the Settings pane must not publish. Publishing happens
/// on sync, on calendar edits, and through `publish_calendar`.
#[tauri::command]
pub async fn ics_path(state: State<'_, AppState>) -> Result<String> {
    Ok(calendar_publish::export_path(&state.data_dir))
}

/// The explicit "Refresh calendar" action: regenerate the .ics and upload
/// it when publishing is enabled.
#[tauri::command]
pub async fn publish_calendar(state: State<'_, AppState>) -> Result<String> {
    calendar_publish::refresh(&state).await
}

#[tauri::command]
pub async fn calendar_events(
    state: State<'_, AppState>,
    from: String,
    to: String,
) -> Result<Vec<calendar::CalendarEvent>> {
    let from = crate::store::parse_date_pub(&from)?;
    let to = crate::store::parse_date_pub(&to)?;
    Ok(calendar::events(&state.store)?
        .into_iter()
        .filter(|event| event.date >= from && event.date <= to)
        .collect())
}

#[tauri::command]
pub async fn calendar_publishing(state: State<'_, AppState>) -> Result<Publishing> {
    calendar_publish::load(&state.data_dir)
}

#[tauri::command]
pub async fn save_calendar_publishing(
    state: State<'_, AppState>,
    config: Publishing,
    password: Option<String>,
) -> Result<Publishing> {
    calendar_publish::configure(&state, config, password).await
}

#[tauri::command]
pub async fn match_calendar_activity(
    state: State<'_, AppState>,
    workout_id: String,
    activity_id: i64,
) -> Result<()> {
    {
        let _guard = state.calendar_lock.lock().await;
        calendar::match_activity(&state.store, &workout_id, activity_id)?;
    }
    calendar_publish::refresh(&state).await?;
    Ok(())
}

/// Drag-and-drop reschedule from the calendar. The local calendar (and its
/// published feed) is refreshed before returning; a pushed plan is re-sent to
/// the watch in the background because that round-trip takes tens of seconds.
#[tauri::command]
pub async fn move_workout(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    workout_id: String,
    date: String,
) -> Result<()> {
    let to = crate::store::parse_date_pub(&date)?;
    {
        let _guard = state.calendar_lock.lock().await;
        calendar::move_workout(
            &state.store,
            &workout_id,
            to,
            chrono::Local::now().date_naive(),
        )?;
    }
    calendar_publish::refresh(&state).await?;
    if let Some((plan, workouts)) = state.store.current_plan()? {
        if plan.status == PlanStatus::Pushed {
            tauri::async_runtime::spawn(async move {
                use tauri::Manager;
                let state = app.state::<AppState>();
                let result = match get_provider(&state, ProviderKind::Coros) {
                    Ok(p) => p.push_plan(&plan, &workouts, None).await,
                    Err(e) => Err(e),
                };
                match result {
                    Ok(Some(id)) => {
                        let _ = state.store.set_plan_pushed_id(&plan.id, &id);
                    }
                    Ok(None) => {}
                    Err(e) => eprintln!("[coros] re-push after move failed: {e}"),
                }
            });
        }
    }
    Ok(())
}

// ---- settings ---------------------------------------------------------------

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub pmtiles_path: Option<String>,
    pub last_provider: Option<ProviderKind>,
    /// "system" | "light" | "dark". None behaves as "system" — the OS preference
    /// wins, which is also what the app did before the setting existed.
    #[serde(default)]
    pub theme: Option<String>,
    /// Colour palette id ("amethyst", "glacier", ...). None means the default.
    #[serde(default)]
    pub palette: Option<String>,
    /// Injury assistant: an OpenAI-compatible chat endpoint. `#[serde(default)]`
    /// keeps existing settings.json files loading after these fields appeared.
    #[serde(default)]
    pub llm_enabled: bool,
    #[serde(default)]
    pub llm_endpoint: Option<String>,
    #[serde(default)]
    pub llm_model: Option<String>,
    /// Editable decision boundaries for training load / status / recovery.
    /// `#[serde(default)]` keeps older settings.json files loading; the values
    /// are normalised on read so a hand-edited file cannot invert a band.
    #[serde(default)]
    pub thresholds: Thresholds,
}

impl Settings {
    /// The thresholds every scoring call site should use: clamped and ordered.
    pub fn thresholds(&self) -> Thresholds {
        self.thresholds.normalized()
    }
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Settings> {
    Ok(state.settings.lock().unwrap().clone())
}

#[tauri::command]
pub async fn save_settings(state: State<'_, AppState>, mut settings: Settings) -> Result<()> {
    // Store the boundaries already normalised. Every read path normalises too,
    // but persisting the raw form would let the Settings page show one number
    // while the dashboard drew another.
    settings.thresholds = settings.thresholds.normalized();
    {
        let mut guard = state.settings.lock().unwrap();
        *guard = settings.clone();
    }
    let json = serde_json::to_string_pretty(&settings)?;
    let tmp = state.data_dir.join("settings.json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, state.data_dir.join("settings.json"))?;
    Ok(())
}

/// Kept for symmetry with the frontend clock; unused server-side.
#[allow(dead_code)]
fn now() -> DateTime<Utc> {
    Utc::now()
}

// ---- injury journal -----------------------------------------------------------

/// Apply a set of workout edits and propagate them everywhere the plan is
/// mirrored: the .ics export and, when the plan was pushed, the provider.
/// Shared by the adaptive engine (auto-applied) and accepted proposals, so the
/// two paths can never drift apart.
async fn apply_workout_edits(
    state: &AppState,
    provider: Option<Arc<dyn Provider>>,
    edits: &[WorkoutEdit],
) -> Result<Vec<PlanWorkout>> {
    let (plan, workouts) = state
        .store
        .current_plan()?
        .ok_or_else(|| AppError::Other("no active plan".into()))?;
    for e in edits {
        let w = workouts
            .iter()
            .find(|w| w.id == e.workout_id)
            .ok_or_else(|| AppError::Other(format!("workout {} no longer exists", e.workout_id)))?;
        state.store.update_workout(&PlanWorkout {
            id: w.id.clone(),
            plan_id: w.plan_id.clone(),
            date: w.date,
            kind: e.kind,
            title: e.title.clone(),
            distance_m: e.distance_m,
            duration_min: e.duration_min,
            pace_lo_s_km: e.pace_lo_s_km,
            pace_hi_s_km: e.pace_hi_s_km,
            done: w.done,
        })?;
    }
    let fresh = state.store.plan_workouts(&plan.id)?;
    calendar_publish::refresh(state).await?;
    // Keep the watch schedule mirroring the local plan, as the adaptive path does.
    if let (Some(p), true) = (provider, plan.status == PlanStatus::Pushed) {
        if let Ok(Some(id)) = p.push_plan(&plan, &fresh, None).await {
            let _ = state.store.set_plan_pushed_id(&plan.id, &id);
        }
    }
    Ok(fresh)
}

#[tauri::command]
pub async fn save_pain_entry(
    state: State<'_, AppState>,
    body_part: String,
    intensity: u32,
    started_on: String,
    notes: String,
) -> Result<(PainEntry, Option<PlanProposal>)> {
    if !(intensity <= 10) {
        return Err(AppError::Other("intensity must be between 0 and 10".into()));
    }
    let entry = PainEntry {
        id: uuid::Uuid::new_v4().to_string(),
        body_part,
        intensity,
        started_on: crate::store::parse_date_pub(&started_on)?,
        notes,
        created_at: Utc::now(),
        resolved_on: None,
        followup_at: None,
    };
    state.store.insert_pain_entry(&entry)?;
    // The offer is generated at the moment pain is logged, so the ask exists
    // as soon as the entry does. A failure here must not lose the journal entry.
    let proposal = match proposal::build(&state.store, &entry) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[proposal] build failed: {e}");
            None
        }
    };
    if let Some(p) = &proposal {
        state.store.insert_proposal(p)?;
    }
    Ok((entry, proposal))
}

#[tauri::command]
pub async fn delete_pain_entry(state: State<'_, AppState>, id: String) -> Result<()> {
    state.store.delete_pain_entry(&id)
}

/// The runner says the pain went away on `day` — closes the episode.
#[tauri::command]
pub async fn resolve_pain(
    state: State<'_, AppState>,
    id: String,
    resolved_on: Option<String>,
) -> Result<()> {
    let day = match resolved_on {
        Some(s) => crate::store::parse_date_pub(&s)?,
        None => Utc::now().date_naive(),
    };
    state.store.resolve_pain(&id, day)
}

/// Record that the "is it gone yet?" follow-up was shown for this episode.
#[tauri::command]
pub async fn mark_pain_followup(state: State<'_, AppState>, id: String) -> Result<()> {
    state.store.mark_pain_followup(&id, Utc::now())
}

#[tauri::command]
pub async fn pain_entries(state: State<'_, AppState>) -> Result<Vec<PainEntry>> {
    state.store.list_pain_entries(500)
}

/// Per-day worst intensity for the calendar month currently on screen.
#[tauri::command]
pub async fn pain_days(
    state: State<'_, AppState>,
    from: String,
    to: String,
) -> Result<Vec<(String, u32)>> {
    let from = crate::store::parse_date_pub(&from)?;
    let to = crate::store::parse_date_pub(&to)?;
    state.store.pain_days(from, to)
}

/// Worst intensity per region over the last `days` — tints the body map.
#[tauri::command]
pub async fn pain_by_region(state: State<'_, AppState>, days: i64) -> Result<Vec<(String, u32)>> {
    let since = Utc::now().date_naive() - chrono::Duration::days(days.max(1));
    state.store.pain_by_region(since)
}

// ---- plan proposals ------------------------------------------------------------

#[tauri::command]
pub async fn pending_proposal(state: State<'_, AppState>) -> Result<Option<PlanProposal>> {
    state.store.pending_proposal()
}

#[tauri::command]
pub async fn proposals(state: State<'_, AppState>, limit: i64) -> Result<Vec<PlanProposal>> {
    state.store.list_proposals(limit.max(1))
}

/// Accept an offer. Pain offers rewrite their days through `apply_workout_edits`;
/// a pacing offer rewrites nothing — it stores the plan's new quality-pace
/// offset, and every range downstream (calendar notes, watch push, the Plan
/// page's bars) shifts through that one number. Both then refresh the calendar
/// and re-push so all mirrors of the plan move together.
#[tauri::command]
pub async fn apply_proposal(
    state: State<'_, AppState>,
    provider: ProviderKind,
    proposal_id: String,
) -> Result<Vec<PlanWorkout>> {
    let p = state
        .store
        .proposal_by_id(&proposal_id)?
        .ok_or_else(|| AppError::Other("proposal not found".into()))?;
    if p.status != ProposalStatus::Pending {
        return Err(AppError::Other("this offer was already answered".into()));
    }
    let prov = get_provider(&state, provider).ok();
    let fresh = if p.source == ProposalSource::Pacing {
        let Some((offset, from)) = p.plan_quality_offset_sec_km.zip(p.plan_quality_offset_from)
        else {
            return Err(AppError::Other(
                "this pacing offer is missing its adjustment".into(),
            ));
        };
        let (plan, _) = state
            .store
            .current_plan()?
            .ok_or_else(|| AppError::Other("no active plan".into()))?;
        state
            .store
            .set_plan_quality_offset(&plan.id, offset, from)?;
        // The stored plan moved; the mirrors below must be built from the
        // updated plan, not the copy loaded before the write.
        let plan = TrainingPlan {
            quality_offset_sec_km: offset,
            quality_offset_from: Some(from),
            ..plan
        };
        calendar_publish::refresh(&state).await?;
        let fresh = state.store.plan_workouts(&plan.id)?;
        if let (Some(p), true) = (prov, plan.status == PlanStatus::Pushed) {
            if let Ok(Some(id)) = p.push_plan(&plan, &fresh, None).await {
                let _ = state.store.set_plan_pushed_id(&plan.id, &id);
            }
        }
        fresh
    } else {
        apply_workout_edits(&state, prov, &p.edits).await?
    };
    state
        .store
        .set_proposal_status(&p.id, ProposalStatus::Applied, Some(Utc::now()), None)?;
    Ok(fresh)
}

#[tauri::command]
pub async fn dismiss_proposal(
    state: State<'_, AppState>,
    proposal_id: String,
    reason: Option<String>,
) -> Result<()> {
    state.store.set_proposal_status(
        &proposal_id,
        ProposalStatus::Dismissed,
        Some(Utc::now()),
        reason.as_deref(),
    )
}

/// The pending pacing offer for the current plan, for the Plan page. None when
/// no plan or no warranted offer — the page then shows only the range bars.
///
/// The offer is materialised here rather than only at sync time: a session
/// completed by hand-matching a run never triggers a sync, and the Plan page is
/// where the offer belongs. `pacing::build` suppresses duplicates itself, so
/// asking twice cannot stack two offers up.
#[tauri::command]
pub async fn pending_pacing_proposal(state: State<'_, AppState>) -> Result<Option<PlanProposal>> {
    let Some((plan, _)) = state.store.current_plan()? else {
        return Ok(None);
    };
    if let Some(offer) = pacing::build(&state.store, &plan, &thresholds(&state))? {
        state.store.insert_proposal(&offer)?;
    }
    state.store.pending_pacing_proposal(&plan.id)
}

/// Where recent quality sessions landed against their prescribed ranges, and
/// how many in a row would be needed to warrant an adjustment. None when there
/// is no plan to grade.
#[tauri::command]
pub async fn pacing_report(state: State<'_, AppState>) -> Result<Option<PacingReport>> {
    let Some((plan, _)) = state.store.current_plan()? else {
        return Ok(None);
    };
    Ok(Some(pacing::report(
        &state.store,
        &plan,
        &thresholds(&state),
    )?))
}

// ---- injury assistant -----------------------------------------------------------

/// Keyring slot for the assistant API key — separate service from COROS OAuth.
const LLM_KEYRING_SERVICE: &str = "com.fradone.runalytics.llm";
const LLM_KEYRING_USER: &str = "api_key";

fn llm_key_entry() -> Result<keyring::Entry> {
    keyring::Entry::new(LLM_KEYRING_SERVICE, LLM_KEYRING_USER)
        .map_err(|e| AppError::Other(format!("keyring unavailable: {e}")))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmStatus {
    /// Endpoint + model configured and the toggle on.
    pub configured: bool,
    /// A key is stored (never the key itself).
    pub has_key: bool,
    pub endpoint: Option<String>,
    pub model: Option<String>,
}

#[tauri::command]
pub async fn llm_status(state: State<'_, AppState>) -> Result<LlmStatus> {
    let s = state.settings.lock().unwrap().clone();
    let has_key = llm_key_entry()?
        .get_password()
        .map(|k| !k.is_empty())
        .unwrap_or(false);
    Ok(LlmStatus {
        configured: s.llm_enabled
            && s.llm_endpoint
                .as_deref()
                .is_some_and(|e| !e.trim().is_empty()),
        has_key,
        endpoint: s.llm_endpoint,
        model: s.llm_model,
    })
}

#[tauri::command]
pub async fn set_llm_api_key(key: String) -> Result<()> {
    if key.trim().is_empty() {
        return Err(AppError::Other("the key is empty".into()));
    }
    llm_key_entry()?
        .set_password(key.trim())
        .map_err(|e| AppError::Other(format!("could not store the key: {e}")))
}

#[tauri::command]
pub async fn clear_llm_api_key() -> Result<()> {
    match llm_key_entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::Other(format!("could not remove the key: {e}"))),
    }
}

/// One turn of the assistant chat for one journal entry. The user message and
/// the reply are both persisted so the thread survives reloads. With no
/// endpoint configured the turn is answered locally — the journal stays usable.
#[tauri::command]
pub async fn ask_assistant(
    state: State<'_, AppState>,
    entry_id: String,
    question: String,
) -> Result<PainMessage> {
    let entry = state
        .store
        .pain_entry_by_id(&entry_id)?
        .ok_or_else(|| AppError::Other("this pain entry no longer exists".into()))?;
    let now = Utc::now();
    let user_msg = PainMessage {
        id: uuid::Uuid::new_v4().to_string(),
        body_part: entry.body_part.clone(),
        entry_id: Some(entry_id.clone()),
        role: "user".into(),
        content: question.clone(),
        ts: now,
    };
    state.store.insert_pain_message(&user_msg)?;

    let history = state.store.list_pain_messages(&entry_id)?;
    let settings = state.settings.lock().unwrap().clone();
    let reply_text = if settings.llm_enabled {
        let endpoint = settings
            .llm_endpoint
            .as_deref()
            .filter(|e| !e.trim().is_empty())
            .ok_or_else(|| {
                AppError::Other("no assistant endpoint configured — set one in Settings".into())
            })?;
        let endpoint = assistant::validate_endpoint(endpoint)?;
        let model = settings
            .llm_model
            .as_deref()
            .filter(|m| !m.trim().is_empty())
            .unwrap_or("gpt-4o-mini")
            .to_string();
        let key = llm_key_entry().ok().and_then(|e| e.get_password().ok());
        let ctx = assistant::build_context(&state.store, &entry.body_part, &settings.thresholds())?;
        // Drop the message we just stored from the history replay.
        let prior: Vec<PainMessage> = history
            .into_iter()
            .filter(|m| m.id != user_msg.id)
            .collect();
        assistant::chat(&endpoint, &model, key.as_deref(), &ctx, &prior, &question).await?
    } else {
        assistant::offline_note(
            settings
                .llm_endpoint
                .as_deref()
                .is_some_and(|e| !e.trim().is_empty()),
        )
    };

    let reply = PainMessage {
        id: uuid::Uuid::new_v4().to_string(),
        body_part: entry.body_part,
        entry_id: Some(entry_id),
        role: "assistant".into(),
        content: reply_text,
        ts: Utc::now(),
    };
    state.store.insert_pain_message(&reply)?;
    Ok(reply)
}

#[tauri::command]
pub async fn pain_messages(
    state: State<'_, AppState>,
    entry_id: String,
) -> Result<Vec<PainMessage>> {
    state.store.list_pain_messages(&entry_id)
}

#[tauri::command]
pub async fn clear_pain_messages(state: State<'_, AppState>, entry_id: String) -> Result<()> {
    state.store.clear_pain_messages(&entry_id)
}
