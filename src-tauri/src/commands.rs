//! Tauri commands exposed to the frontend. All heavy lifting happens here;
//! the frontend never talks to providers directly.

use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use tauri::{Emitter, State};

use crate::assistant;
use crate::calendar;
use crate::calendar_publish::{self, Publishing};
use crate::error::{AppError, Result};
use crate::laps;
use crate::metrics;
use crate::plan::{adaptive, engine, pacing, receipt};
use crate::plan::receipt::ReceiptLine;
use crate::plan_sync;
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

// ---- plan-mirror stage events ----------------------------------------------
//
// A plan lives in four places: this device's database, the published
// calendar, the encrypted gist the phone pulls, and the COROS watch. Any
// interaction that can move one of them (create, sync, erase, abort, move a
// session) reports each mirror as `working` then `done`/`error` on the
// `plan://stage` event, so the toast can show every line and only start its
// self-dismiss countdown once all of them have settled. The frontend owns
// the labels; `target` is the stable contract.

#[derive(Clone, Copy)]
enum Stage {
    Db,
    Calendar,
    Gist,
    Coros,
    Strava,
    /// The operation itself: its `done`/`error` tells the toast the command
    /// has returned, so no new line can arrive any more — the only case the
    /// toast cannot infer from the lines is a mirror that was never
    /// attempted (an unconfigured gist emits nothing at all).
    Op,
}

impl Stage {
    fn target(self) -> &'static str {
        match self {
            Stage::Db => "db",
            Stage::Calendar => "calendar",
            Stage::Gist => "gist",
            Stage::Coros => "coros",
            Stage::Strava => "strava",
            Stage::Op => "op",
        }
    }
}

/// Marks the operation open on the toast for the lifetime of the command.
/// `commit` before returning `Ok`; the `Drop` covers every early `?` so the
/// toast always learns the operation has finished, even on failure paths
/// that never touched a mirror.
struct OpGuard {
    app: tauri::AppHandle,
    op: &'static str,
    ok: bool,
}

impl OpGuard {
    fn new(app: &tauri::AppHandle, op: &'static str) -> Self {
        stage(app, op, Stage::Op, "working", None);
        Self {
            app: app.clone(),
            op,
            ok: false,
        }
    }

    fn commit(&mut self) {
        self.ok = true;
    }
}

impl Drop for OpGuard {
    fn drop(&mut self) {
        stage(
            &self.app,
            self.op,
            Stage::Op,
            if self.ok { "done" } else { "error" },
            None,
        );
    }
}

fn stage(app: &tauri::AppHandle, op: &str, target: Stage, state: &str, detail: Option<String>) {
    let _ = app.emit(
        "plan://stage",
        serde_json::json!({ "op": op, "target": target.target(), "state": state, "detail": detail }),
    );
}

/// The calendar mirror: refresh reports upload failures through the stored
/// config rather than the return value, so the stage reads that back before
/// calling it done. When publishing is off the refresh only regenerates the
/// local file, which is still the calendar the runner subscribes to.
async fn stage_calendar(
    app: &tauri::AppHandle,
    op: &str,
    target: Stage,
    state: &AppState,
) -> Result<()> {
    stage(app, op, target, "working", None);
    match calendar_publish::refresh(state).await {
        Err(error) => {
            stage(app, op, target, "error", Some(error.to_string()));
            Err(error)
        }
        Ok(_) => {
            let failure = calendar_publish::load(&state.data_dir)
                .ok()
                .filter(|config| config.enabled)
                .and_then(|config| config.last_error.clone());
            match failure {
                Some(detail) => stage(app, op, target, "error", Some(detail)),
                None => stage(app, op, target, "done", None),
            }
            Ok(())
        }
    }
}

/// The gist mirror, staged. Silent (no line at all) when plan sync is not
/// configured — a runner who never opened that setting sees no cloud row.
async fn stage_gist(app: &tauri::AppHandle, op: &str, target: Stage, state: &AppState) -> Result<()> {
    let before = plan_sync::load(&state.data_dir).ok().and_then(|config| config.last_pulled);
    match plan_sync::try_push_if_enabled(state).await {
        None => {}
        Some(Ok(config)) => {
            stage(app, op, target, "done", None);
            if config.last_pulled != before {
                let _ = app.emit("plan://updated", ());
                let _ = calendar_publish::refresh(state).await;
                return Err(AppError::Other("The Gist changed on another device. Its latest plan replaced the conflicting local change.".into()));
            }
        }
        Some(Err(error)) => stage(app, op, target, "error", Some(error.to_string())),
    }
    Ok(())
}

async fn prepare_plan_change(app: &tauri::AppHandle, state: &AppState) -> Result<()> {
    if plan_sync::refresh(state).await.unwrap_or(false) {
        let _ = app.emit("plan://updated", ());
        let _ = calendar_publish::refresh(state).await;
        return Err(AppError::Other("The plan changed on another device. Review the latest plan before trying this action again.".into()));
    }
    Ok(())
}

fn check_expected_plan(state: &AppState, expected: Option<&str>) -> Result<()> {
    if let Some(expected) = expected {
        if state.store.current_plan()?.as_ref().map(|(plan, _)| plan.id.as_str()) != Some(expected) {
            return Err(AppError::Other("This action belongs to a replaced plan. Review the latest plan and try again.".into()));
        }
    }
    Ok(())
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

/// Where the COROS sign-in got to. On mobile the authorization runs inside the
/// app's own webview, which destroys the page that started it, so the page that
/// loads afterwards asks this instead of awaiting the invoke that started it.
/// `Some("pending")` while a sign-in is still in flight.
#[tauri::command]
pub async fn coros_signin_notice() -> Option<String> {
    crate::providers::coros::take_auth_notice()
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
    let mut op = OpGuard::new(app, "sync");
    state.syncing.store(true, Ordering::SeqCst);
    let _ = app.emit("sync://started", ());
    if plan_sync::refresh(state).await.unwrap_or(false) {
        let _ = app.emit("plan://updated", ());
    }
    // The download streams straight into the database, so both the provider
    // read and the local write are in flight for the whole inner sync; they
    // settle together when it returns.
    stage(app, "sync", Stage::Coros, "working", None);
    stage(app, "sync", Stage::Db, "working", None);
    let mut out = run_full_sync_inner(app, state, provider).await;
    match &out {
        Ok(_) => {
            stage(app, "sync", Stage::Coros, "done", None);
            stage(app, "sync", Stage::Db, "done", None);
        }
        Err(error) => {
            stage(app, "sync", Stage::Coros, "error", Some(error.to_string()));
            stage(app, "sync", Stage::Db, "error", Some(error.to_string()));
        }
    }
    // Every sync ends by republishing the calendar: regenerate the .ics
    // (after reconciling planned vs completed runs) and upload it when
    // publishing is configured. Report the outcome in the sync summary so
    // the user can see the calendar actually moved, not just that nothing
    // crashed.
    stage(app, "sync", Stage::Calendar, "working", None);
    let calendar_message = match calendar_publish::refresh(state).await {
        Err(error) => {
            stage(app, "sync", Stage::Calendar, "error", Some(error.to_string()));
            Some(format!("calendar refresh failed: {error}"))
        }
        Ok(_) => {
            let config = calendar_publish::load(&state.data_dir).ok();
            match config.as_ref().and_then(|config| config.last_error.clone()) {
                Some(error) => {
                    stage(app, "sync", Stage::Calendar, "error", Some(error.clone()));
                    Some(format!("calendar publish failed: {error}"))
                }
                None if config.as_ref().is_some_and(|config| config.enabled) => {
                    stage(app, "sync", Stage::Calendar, "done", None);
                    Some("calendar published".into())
                }
                None => {
                    stage(app, "sync", Stage::Calendar, "done", None);
                    None
                }
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
    let strava_configured = crate::strava::is_configured().unwrap_or(true);
    if strava_configured {
        stage(app, "sync", Stage::Strava, "working", None);
    }
    match crate::strava::sync(&state.store).await {
        Ok(message) => {
            if strava_configured {
                stage(app, "sync", Stage::Strava, "done", (!message.is_empty()).then(|| message.clone()));
            }
            if !message.is_empty() {
                if let Ok(report) = &mut out { report.messages.push(message); }
            }
        }
        Err(error) => {
            stage(app, "sync", Stage::Strava, "error", Some(error.to_string()));
            if let Ok(report) = &mut out {
                report.messages.push("Strava descriptions could not finish. Check the Strava connection in Settings.".into());
            }
        }
    }
    // Publish whatever this sync produced — a reconciliation that matched a
    // session, an adaptive softening, a completion imported and re-stamped —
    // so the other device's next pull sees it. Silent when sync is off, and
    // idempotent: pushing an unchanged revision is what keeps the channel
    // honest about who is current.
    match plan_sync::try_push_if_enabled(state).await {
        None => {}
        Some(Ok(_)) => stage(app, "sync", Stage::Gist, "done", None),
        Some(Err(error)) => stage(app, "sync", Stage::Gist, "error", Some(error.to_string())),
    }
    state.syncing.store(false, Ordering::SeqCst);
    if out.is_ok() {
        op.commit();
    }
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
    // Take the channel's newer copy before reconciling, on either device: the
    // phone's edits reach the desktop this way and the desktop's reach the
    // phone, and an imported plan's workouts are matched against the
    // activities that have just synced.
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
                    .soften_workout(&a.workout_id, a.new_kind, &a.new_title, &a.reason)
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

/// The review of one recorded run — the run cut into the laps the plan asked
/// for (each with its target range) plus per-kilometre splits. Every run gets
/// one; a run with no plan session behind it falls back to per-kilometre laps
/// with no targets, which is still the "how did my pace move" view.
#[tauri::command]
pub async fn activity_review(
    state: State<'_, AppState>,
    activity_id: i64,
) -> Result<ActivityReview> {
    activity_review_for_store(&state.store, activity_id, thresholds(&state).pace_tolerance_sec_km)
}

pub(crate) fn activity_review_for_store(
    store: &crate::store::Store,
    activity_id: i64,
    tolerance: f64,
) -> Result<ActivityReview> {
    let activity = store
        .activity_by_id(activity_id)?
        .ok_or_else(|| AppError::Other(format!("activity {activity_id} not found")))?;
    let points = store.track_points(activity_id, laps::TRACK_POINTS)?;
    let completion = store.calendar_completions()?.into_iter().find(|entry| entry.1 == activity_id);
    // The plan session this run stands for, if the calendar linked them.
    let prescribed = if completion.as_ref().is_some_and(|entry| !entry.2.trim().is_empty()) {
        None
    } else {
        store.current_plan()?.and_then(|(plan, workouts)| {
            let workout_id = &completion.as_ref()?.0;
            let workout = workouts.into_iter().find(|workout| &workout.id == workout_id)?;
            Some((plan, workout))
        })
    };
    let (laps, splits) = match (&completion, &prescribed) {
        (Some((_, _, notes)), _) if !notes.trim().is_empty() => laps::build_completed(
            &points, &activity, notes, tolerance,
        ),
        (_, Some((plan, workout))) => laps::build(
            &points,
            &activity,
            Some((plan, workout)),
            tolerance,
        ),
        _ => laps::build(&points, &activity, None, tolerance),
    };
    Ok(ActivityReview {
        has_track: !points.is_empty(),
        track: points,
        workout_id: completion.as_ref().map(|entry| entry.0.clone()),
        // "Prescribed" in the UI means "there are target caps to read
        // against", which is a property of the laps, not of the lookup
        // succeeding — a fallback layout must not claim targets it lacks.
        prescribed: laps.iter().any(|l| l.band_lo_s_km.is_some()),
        activity,
        laps,
        splits,
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

/// A banked run that matched a plan session, with the session's plan title and
/// its training description, so the activities list can name the run after the
/// session it ran and say what that session was for. `purpose` is the one-line
/// "why" that fits a list row; `notes` is the full session description the
/// exported calendar event carries, which the review has room to show.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchedSession {
    pub activity_id: i64,
    pub title: String,
    pub purpose: String,
    pub notes: String,
}

#[tauri::command]
pub async fn matched_sessions(state: State<'_, AppState>) -> Result<Vec<MatchedSession>> {
    matched_sessions_for_store(&state.store)
}

pub(crate) fn matched_sessions_for_store(store: &crate::store::Store) -> Result<Vec<MatchedSession>> {
    Ok(calendar::events(store)?
        .into_iter()
        .filter_map(|event| {
            event.workout_id.as_ref()?;
            let activity_id = event.activity_id?;
            let purpose = event.notes
                .split_once("\n\nWhy:")
                .map(|(_, rest)| rest.split("\n\n").next().unwrap_or_default().trim().to_string())
                .unwrap_or_default();
            Some(MatchedSession {
                activity_id,
                title: event.title.strip_prefix("🏃 ").unwrap_or(&event.title).to_string(),
                purpose,
                notes: event.notes,
            })
        })
        .collect())
}

#[tauri::command]
pub async fn create_plan(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    request: PlanRequest,
    threshold_evidence: Option<crate::plan::threshold::Evidence>,
) -> Result<(TrainingPlan, Vec<PlanWorkout>)> {
    let _sync_guard = state.sync_lock.lock().await;
    prepare_plan_change(&app, &state).await?;
    let mut op = OpGuard::new(&app, "create");
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
    let evidence = combined_threshold_evidence(&state.store, threshold_evidence)?;
    let (plan, workouts) = engine::generate_with_evidence(&request, baseline, &evidence)?;
    stage(&app, "create", Stage::Db, "working", None);
    let stored = state
        .store
        .insert_plan(&plan)
        .and_then(|()| state.store.insert_workouts(&workouts));
    match &stored {
        Ok(()) => stage(&app, "create", Stage::Db, "done", None),
        Err(error) => stage(&app, "create", Stage::Db, "error", Some(error.to_string())),
    }
    stored?;
    stage_gist(&app, "create", Stage::Gist, &state).await?;
    stage_calendar(&app, "create", Stage::Calendar, &state).await?;
    op.commit();
    Ok((plan, workouts))
}

fn combined_threshold_evidence(
    store: &crate::store::Store,
    supplemental: Option<crate::plan::threshold::Evidence>,
) -> Result<crate::plan::threshold::Evidence> {
    let mut evidence = crate::plan::threshold::wearable_evidence(store, Utc::now().date_naive())?;
    if let Some(mut supplemental) = supplemental {
        evidence.observations.append(&mut supplemental.observations);
        evidence.sessions.append(&mut supplemental.sessions);
        evidence.performances.append(&mut supplemental.performances);
        let recorded_injury = evidence.capacity.injury_active;
        let recorded_sleep = evidence.capacity.sleep_hours;
        let recorded_km = evidence.capacity.weekly_km;
        let recorded_history = evidence.capacity.historical_weekly_km;
        let recorded_frequency = evidence.capacity.weekly_sessions;
        evidence.capacity = supplemental.capacity;
        if recorded_injury == Some(true) { evidence.capacity.injury_active = Some(true); }
        evidence.capacity.sleep_hours = match (recorded_sleep, evidence.capacity.sleep_hours) {
            (Some(recorded), Some(reported)) => Some(recorded.min(reported)),
            (recorded, reported) => recorded.or(reported),
        };
        evidence.capacity.weekly_km = recorded_km;
        evidence.capacity.historical_weekly_km = recorded_history;
        evidence.capacity.weekly_sessions = recorded_frequency;
    }
    Ok(evidence)
}

#[tauri::command]
pub async fn threshold_report(
    state: State<'_, AppState>,
    threshold_evidence: Option<crate::plan::threshold::Evidence>,
) -> Result<crate::plan::threshold::AthleteState> {
    let evidence = combined_threshold_evidence(&state.store, threshold_evidence)?;
    let pace = state.store.current_plan()?.map_or(0.0, |(plan, _)| plan.threshold_sec_km);
    Ok(crate::plan::threshold::athlete_state(&evidence, Utc::now().date_naive(), pace))
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

/// The Mondays of the current plan's recovery weeks, as `YYYY-MM-DD`.
///
/// A separate read rather than a field on the plan or on each workout: the
/// cycle is a property of the plan's schedule, and the two surfaces that label
/// it (the calendar's week header and the dashboard's week card) both want the
/// same short list of dates to test a week against.
#[tauri::command]
pub async fn down_weeks(state: State<'_, AppState>) -> Result<Vec<NaiveDate>> {
    Ok(state
        .store
        .current_plan()?
        .map(|(plan, _)| engine::down_weeks(&plan))
        .unwrap_or_default())
}

#[tauri::command]
pub async fn push_plan_to_provider(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    provider: ProviderKind,
) -> Result<String> {
    let _guard = state.sync_lock.lock().await;
    prepare_plan_change(&app, &state).await?;
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
    let mut op = OpGuard::new(&app, "push");
    stage(&app, "push", Stage::Coros, "working", None);
    let pushed = p.push_plan(&plan, &workouts, Some(&emit)).await;
    match &pushed {
        Ok(_) => stage(&app, "push", Stage::Coros, "done", None),
        Err(error) => stage(&app, "push", Stage::Coros, "error", Some(error.to_string())),
    }
    let id = pushed?;
    if let Some(id) = &id {
        state.store.set_plan_pushed_id(&plan.id, id)?;
    }
    state.store.set_plan_status(&plan.id, PlanStatus::Pushed)?;
    op.commit();
    Ok(id.unwrap_or_else(|| "scheduled".into()))
}

#[tauri::command]
pub async fn abort_plan(app: tauri::AppHandle, state: State<'_, AppState>, expected_plan_id: Option<String>) -> Result<()> {
    let _sync_guard = state.sync_lock.lock().await;
    check_expected_plan(&state, expected_plan_id.as_deref())?;
    prepare_plan_change(&app, &state).await?;
    let mut op = OpGuard::new(&app, "abort");
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
                    stage(&app, "abort", Stage::Coros, "working", None);
                    if let Err(e) = p.cancel_plan(plan.pushed_id.as_deref(), Some(&emit)).await {
                        eprintln!("[coros] plan cancel failed: {e}");
                        stage(&app, "abort", Stage::Coros, "error", Some(e.to_string()));
                        provider_note = Some(format!(
                            "the plan was removed locally, but COROS could not be updated ({e}) — \
                             quit it in the COROS app if it still shows there"
                        ));
                    } else {
                        stage(&app, "abort", Stage::Coros, "done", None);
                    }
                }
                Err(e) => eprintln!("[coros] provider unavailable for cancel: {e}"),
            }
        }
        // Delete the plan outright: cascading removes its workouts, which is
        // what actually clears them from the calendar view and the .ics file.
        plan_sync::remember_reset(&state.data_dir, &plan.id)?;
        stage(&app, "abort", Stage::Db, "working", None);
        match state.store.delete_plan(&plan.id) {
            Ok(()) => stage(&app, "abort", Stage::Db, "done", None),
            Err(error) => {
                stage(&app, "abort", Stage::Db, "error", Some(error.to_string()));
                return Err(error);
            }
        }
        stage_gist(&app, "abort", Stage::Gist, &state).await?;
        stage_calendar(&app, "abort", Stage::Calendar, &state).await?;
        op.commit();
        if let Some(note) = provider_note {
            return Err(AppError::Other(note));
        }
    }
    Ok(())
}

/// Clear every plan the local database holds — the runner's "start over"
/// button. The plan mirrors go first: the COROS slot is blanked (the closest
/// the API has to quitting, and it stops the watch showing sessions that no
/// longer exist here), each deleted plan id is recorded as reset so the sync
/// channel can never resurrect it on the next pull, and the published plan
/// gist and calendar are refreshed to show the clear.
///
/// Deliberately does NOT touch activity, health, pain or track history: that
/// data is the raw watch record, rebuilding it means re-syncing months of
/// runs, and none of it is what "the plan keeps getting overridden" is about.
#[tauri::command]
pub async fn clear_local_plan_data(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<()> {
    let _sync_guard = state.sync_lock.lock().await;
    prepare_plan_change(&app, &state).await?;
    let mut op = OpGuard::new(&app, "clear");
    // Every non-finished plan, not just the current one: a device can hold a
    // draft beside a pushed plan, and clearing must not leave the older one
    // behind to become "current" the moment this command finishes.
    let plans = state.store.all_plans()?;
    let mut provider_note = None;
    let mut coros_attempted = false;
    for plan in &plans {
        // Blank it on the watch before forgetting it, same ordering as
        // abort_plan: once the local row is gone there is no pushed id left
        // to cancel with.
        if plan.status == PlanStatus::Pushed {
            if let Ok(p) = get_provider(&state, ProviderKind::Coros) {
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
                if !coros_attempted {
                    coros_attempted = true;
                    stage(&app, "clear", Stage::Coros, "working", None);
                }
                if let Err(e) = p.cancel_plan(plan.pushed_id.as_deref(), Some(&emit)).await {
                    eprintln!("[coros] clear: plan cancel failed: {e}");
                    provider_note = Some(
                        "the plans were removed locally, but COROS could not be updated — \
                         quit them in the COROS app if they still show there"
                            .to_string(),
                    );
                }
            }
        }
        plan_sync::remember_reset(&state.data_dir, &plan.id)?;
    }
    if coros_attempted {
        match &provider_note {
            Some(note) => stage(&app, "clear", Stage::Coros, "error", Some(note.clone())),
            None => stage(&app, "clear", Stage::Coros, "done", None),
        }
    }
    stage(&app, "clear", Stage::Db, "working", None);
    match state.store.clear_plan_data() {
        Ok(()) => stage(&app, "clear", Stage::Db, "done", None),
        Err(error) => {
            stage(&app, "clear", Stage::Db, "error", Some(error.to_string()));
            return Err(error);
        }
    }
    stage_gist(&app, "clear", Stage::Gist, &state).await?;
    stage_calendar(&app, "clear", Stage::Calendar, &state).await?;
    op.commit();
    if let Some(note) = provider_note {
        return Err(AppError::Other(note));
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

// ---- plan sync -------------------------------------------------------------

#[tauri::command]
pub async fn plan_sync_status(state: State<'_, AppState>) -> Result<plan_sync::PlanSyncStatus> {
    plan_sync::status(&state)
}

#[tauri::command]
pub async fn plan_sync_configure(
    state: State<'_, AppState>,
    config: plan_sync::PlanSyncPatch,
    token: Option<String>,
) -> Result<plan_sync::PlanSyncStatus> {
    let _guard = state.sync_lock.lock().await;
    plan_sync::configure(&state, config, token)
}

/// Publish the active plan to the sync channel now.
#[tauri::command]
pub async fn plan_sync_push(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<plan_sync::PlanSync> {
    let _guard = state.sync_lock.lock().await;
    let before = plan_sync::load(&state.data_dir)?.last_pulled;
    let config = plan_sync::push(&state, true).await?;
    if before != config.last_pulled {
        let _ = app.emit("plan://updated", ());
        calendar_publish::refresh(&state).await?;
        return Err(AppError::Other("The Gist changed on another device. Its plan has been imported instead of overwriting it.".into()));
    }
    Ok(config)
}

#[tauri::command]
pub async fn plan_sync_refresh(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<bool> {
    refresh_plan_channel(&app, &state).await
}

pub async fn refresh_plan_channel(app: &tauri::AppHandle, state: &AppState) -> Result<bool> {
    let Ok(_guard) = state.sync_lock.try_lock() else { return Ok(false); };
    if plan_sync::load(&state.data_dir)?.last_checked
        .and_then(|checked| chrono::DateTime::parse_from_rfc3339(&checked).ok())
        .is_some_and(|checked| (Utc::now() - checked.with_timezone(&Utc)).num_seconds() < 2) {
        return Ok(false);
    }
    let pending = plan_sync::pending_changes(state).unwrap_or(false);
    let result = plan_sync::refresh(state).await;
    let _ = app.emit("plan://checked", ());
    let changed = result?;
    if changed {
        let _ = app.emit("plan://updated", serde_json::json!({ "discarded": pending }));
        calendar_publish::refresh(state).await?;
    }
    Ok(changed)
}

/// Import the synced plan. `force` replaces a different plan already on device.
/// The caller refreshes the views itself, as it already does for a calendar edit.
#[tauri::command]
pub async fn plan_sync_pull(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    force: bool,
) -> Result<plan_sync::PullOutcome> {
    let _guard = state.sync_lock.lock().await;
    let (_, outcome) = plan_sync::pull(&state, force).await?;
    if outcome.installed {
        let _ = app.emit("plan://updated", ());
        calendar_publish::refresh(&state).await?;
    }
    Ok(outcome)
}

/// Read what the sync channel holds without installing it, for the status
/// pill. `None` means there is nothing to say (sync off, no channel yet).
#[tauri::command]
pub async fn plan_sync_check(
    state: State<'_, AppState>,
) -> Result<Option<plan_sync::RemotePlan>> {
    plan_sync::check_remote(&state).await
}

#[tauri::command]
pub async fn match_calendar_activity(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    workout_id: String,
    activity_id: i64,
) -> Result<()> {
    let _sync_guard = state.sync_lock.lock().await;
    prepare_plan_change(&app, &state).await?;
    let mut op = OpGuard::new(&app, "match");
    stage(&app, "match", Stage::Db, "working", None);
    {
        let _guard = state.calendar_lock.lock().await;
        if let Err(error) = calendar::match_activity(&state.store, &workout_id, activity_id) {
            stage(&app, "match", Stage::Db, "error", Some(error.to_string()));
            return Err(error);
        }
    }
    stage(&app, "match", Stage::Db, "done", None);
    if let Some((plan, _)) = state.store.current_plan()? {
        if let Some(offer) = pacing::build(&state.store, &plan, &thresholds(&state))? {
            state.store.insert_proposal(&offer)?;
        }
    }
    // The pairing is a plan edit like any other: the other device has to see
    // that this session is done, and against which run.
    stage_gist(&app, "match", Stage::Gist, &state).await?;
    stage_calendar(&app, "match", Stage::Calendar, &state).await?;
    op.commit();
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
    let _sync_guard = state.sync_lock.lock().await;
    prepare_plan_change(&app, &state).await?;
    let to = crate::store::parse_date_pub(&date)?;
    let mut op = OpGuard::new(&app, "move");
    stage(&app, "move", Stage::Db, "working", None);
    let moved = {
        let _guard = state.calendar_lock.lock().await;
        calendar::move_workout(
            &state.store,
            &workout_id,
            to,
            chrono::Local::now().date_naive(),
        )
    };
    match &moved {
        Ok(_) => stage(&app, "move", Stage::Db, "done", None),
        Err(error) => stage(&app, "move", Stage::Db, "error", Some(error.to_string())),
    }
    moved?;
    let moved = after_schedule_change(&app, &state, "move").await;
    if moved.is_ok() {
        op.commit();
    }
    moved
}

#[tauri::command]
pub async fn schedule_benchmark(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    date: String,
    name: String,
    distance_m: f64,
    expected_plan_id: Option<String>,
    reduce_mileage: Option<bool>,
) -> Result<()> {
    let _sync_guard = state.sync_lock.lock().await;
    check_expected_plan(&state, expected_plan_id.as_deref())?;
    prepare_plan_change(&app, &state).await?;
    let mut op = OpGuard::new(&app, "benchmark");
    stage(&app, "benchmark", Stage::Db, "working", None);
    let scheduled = {
        let _guard = state.calendar_lock.lock().await;
        calendar::schedule_benchmark(&state.store, crate::store::parse_date_pub(&date)?,
            chrono::Local::now().date_naive(), &name, distance_m, reduce_mileage.unwrap_or(true))
    };
    match &scheduled {
        Ok(_) => stage(&app, "benchmark", Stage::Db, "done", None),
        Err(error) => stage(&app, "benchmark", Stage::Db, "error", Some(error.to_string())),
    }
    scheduled?;
    let done = after_schedule_change(&app, &state, "benchmark").await;
    if done.is_ok() {
        op.commit();
    }
    done
}

/// What a "reset this week" preview shows for one drifted session: where it
/// sits now and the day the plan generated it for. A reset that silently moved
/// four runs is the same surprise as a bad drag, so the sheet names them first.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekDeviation {
    pub workout_id: String,
    pub title: String,
    /// Where the session sits now.
    pub from: String,
    /// The day the plan generated it for — where a reset would put it.
    pub to: String,
    /// A session already run can never be restored, and a reset refuses
    /// it — the preview says so rather than promising a move it will skip.
    pub restorable: bool,
    /// Restoring this one would land it on today, which is the case a runner
    /// most wants to know before agreeing: it changes what they are doing now.
    pub is_today: bool,
    /// The reset moves this session's day back onto its generated day.
    pub restores_date: bool,
    /// A benchmark overwrote what the plan generated here, and the reset puts
    /// the planned session back.
    pub restores_content: bool,
    /// The benchmark created this session on a rest day, so putting the plan
    /// back removes it rather than rewriting it.
    pub removes: bool,
}

/// One week of the current plan: Monday, metres prescribed, metres banked.
/// The plan roadmap marks each week from these numbers rather than from the
/// sessions' `done` flags alone, so "done" means the mileage actually landed.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekProgress {
    pub week_start: String,
    pub planned_m: f64,
    pub ran_m: f64,
}

/// Per-week planned vs ran mileage of the current plan, oldest week first.
#[tauri::command]
pub async fn plan_week_progress(
    state: State<'_, AppState>,
) -> Result<Vec<WeekProgress>> {
    Ok(state
        .store
        .plan_week_progress()?
        .into_iter()
        .map(|(week_start, planned_m, ran_m)| WeekProgress {
            week_start,
            planned_m,
            ran_m,
        })
        .collect())
}

/// How many sessions a "reset this week" would put back, and from which days.
#[tauri::command]
pub async fn week_deviations(
    state: State<'_, AppState>,
    week_start: String,
) -> Result<Vec<WeekDeviation>> {
    let monday = crate::store::parse_date_pub(&week_start)?;
    let today = chrono::Local::now().date_naive();
    Ok(calendar::week_deviations(&state.store, monday, today)?
        .into_iter()
        .map(|drift| WeekDeviation {
            restorable: drift.restorable(),
            workout_id: drift.workout.id,
            title: drift.workout.title,
            from: drift.workout.date.format("%Y-%m-%d").to_string(),
            to: drift.planned.format("%Y-%m-%d").to_string(),
            // A session already run, or whose generated day has already passed,
            // can never be restored — the preview says so rather than promising
            // a move the reset will skip.
            is_today: drift.planned == today,
            restores_date: drift.restores_date,
            restores_content: drift.original.is_some() && !drift.removes,
            removes: drift.removes,
        })
        .collect())
}

/// Put one ISO week back onto the days the plan generated it for. The runner
/// changed their mind, the plan's own layout is still the better answer, and
/// without this the only way back is to regenerate the whole plan — which would
/// also throw away every other week's edits.
#[tauri::command]
pub async fn reset_week(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    week_start: String,
    expected_plan_id: Option<String>,
) -> Result<usize> {
    let _sync_guard = state.sync_lock.lock().await;
    check_expected_plan(&state, expected_plan_id.as_deref())?;
    prepare_plan_change(&app, &state).await?;
    let monday = crate::store::parse_date_pub(&week_start)?;
    let mut op = OpGuard::new(&app, "reset");
    stage(&app, "reset", Stage::Db, "working", None);
    let restored = {
        let _guard = state.calendar_lock.lock().await;
        calendar::reset_week(&state.store, monday, chrono::Local::now().date_naive())
    };
    match &restored {
        Ok(_) => stage(&app, "reset", Stage::Db, "done", None),
        Err(error) => stage(&app, "reset", Stage::Db, "error", Some(error.to_string())),
    };
    let restored = restored?;
    after_schedule_change(&app, &state, "reset").await?;
    op.commit();
    Ok(restored)
}

/// Mark a planned session as deliberately skipped, or undo that. The reason it
/// is a status rather than a deletion: a skipped session must stop counting
/// against adherence while still showing the runner what they decided.
#[tauri::command]
pub async fn set_workout_skipped(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    workout_id: String,
    skipped: bool,
) -> Result<()> {
    let _sync_guard = state.sync_lock.lock().await;
    prepare_plan_change(&app, &state).await?;
    let mut op = OpGuard::new(&app, "skip");
    stage(&app, "skip", Stage::Db, "working", None);
    let set = {
        let _guard = state.calendar_lock.lock().await;
        calendar::set_skipped(&state.store, &workout_id, skipped)
    };
    match &set {
        Ok(_) => stage(&app, "skip", Stage::Db, "done", None),
        Err(error) => stage(&app, "skip", Stage::Db, "error", Some(error.to_string())),
    }
    set?;
    let done = after_schedule_change(&app, &state, "skip").await;
    if done.is_ok() {
        op.commit();
    }
    done
}

/// Everything that has to happen after the plan's *days* change, in one place:
/// the calendar file and its feed, a pushed plan re-sent to the phone, and a
/// plan already on the watch re-pushed off-thread.
///
/// The re-push is spawned rather than awaited because a COROS round-trip takes
/// tens of seconds and the UI has nothing to show while it waits — the calendar
/// above is already correct, and the watch catches up. The stage toast is the
/// exception: its COROS line stays open until the re-push settles, which is
/// exactly what the runner wants to see while the watch lags behind.
async fn after_schedule_change(
    app: &tauri::AppHandle,
    state: &State<'_, AppState>,
    op: &str,
) -> Result<()> {
    stage_gist(app, op, Stage::Gist, state).await?;
    stage_calendar(app, op, Stage::Calendar, state).await?;
    if let Some((plan, _)) = state.store.current_plan()? {
        if plan.status == PlanStatus::Pushed {
            let app = app.clone();
            let op = op.to_string();
            stage(&app, &op, Stage::Coros, "working", None);
            tauri::async_runtime::spawn(async move {
                use tauri::Manager;
                let state = app.state::<AppState>();
                let _guard = state.sync_lock.lock().await;
                let Some((latest, workouts)) = state.store.current_plan().ok().flatten() else {
                    stage(&app, &op, Stage::Coros, "done", None);
                    return;
                };
                if latest.id != plan.id {
                    stage(&app, &op, Stage::Coros, "done", None);
                    return;
                }
                let plan = latest;
                let result = match get_provider(&state, ProviderKind::Coros) {
                    Ok(p) => p.push_plan(&plan, &workouts, None).await,
                    Err(e) => Err(e),
                };
                match &result {
                    Ok(_) => stage(&app, &op, Stage::Coros, "done", None),
                    Err(e) => {
                        eprintln!("[coros] re-push after schedule change failed: {e}");
                        stage(&app, &op, Stage::Coros, "error", Some(e.to_string()));
                    }
                }
                if let Ok(Some(id)) = &result {
                    let _ = state.store.set_plan_pushed_id(&plan.id, id);
                }
            });
        }
    }
    Ok(())
}

// ---- settings ---------------------------------------------------------------

#[tauri::command]
pub async fn session_feedback(state: State<'_, AppState>) -> Result<Option<crate::plan::difficulty::FeedbackContext>> {
    let _guard = state.sync_lock.lock().await;
    crate::plan::difficulty::context(&state.store)
}

#[tauri::command]
pub async fn save_session_feedback(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    plan_id: String,
    revision: i64,
    workout_id: String,
    rating: Option<u8>,
    dismiss: bool,
) -> Result<crate::plan::difficulty::Outcome> {
    let _guard = state.sync_lock.lock().await;
    prepare_plan_change(&app, &state).await?;
    let outcome = crate::plan::difficulty::apply(&state.store, &thresholds(&state), &plan_id, revision, None, Some((&workout_id, rating, dismiss)))?;
    let _ = app.emit("plan://updated", serde_json::json!({ "local": true }));
    if after_schedule_change(&app, &state, "feedback").await.is_err() {
        let _ = app.emit("plan://stage", serde_json::json!({ "op":"feedback", "target":"op", "state":"error", "detail":"Saved locally; publishing will retry on the next sync." }));
    }
    Ok(outcome)
}

#[tauri::command]
pub async fn adjust_plan_difficulty(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    plan_id: String,
    revision: i64,
    direction: crate::plan::difficulty::Direction,
) -> Result<crate::plan::difficulty::Outcome> {
    let _guard = state.sync_lock.lock().await;
    prepare_plan_change(&app, &state).await?;
    let outcome = crate::plan::difficulty::apply(&state.store, &thresholds(&state), &plan_id, revision, Some(direction), None)?;
    let _ = app.emit("plan://updated", serde_json::json!({ "local": true }));
    if after_schedule_change(&app, &state, "difficulty").await.is_err() {
        let _ = app.emit("plan://stage", serde_json::json!({ "op":"difficulty", "target":"op", "state":"error", "detail":"Saved locally; publishing will retry on the next sync." }));
    }
    Ok(outcome)
}

#[tauri::command]
pub async fn undo_plan_difficulty(app: tauri::AppHandle, state: State<'_, AppState>, plan_id: String) -> Result<()> {
    let _guard = state.sync_lock.lock().await;
    prepare_plan_change(&app, &state).await?;
    state.store.undo_difficulty(&plan_id)?;
    let _ = app.emit("plan://updated", serde_json::json!({ "local": true }));
    let _ = after_schedule_change(&app, &state, "difficulty").await;
    Ok(())
}

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
    app: &tauri::AppHandle,
    op: &str,
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
            planned_date: w.planned_date,
            kind: e.kind,
            title: e.title.clone(),
            distance_m: e.distance_m,
            duration_min: e.duration_min,
            pace_lo_s_km: e.pace_lo_s_km,
            pace_hi_s_km: e.pace_hi_s_km,
            done: w.done,
            status: w.status,
            moved_at: w.moved_at,
            customization: if e.kind == w.kind && e.distance_m == w.distance_m
                && e.pace_lo_s_km == w.pace_lo_s_km && e.pace_hi_s_km == w.pace_hi_s_km {
                w.customization.clone()
            } else { None },
        })?;
    }
    let fresh = state.store.plan_workouts(&plan.id)?;
    stage_gist(app, op, Stage::Gist, state).await?;
    stage_calendar(app, op, Stage::Calendar, state).await?;
    // Keep the watch schedule mirroring the local plan, as the adaptive path does.
    if let (Some(p), true) = (provider, plan.status == PlanStatus::Pushed) {
        stage(app, op, Stage::Coros, "working", None);
        match p.push_plan(&plan, &fresh, None).await {
            Ok(id) => {
                stage(app, op, Stage::Coros, "done", None);
                if let Some(id) = id {
                    let _ = state.store.set_plan_pushed_id(&plan.id, &id);
                }
            }
            Err(error) => stage(app, op, Stage::Coros, "error", Some(error.to_string())),
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
    let _guard = state.sync_lock.lock().await;
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
/// pacing offers atomically commit the quality-pace offset, bounded future
/// distance edits and decision status. Both then refresh external mirrors.
#[tauri::command]
pub async fn apply_proposal(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    provider: ProviderKind,
    proposal_id: String,
) -> Result<Vec<PlanWorkout>> {
    let _sync_guard = state.sync_lock.lock().await;
    prepare_plan_change(&app, &state).await?;
    let mut op = OpGuard::new(&app, "proposal");
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
        if state.store.plan_workouts(&plan.id)?.iter().any(|workout| workout.customization.as_ref().is_some_and(|value| value.prescription.is_some())) {
            return Err(AppError::Other("This pacing offer predates your difficulty adjustment. Use Edit plan instead.".into()));
        }
        if !p.signature.ends_with(&format!(":{}", plan.id))
            || from < Utc::now().date_naive()
            || !offset.is_finite()
            || ((offset - plan.quality_offset_sec_km).abs() - 5.0).abs() > 0.001
            || offset.abs() > 20.0
        {
            return Err(AppError::Other("this pacing offer is stale; dismiss it and request a fresh review".into()));
        }
        let evidence = pacing::adaptation_reviews(&state.store, &plan, &thresholds(&state))?;
        let (count, direction) = pacing::streak(&evidence);
        let expected_direction = if offset < plan.quality_offset_sec_km { Execution::Ahead } else { Execution::Behind };
        let benchmark = evidence.last().filter(|review| review.kind == WorkoutKind::Race);
        let benchmark_signature = benchmark.map(|review| format!("pacing:benchmark:{}:{}:{}", review.workout_id,
            if expected_direction == Execution::Ahead { "ahead" } else { "behind" }, plan.id));
        if (count < thresholds(&state).pace_streak as usize && benchmark.is_none())
            || direction != Some(expected_direction)
            || (p.signature.starts_with("pacing:benchmark:") && benchmark_signature.as_deref() != Some(p.signature.as_str())) {
            return Err(AppError::Other("recent reps no longer support this offer; dismiss it and review the latest evidence".into()));
        }
        let expected = pacing::adaptation_edits(&state.store, &plan, offset, from)?;
        if !p.edits.is_empty() && serde_json::to_value(&expected)? != serde_json::to_value(&p.edits)? {
            return Err(AppError::Other("the proposed sessions changed; dismiss this offer and review again".into()));
        }
        if offset < plan.quality_offset_sec_km {
            let scores = metrics::compute_scores(&state.store, &thresholds(&state))?;
            let ratio = state.store.latest_training_load()?.and_then(|load| load.ratio);
            if state.store.latest_pain()?.is_some() || scores.recovery_score <= thresholds(&state).recovery_fatigued
                || scores.injury_risk >= 60 || !ratio.is_some_and(|value| value.is_finite() && value <= 1.3)
            {
                return Err(AppError::Other("recovery or load no longer supports progression; keep the current plan".into()));
            }
        }
        state.store.accept_pacing_proposal(&plan, &p)?;
        // The stored plan moved; the mirrors below must be built from the
        // updated plan, not the copy loaded before the write.
        let plan = TrainingPlan {
            quality_offset_sec_km: offset,
            quality_offset_from: Some(from),
            ..plan
        };
        stage_gist(&app, "proposal", Stage::Gist, &state).await?;
        stage_calendar(&app, "proposal", Stage::Calendar, &state).await?;
        let fresh = state.store.plan_workouts(&plan.id)?;
        if let (Some(p), true) = (prov, plan.status == PlanStatus::Pushed) {
            stage(&app, "proposal", Stage::Coros, "working", None);
            match p.push_plan(&plan, &fresh, None).await {
                Ok(id) => {
                    stage(&app, "proposal", Stage::Coros, "done", None);
                    if let Some(id) = id {
                        let _ = state.store.set_plan_pushed_id(&plan.id, &id);
                    }
                }
                Err(error) => stage(&app, "proposal", Stage::Coros, "error", Some(error.to_string())),
            }
        }
        fresh
    } else {
        apply_workout_edits(&app, "proposal", &state, prov, &p.edits).await?
    };
    if p.source != ProposalSource::Pacing {
        state.store.set_proposal_status(&p.id, ProposalStatus::Applied, Some(Utc::now()), None)?;
    }
    op.commit();
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

#[tauri::command]
pub async fn pace_insights(state: State<'_, AppState>) -> Result<Option<PaceInsights>> {
    let Some((plan, _)) = state.store.current_plan()? else {
        return Ok(None);
    };
    Ok(Some(pacing::insights(&state.store, &plan)?))
}

/// The verdict on one session, for the sheet that opens it. `None` when there
/// is nothing honest to say about it — not a quality session, no run attached,
/// no track yet, or run before the plan's current ranges took effect.
#[tauri::command]
pub async fn session_review(
    state: State<'_, AppState>,
    workout_id: String,
) -> Result<Option<SessionReview>> {
    let Some((plan, _)) = state.store.current_plan()? else {
        return Ok(None);
    };
    pacing::review_for_workout(&state.store, &plan, &workout_id, &thresholds(&state))
}

/// What the current plan was built from, said back as ordered input/sentence
/// pairs. `None` when there is no plan. Rendering only — nothing here is
/// re-derived from the model, so the receipt cannot drift from the plan.
#[tauri::command]
pub async fn plan_receipt(state: State<'_, AppState>) -> Result<Option<Vec<ReceiptLine>>> {
    let Some((plan, _)) = state.store.current_plan()? else {
        return Ok(None);
    };
    let workouts = state.store.plan_workouts(&plan.id)?;
    Ok(Some(receipt::receipt(&plan, &workouts)))
}

// ---- injury assistant -----------------------------------------------------------

/// Keyring slot for the assistant API key — separate service from COROS OAuth.
const LLM_KEYRING_SERVICE: &str = "com.fradone.runalytics.llm";
const LLM_KEYRING_USER: &str = "api_key";

fn llm_key_entry() -> Result<keyring_core::Entry> {
    crate::secure_store::entry(LLM_KEYRING_SERVICE, LLM_KEYRING_USER)
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
        Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
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
