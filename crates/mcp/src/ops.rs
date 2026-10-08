//! Transport-free tool implementations.
//!
//! Every function here takes a [`Context`] and plain arguments and returns a
//! JSON value — no MCP types, no transport, no async. The rmcp layer in
//! [`crate::server`] is a thin shell over these, which is what makes the whole
//! tool surface testable without an MCP client and keeps the desktop app able
//! to call the same operations in-process.
//!
//! Errors are [`OpsError`], rendered to tool callers as structured error
//! results. Messages are written to be *read by an agent*: they name the
//! missing thing and, where useful, what to call instead.

use std::path::PathBuf;

use chrono::{TimeDelta, Utc};
use runalytics_calendar::{ApplescriptSink, FileSink, SinkRegistry, ics_for_plan};
use runalytics_core::{
    Activity, ActivityId, AthleteSnapshot, Date, DomainError, HealthDay, Pace, Plan, PlanId,
    PlanRequest, PlanStatus, PlannedSession, PlannedSessionId, TimeOfDay, Timestamp, Tz,
};
use runalytics_scoring::{
    InjuryRisk, Readiness, ReadinessBand, SCORING_VERSION, SessionFeedback, SessionQuality,
    acute_chronic_ratio, acute_load, chronic_load, daily_loads, injury_risk_for, intensity_factor,
    monotony, performance_index, readiness_for, score_session, training_stress_score, weekly_spike,
};
use runalytics_store::{
    ActivityRepo, Db, FeedbackRepo, FitnessRepo, HealthRepo, PlanRepo, ProviderAccountRepo,
    ReadinessRepo, ScoreRepo, StoreError, SyncRunRepo,
    repos::{FeedbackRow, InjuryRiskRow, MetricSnapshotRow, ReadinessRow, SessionScoreRow},
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

/// Why an operation failed, in agent-readable form.
#[derive(Debug, thiserror::Error)]
pub enum OpsError {
    /// The call was well-formed but the inputs do not work: missing plan,
    /// unparseable date, unknown id, no data for the day.
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error("calendar publish failed: {0}")]
    Calendar(String),
}

impl OpsError {
    fn invalid(args: impl std::fmt::Display) -> Self {
        Self::Invalid(args.to_string())
    }
}

/// Static configuration the server was started with.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// SQLite file; `None` means an ephemeral in-memory database (tests).
    pub db_path: Option<PathBuf>,
    /// The athlete's IANA time zone; drives "today" and calendar times.
    pub timezone: Tz,
    /// Calendar.app calendar the AppleScript sink writes into.
    pub calendar_name: String,
    /// Optional path where the ICS feed is also written for webcal clients.
    pub ics_path: Option<PathBuf>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            db_path: None,
            timezone: "Europe/Berlin"
                .parse()
                .expect("static time zone must parse"),
            calendar_name: "Runalytics".to_string(),
            ics_path: None,
        }
    }
}

/// Everything a tool needs: the shared database plus launch configuration.
/// (`Db` is not `Debug` — it wraps a locked connection — so this is not either.)
#[derive(Clone)]
pub struct Context {
    /// Shared WAL-mode connection; cheap to clone.
    pub db: Db,
    /// Launch configuration.
    pub config: ServerConfig,
}

impl Context {
    /// Open (or create) the database described by `config`.
    ///
    /// # Errors
    /// Returns [`OpsError::Store`] when the file cannot be opened or migrated.
    pub fn open(config: ServerConfig) -> Result<Self, OpsError> {
        let db = match &config.db_path {
            Some(path) => Db::open(path)?,
            None => Db::in_memory()?,
        };
        Ok(Self { db, config })
    }

    /// Today as the athlete experiences it.
    #[must_use]
    pub fn today(&self) -> Date {
        Utc::now().with_timezone(&self.config.timezone).date_naive()
    }

    /// Now, in UTC.
    #[must_use]
    pub fn now(&self) -> Timestamp {
        Utc::now()
    }
}

/// The athlete to score against: the active plan's snapshot when there is
/// one, a conservative placeholder otherwise.
fn athlete_for(ctx: &Context, plan: Option<&Plan>) -> AthleteSnapshot {
    plan.map_or_else(
        || AthleteSnapshot::placeholder(ctx.config.timezone),
        |p| p.athlete.clone(),
    )
}

/// Threshold pace for load math: provider-reported, else VO2max-derived,
/// else the domain default. Never guesses from nothing.
fn threshold_for(ctx: &Context) -> Pace {
    FitnessRepo::latest(&ctx.db)
        .ok()
        .flatten()
        .and_then(|f| {
            f.threshold_pace
                .or_else(|| f.vo2max.map(runalytics_scoring::threshold_from_vo2max))
        })
        .unwrap_or_default()
}

fn parse_date(s: &str) -> Result<Date, OpsError> {
    Date::parse_from_str(s, "%Y-%m-%d").map_err(|e| OpsError::invalid(format!("date '{s}': {e}")))
}

fn parse_time(s: &str) -> Result<TimeOfDay, OpsError> {
    TimeOfDay::parse_from_str(s, "%H:%M").map_err(|e| OpsError::invalid(format!("time '{s}': {e}")))
}

fn parse_plan_id(s: &str) -> Result<PlanId, OpsError> {
    s.parse()
        .map_err(|_| OpsError::invalid(format!("planId '{s}' is not a UUID")))
}

fn parse_session_id(s: &str) -> Result<PlannedSessionId, OpsError> {
    s.parse()
        .map_err(|_| OpsError::invalid(format!("sessionId '{s}' is not a UUID")))
}

fn parse_activity_id(s: &str) -> Result<ActivityId, OpsError> {
    s.parse()
        .map_err(|_| OpsError::invalid(format!("activityId '{s}' is not a UUID")))
}

/// The active plan, or a message that names the tool to fix it with.
fn active_plan(ctx: &Context) -> Result<Plan, OpsError> {
    PlanRepo::active(&ctx.db)?.ok_or_else(|| {
        OpsError::invalid("no active plan — generate one with generate_plan, then activate_plan")
    })
}

// ---------------------------------------------------------------------------
// Plan tools
// ---------------------------------------------------------------------------

/// Input for [`generate_plan`].
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GeneratePlanIn {
    /// A full PlanRequest object: `{goal, anchor, athlete, constraints,
    /// startDate}`. `goal` is one of marathon, half_marathon, ten_k, five_k,
    /// recovery, maintain. `anchor` is `{"kind":"horizon","value":{"weeks":8}}`
    /// or `{"kind":"raceDate","value":{"date":"2026-10-12"}}`. `athlete` is a
    /// full AthleteSnapshot object (camelCase); see get_status for a starting
    /// point.
    pub request: Value,
    /// Persist the plan (draft) and return its id. Default true.
    #[serde(default = "default_true")]
    pub save: bool,
}

fn default_true() -> bool {
    true
}

/// Generate a deterministic plan from a request and persist it as a draft.
///
/// # Errors
/// [`OpsError::Invalid`] for an unparseable request, [`OpsError::Domain`]
/// when the anchor is unusable, [`OpsError::Store`] on persistence failure.
pub fn generate_plan(ctx: &Context, input: GeneratePlanIn) -> Result<Value, OpsError> {
    let request: PlanRequest = serde_json::from_value(input.request)
        .map_err(|e| OpsError::invalid(format!("request is not a PlanRequest: {e}")))?;
    let generated = runalytics_plan_engine::generate(&request, ctx.today(), ctx.now())?;
    if input.save {
        PlanRepo::save(&ctx.db, &generated.plan)?;
    }
    Ok(json!({
        "planId": generated.plan.id.to_string(),
        "name": generated.plan.name,
        "status": generated.plan.status.as_str(),
        "resolution": generated.plan.resolution,
        "weeks": generated.plan.weeks.iter().map(|w| json!({
            "index": w.index,
            "phase": w.phase.as_str(),
            "start": w.start.format("%Y-%m-%d").to_string(),
            "targetVolumeKm": w.target_volume.as_f64(),
            "sessions": w.sessions.iter().map(session_json).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "notes": generated.notes.iter().map(|n| json!({"code": n.code, "message": n.message})).collect::<Vec<_>>(),
        "saved": input.save,
    }))
}

fn session_json(s: &PlannedSession) -> Value {
    json!({
        "id": s.id.to_string(),
        "date": s.date.format("%Y-%m-%d").to_string(),
        "start": s.start.format("%H:%M").to_string(),
        "kind": s.kind.as_str(),
        "title": s.title,
        "intent": s.intent,
        "targetVolumeKm": s.target_volume.as_f64(),
        "targetDurationSecs": s.target_duration.as_u32(),
        "targetPaceSecsPerKm": s.target_pace.map(Pace::as_secs_per_km),
        "quality": s.quality,
    })
}

/// Input for [`get_plan`].
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetPlanIn {
    /// Plan id; defaults to the active plan.
    #[serde(default)]
    pub plan_id: Option<String>,
}

/// Return a stored plan in full (active by default).
///
/// # Errors
/// [`OpsError::Store`] when the id is unknown, [`OpsError::Invalid`] when no
/// plan id was given and no plan is active.
pub fn get_plan(ctx: &Context, input: GetPlanIn) -> Result<Value, OpsError> {
    let plan = match input.plan_id {
        Some(id) => PlanRepo::load(&ctx.db, parse_plan_id(&id)?)?,
        None => active_plan(ctx)?,
    };
    Ok(json!({ "plan": plan }))
}

/// Input for [`list_plans`].
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListPlansIn {}

/// A complete, valid PlanRequest skeleton with placeholder values.
///
/// # Errors
/// Never fails; kept uniform with the other ops.
pub fn plan_request_template(_ctx: &Context) -> Result<Value, OpsError> {
    let athlete = AthleteSnapshot::placeholder("Europe/Berlin".parse().expect("static tz parses"));
    let request = PlanRequest {
        goal: runalytics_core::GoalKind::HalfMarathon,
        anchor: runalytics_core::Anchor::Horizon { weeks: 8 },
        athlete,
        constraints: runalytics_core::PlanConstraints::default(),
        start_date: None,
    };
    Ok(serde_json::to_value(request).unwrap_or(Value::Null))
}

/// List every stored plan, newest first.
///
/// # Errors
/// [`OpsError::Store`] on a database failure.
pub fn list_plans(ctx: &Context, _input: ListPlansIn) -> Result<Value, OpsError> {
    let summaries = PlanRepo::list(&ctx.db)?;
    Ok(json!({ "plans": summaries }))
}

/// Input for [`activate_plan`].
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActivatePlanIn {
    /// The plan to make active.
    pub plan_id: String,
}

/// Make a plan active; pause whichever plan was active before.
///
/// # Errors
/// [`OpsError::Store`] for an unknown id, [`OpsError::Invalid`] for a plan
/// whose status cannot transition (completed/archived).
pub fn activate_plan(ctx: &Context, input: &ActivatePlanIn) -> Result<Value, OpsError> {
    let id = parse_plan_id(&input.plan_id)?;
    let target = PlanRepo::load(&ctx.db, id)?;
    if !target.status.is_mutable() {
        return Err(OpsError::invalid(format!(
            "plan {id} is {} and can no longer be activated",
            target.status.as_str()
        )));
    }
    let previous = PlanRepo::active(&ctx.db)?;
    PlanRepo::set_status(&ctx.db, id, PlanStatus::Active)?;
    let mut paused = Vec::new();
    if let Some(old) = previous.filter(|p| p.id != id) {
        PlanRepo::set_status(&ctx.db, old.id, PlanStatus::Paused)?;
        paused.push(json!({"planId": old.id.to_string(), "name": old.name}));
    }
    Ok(json!({
        "activePlanId": id.to_string(),
        "name": target.name,
        "pausedPrevious": paused,
    }))
}

/// Input for [`update_session`].
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSessionIn {
    /// The session to change.
    pub session_id: String,
    /// New calendar day, `YYYY-MM-DD`.
    #[serde(default)]
    pub date: Option<String>,
    /// New start time, `HH:MM` (athlete-local).
    #[serde(default)]
    pub start: Option<String>,
    /// New session kind (easy, long_run, tempo, intervals, rest, ...).
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub intent: Option<String>,
    #[serde(default)]
    pub target_volume_km: Option<f64>,
    #[serde(default)]
    pub target_duration_min: Option<u32>,
    /// Prescribed pace in seconds per km (e.g. 255 = 4:15/km).
    #[serde(default)]
    pub target_pace_secs_per_km: Option<f64>,
    #[serde(default)]
    pub rpe_target: Option<u8>,
}

/// Apply a partial update to one planned session.
///
/// This is the calendar-plan mutation skill wants: move a day, swap a tempo
/// for intervals, mark a rest day. The session keeps its id, so calendar
/// entries keyed by `runalytics://session/{id}` follow the move on the next
/// [`sync_calendar`].
///
/// # Errors
/// [`OpsError::Invalid`] for bad ids/values, [`OpsError::Store`] when the
/// session does not exist or its plan is immutable.
pub fn update_session(ctx: &Context, input: UpdateSessionIn) -> Result<Value, OpsError> {
    let session_id = parse_session_id(&input.session_id)?;
    let plan_id = PlanRepo::session_plan_id(&ctx.db, session_id)?;
    let mut plan = PlanRepo::load(&ctx.db, plan_id)?;
    if !plan.status.is_mutable() {
        return Err(OpsError::invalid(format!(
            "plan {} is {} — activate a draft to edit it",
            plan.id,
            plan.status.as_str()
        )));
    }
    let session = plan
        .session_by_id_mut(session_id)
        .ok_or_else(|| OpsError::invalid("session vanished from its plan"))?;
    if let Some(date) = &input.date {
        session.date = parse_date(date)?;
    }
    if let Some(start) = &input.start {
        session.start = parse_time(start)?;
    }
    if let Some(kind) = &input.kind {
        session.kind = kind
            .parse()
            .map_err(|e: String| OpsError::invalid(format!("kind '{kind}': {e}")))?;
    }
    if let Some(title) = input.title {
        session.title = title;
    }
    if let Some(intent) = input.intent {
        session.intent = intent;
    }
    if let Some(km) = input.target_volume_km {
        session.target_volume = runalytics_core::VolumeKm::new(km);
    }
    if let Some(mins) = input.target_duration_min {
        session.target_duration = runalytics_core::DurationSecs::from_minutes(mins);
    }
    if let Some(pace) = input.target_pace_secs_per_km {
        session.target_pace = Some(Pace::new(pace));
    }
    if let Some(rpe) = input.rpe_target {
        session.rpe_target = Some(rpe);
    }
    let updated = session.clone();
    PlanRepo::update_session(&ctx.db, &updated)?;
    Ok(json!({ "session": session_json(&updated) }))
}

/// Input for [`upcoming_sessions`].
#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpcomingSessionsIn {
    /// First day to include; defaults to today.
    #[serde(default)]
    pub from: Option<String>,
    /// How many days ahead to look. Default 7.
    #[serde(default)]
    pub days: Option<i64>,
}

/// List scheduled sessions in an upcoming window.
///
/// # Errors
/// [`OpsError::Invalid`] for a bad date, [`OpsError::Store`] on failure.
pub fn upcoming_sessions(ctx: &Context, input: &UpcomingSessionsIn) -> Result<Value, OpsError> {
    let from = input
        .from
        .as_deref()
        .map(parse_date)
        .transpose()?
        .unwrap_or_else(|| ctx.today());
    let days = input.days.unwrap_or(7).clamp(1, 90);
    let until = from + TimeDelta::try_days(days).unwrap_or(TimeDelta::days(7));
    let sessions = PlanRepo::upcoming_sessions(&ctx.db, from)?
        .into_iter()
        .filter(|s| s.date <= until)
        .collect::<Vec<_>>();
    Ok(json!({
        "from": from.format("%Y-%m-%d").to_string(),
        "until": until.format("%Y-%m-%d").to_string(),
        "sessions": sessions.iter().map(session_json).collect::<Vec<_>>(),
    }))
}

// ---------------------------------------------------------------------------
// Calendar tools
// ---------------------------------------------------------------------------

/// Input for [`sync_calendar`].
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SyncCalendarIn {}

/// Publish the active plan to the configured calendar sinks.
///
/// Sinks: Calendar.app via AppleScript, plus the ICS file when the server was
/// started with `--ics-path`. Publishing is idempotent — sessions are keyed
/// by their stable `runalytics://session/{id}` marker, so a re-sync updates
/// moved sessions instead of duplicating them.
///
/// # Errors
/// [`OpsError::Invalid`] when no plan is active, [`OpsError::Calendar`] when
/// every sink failed (partial success is reported, not an error).
pub async fn sync_calendar(ctx: &Context, _input: SyncCalendarIn) -> Result<Value, OpsError> {
    let plan = active_plan(ctx)?;
    let mut registry = SinkRegistry::new();
    registry.register(Box::new(ApplescriptSink::new(
        ctx.config.calendar_name.clone(),
    )));
    if let Some(path) = &ctx.config.ics_path {
        registry.register(Box::new(FileSink::new(path.clone())));
    }
    let report = registry.publish(&plan, &ctx.config.timezone).await;
    if report.ok.is_empty() {
        return Err(OpsError::Calendar(format!(
            "every sink failed: {}",
            report.failed.join(", ")
        )));
    }
    Ok(json!({
        "planId": plan.id.to_string(),
        "publishedTo": report.ok,
        "failedSinks": report.failed,
        "sessions": plan.all_sessions().filter(|s| s.kind.is_schedulable()).count(),
    }))
}

/// Input for [`get_calendar_feed`].
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetCalendarFeedIn {}

/// Return the raw ICS text for the active plan.
///
/// # Errors
/// [`OpsError::Invalid`] when no plan is active.
pub fn get_calendar_feed(ctx: &Context, _input: GetCalendarFeedIn) -> Result<Value, OpsError> {
    let plan = active_plan(ctx)?;
    let ics = ics_for_plan(&plan, &ctx.config.timezone, ctx.now());
    Ok(json!({
        "planId": plan.id.to_string(),
        "calendarName": ctx.config.calendar_name,
        "ics": ics,
    }))
}

// ---------------------------------------------------------------------------
// Status and scoring tools
// ---------------------------------------------------------------------------

/// Input for [`get_status`].
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetStatusIn {}

/// Today's dashboard as JSON: plan, load, readiness, injury risk, accounts.
///
/// # Errors
/// [`OpsError::Store`] on a database failure.
pub fn get_status(ctx: &Context, _input: GetStatusIn) -> Result<Value, OpsError> {
    let today = ctx.today();
    let plan = PlanRepo::active(&ctx.db)?;
    let metrics = ScoreRepo::latest_metric_snapshot(&ctx.db)?;
    let readiness = ReadinessRepo::readiness(&ctx.db, today)?;
    let injury = ReadinessRepo::injury_risk(&ctx.db, today)?;
    let accounts = ProviderAccountRepo::list(&ctx.db)?;
    let sync_runs = SyncRunRepo::recent(&ctx.db, 5)?;
    let next = PlanRepo::upcoming_sessions(&ctx.db, today)?
        .into_iter()
        .find(|s| s.kind.is_schedulable());
    Ok(json!({
        "today": today.format("%Y-%m-%d").to_string(),
        "timezone": ctx.config.timezone.to_string(),
        "activePlan": plan.as_ref().map(|p| json!({
            "id": p.id.to_string(),
            "name": p.name,
            "goal": p.goal.as_str(),
            "weeks": p.weeks.len(),
            "end": p.resolution.end.format("%Y-%m-%d").to_string(),
        })),
        "latestMetrics": metrics,
        "readinessToday": readiness,
        "injuryRiskToday": injury,
        "nextSession": next.as_ref().map(session_json),
        "accounts": accounts,
        "recentSyncRuns": sync_runs,
    }))
}

/// Input for [`score_day`].
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ScoreDayIn {
    /// Day to score, `YYYY-MM-DD`; defaults to today.
    #[serde(default)]
    pub date: Option<String>,
}

/// Compute, persist and return readiness + injury risk + load for one day.
///
/// # Errors
/// [`OpsError::Invalid`] for a bad date or a day with no health data,
/// [`OpsError::Store`] on persistence failure.
pub fn score_day(ctx: &Context, input: &ScoreDayIn) -> Result<Value, OpsError> {
    let date = input
        .date
        .as_deref()
        .map(parse_date)
        .transpose()?
        .unwrap_or_else(|| ctx.today());
    let history_from = date - TimeDelta::try_days(70).unwrap_or(TimeDelta::days(70));
    let activities = ActivityRepo::list_range(&ctx.db, history_from, date)?;
    let plan = PlanRepo::active(&ctx.db)?;
    let athlete = athlete_for(ctx, plan.as_ref());
    let threshold = threshold_for(ctx);
    let series = daily_loads(&activities, &athlete, threshold);

    let health_history = HealthRepo::list_range(&ctx.db, history_from, date)?;
    let day = health_history
        .iter()
        .rev()
        .find(|d| d.date == date)
        .or_else(|| {
            health_history
                .iter()
                .rev()
                .find(|d| HealthDay::has_recovery_signal(d))
        })
        .ok_or_else(|| {
            OpsError::invalid(format!(
                "no health data for {date} — sync the provider first"
            ))
        })?;
    let readiness = readiness_for(day, &health_history);
    let injury = injury_risk_for(&series, &athlete, Some(readiness.score), None);

    ReadinessRepo::save_readiness(
        &ctx.db,
        &ReadinessRow {
            date,
            formula_version: SCORING_VERSION,
            score: readiness.score,
            components: json!({
                "sleep": readiness.components.sleep,
                "hrv": readiness.components.hrv,
                "restingHr": readiness.components.resting_hr,
                "stress": readiness.components.stress,
                "lowestHr": readiness.components.lowest_hr,
            }),
            inputs: json!({"confidence": readiness.confidence}),
        },
    )?;
    ReadinessRepo::save_injury_risk(
        &ctx.db,
        &InjuryRiskRow {
            date,
            formula_version: SCORING_VERSION,
            score: injury.score,
            band: injury.band.as_str().to_string(),
            drivers: serde_json::to_value(
                injury
                    .drivers
                    .iter()
                    .map(|d| json!({"code": d.code, "message": d.message, "contribution": d.contribution}))
                    .collect::<Vec<_>>(),
            )
            .unwrap_or(Value::Null),
        },
    )?;
    ScoreRepo::save_metric_snapshot(
        &ctx.db,
        &MetricSnapshotRow {
            date,
            formula_version: SCORING_VERSION,
            chronic_load: chronic_load(&series),
            acute_load: acute_load(&series),
            training_stress_balance: chronic_load(&series) - acute_load(&series),
            acwr: acute_chronic_ratio(&series),
            monotony: monotony(&series),
            weekly_spike: weekly_spike(&series),
            vo2max_estimate: athlete.vo2max,
            threshold_pace: Some(threshold.as_secs_per_km()),
            performance_index: Some(performance_index(threshold)),
            components: json!({"activities": activities.len()}),
        },
    )?;

    Ok(json!({
        "date": date.format("%Y-%m-%d").to_string(),
        "readiness": readiness_json(&readiness),
        "injuryRisk": injury_json(&injury),
        "load": {
            "chronic": chronic_load(&series),
            "acute": acute_load(&series),
            "acwr": acute_chronic_ratio(&series),
            "monotony": monotony(&series),
            "weeklySpike": weekly_spike(&series),
        },
        "persisted": true,
    }))
}

fn readiness_json(r: &Readiness) -> Value {
    json!({
        "score": r.score,
        "band": ReadinessBand::from_score(r.score).as_str(),
        "confidence": r.confidence,
        "components": {
            "sleep": r.components.sleep,
            "hrv": r.components.hrv,
            "restingHr": r.components.resting_hr,
            "stress": r.components.stress,
            "lowestHr": r.components.lowest_hr,
        },
    })
}

fn injury_json(r: &InjuryRisk) -> Value {
    json!({
        "score": r.score,
        "band": r.band.as_str(),
        "drivers": r.drivers.iter().map(|d| json!({
            "code": d.code,
            "message": d.message,
            "contribution": d.contribution,
        })).collect::<Vec<_>>(),
    })
}

/// Input for [`score_session`].
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScoreSessionIn {
    /// The planned session that was executed.
    pub session_id: String,
    /// The activity that executed it.
    pub activity_id: String,
}

/// Score an executed session against its plan and persist the score.
///
/// Also links the activity to the session, so plan-vs-actual views and the
/// adherence trend have data.
///
/// # Errors
/// [`OpsError::Invalid`] for bad ids, [`OpsError::Store`] when either side is
/// unknown or persistence fails.
pub fn score_session_tool(ctx: &Context, input: &ScoreSessionIn) -> Result<Value, OpsError> {
    let session_id = parse_session_id(&input.session_id)?;
    let activity_id = parse_activity_id(&input.activity_id)?;
    let plan_id = PlanRepo::session_plan_id(&ctx.db, session_id)?;
    let plan = PlanRepo::load(&ctx.db, plan_id)?;
    let planned = plan
        .session_by_id(session_id)
        .ok_or_else(|| OpsError::invalid("session not in its own plan"))?
        .clone();
    let activity: Activity = ActivityRepo::get(&ctx.db, activity_id)?;
    let athlete = athlete_for(ctx, Some(&plan));
    let threshold = threshold_for(ctx);
    let feedback = latest_feedback(ctx, session_id, activity.local_date)?;
    let quality = score_session(&planned, &activity, &feedback, &athlete, threshold);

    ActivityRepo::match_session(&ctx.db, activity_id, Some(session_id))?;
    ScoreRepo::save_session_score(
        &ctx.db,
        &SessionScoreRow {
            session_id,
            formula_version: SCORING_VERSION,
            score: quality.score,
            tss: quality.actual_tss,
            intensity_factor: intensity_factor(&activity, &athlete, threshold),
            duration_s: activity.summary.duration.as_u32(),
            load: training_stress_score(&activity, &athlete, threshold),
            planned_quality: quality.planned_quality,
            executed_quality: quality.executed_quality,
            adherence: Some(quality.adherence),
            components: quality_json_components(&quality),
        },
    )?;
    Ok(json!({
        "sessionId": session_id.to_string(),
        "activityId": activity_id.to_string(),
        "score": quality.score,
        "plannedTss": quality.planned_tss,
        "actualTss": quality.actual_tss,
        "adherence": quality.adherence,
        "plannedQuality": quality.planned_quality,
        "executedQuality": quality.executed_quality,
        "confidence": quality.confidence,
        "persisted": true,
    }))
}

fn quality_json_components(q: &SessionQuality) -> Value {
    json!({
        "duration": q.components.duration,
        "distance": q.components.distance,
        "structure": q.components.structure,
        "pace": q.components.pace,
        "timeInZone": q.components.time_in_zone,
        "effort": q.components.effort,
        "subjective": q.components.subjective,
        "decouplingPenalty": q.components.decoupling_penalty,
    })
}

fn latest_feedback(
    ctx: &Context,
    session_id: PlannedSessionId,
    date: Date,
) -> Result<SessionFeedback, OpsError> {
    let from = date - TimeDelta::try_days(2).unwrap_or(TimeDelta::days(2));
    let rows = FeedbackRepo::list_range(&ctx.db, from, date + TimeDelta::days(1))?;
    let row = rows
        .into_iter()
        .filter(|r| r.session_id == Some(session_id))
        .max_by_key(|r| r.date);
    Ok(match row {
        Some(r) => SessionFeedback {
            rpe: r.rpe,
            feeling: r.mood,
            note: (!r.notes.is_empty()).then_some(r.notes),
        },
        None => SessionFeedback::default(),
    })
}

/// Input for [`record_feedback`].
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecordFeedbackIn {
    /// Day the feedback is about, `YYYY-MM-DD`; defaults to today.
    #[serde(default)]
    pub date: Option<String>,
    /// Planned session it refers to, when there is one.
    #[serde(default)]
    pub session_id: Option<String>,
    /// Session RPE, 6-20 (Borg).
    #[serde(default)]
    pub rpe: Option<u8>,
    /// Mood 1-5.
    #[serde(default)]
    pub mood: Option<u8>,
    /// Legs freshness 1-5.
    #[serde(default)]
    pub legs: Option<u8>,
    /// Motivation 1-5.
    #[serde(default)]
    pub motivation: Option<u8>,
    /// Free text.
    #[serde(default)]
    pub notes: Option<String>,
}

/// Store athlete feedback for a day/session.
///
/// # Errors
/// [`OpsError::Invalid`] for a bad date/session id, [`OpsError::Store`] on
/// persistence failure.
pub fn record_feedback(ctx: &Context, input: RecordFeedbackIn) -> Result<Value, OpsError> {
    let date = input
        .date
        .as_deref()
        .map(parse_date)
        .transpose()?
        .unwrap_or_else(|| ctx.today());
    let session_id = input
        .session_id
        .as_deref()
        .map(parse_session_id)
        .transpose()?;
    let row = FeedbackRow {
        id: runalytics_core::Uuid::now_v7(),
        session_id,
        date,
        rpe: input.rpe,
        mood: input.mood,
        legs: input.legs,
        motivation: input.motivation,
        notes: input.notes.unwrap_or_default(),
    };
    FeedbackRepo::insert(&ctx.db, &row)?;
    Ok(json!({ "feedbackId": row.id.to_string(), "date": date.format("%Y-%m-%d").to_string() }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use runalytics_core::Uuid;

    fn ctx() -> Context {
        Context::open(ServerConfig::default()).expect("in-memory context")
    }

    fn minimal_request() -> Value {
        let athlete = AthleteSnapshot::placeholder("Europe/Berlin".parse().expect("tz"));
        json!({
            "goal": "half_marathon",
            "anchor": {"kind": "horizon", "value": {"weeks": 8}},
            "athlete": athlete,
            "constraints": {
                "blackoutWeekdays": [],
                "noQualityWeekdays": [],
                "preferredStart": "07:00",
                "maxWeeklyVolume": null,
                "qualitySessions": null,
                "blackoutDates": [],
                "longRunWeekday": null,
            },
        })
    }

    #[test]
    fn generate_then_activate_then_get() {
        let ctx = ctx();
        let out = generate_plan(
            &ctx,
            GeneratePlanIn {
                request: minimal_request(),
                save: true,
            },
        )
        .expect("generate");
        let plan_id = out["planId"].as_str().expect("id").to_string();
        assert_eq!(out["status"], "draft");

        let activated = activate_plan(
            &ctx,
            &ActivatePlanIn {
                plan_id: plan_id.clone(),
            },
        )
        .expect("activate");
        assert_eq!(activated["activePlanId"], plan_id.as_str());

        let got = get_plan(&ctx, GetPlanIn { plan_id: None }).expect("get active");
        assert_eq!(got["plan"]["status"], "active");
    }

    #[test]
    fn generate_without_save_is_not_stored() {
        let ctx = ctx();
        let out = generate_plan(
            &ctx,
            GeneratePlanIn {
                request: minimal_request(),
                save: false,
            },
        )
        .expect("preview");
        assert!(!out["saved"].as_bool().expect("saved flag"));
        let plans = list_plans(&ctx, ListPlansIn {}).expect("list");
        assert_eq!(plans["plans"].as_array().expect("array").len(), 0);
    }

    #[test]
    fn bad_request_is_invalid_not_panic() {
        let ctx = ctx();
        let err = generate_plan(
            &ctx,
            GeneratePlanIn {
                request: json!({"goal": "ultra_marathon"}),
                save: true,
            },
        )
        .expect_err("must fail");
        assert!(matches!(err, OpsError::Invalid(_)), "got {err:?}");
    }

    #[test]
    fn update_session_moves_it_and_keeps_id() {
        let ctx = ctx();
        let out = generate_plan(
            &ctx,
            GeneratePlanIn {
                request: minimal_request(),
                save: true,
            },
        )
        .expect("generate");
        let first = PlanRepo::load(
            &ctx.db,
            parse_plan_id(out["planId"].as_str().expect("id")).expect("id"),
        )
        .expect("load");
        let session = first
            .all_sessions()
            .find(|s| s.kind.is_schedulable())
            .expect("a schedulable session");
        let sid = session.id.to_string();
        let updated = update_session(
            &ctx,
            UpdateSessionIn {
                session_id: sid.clone(),
                date: Some("2026-12-25".to_string()),
                start: Some("06:15".to_string()),
                kind: Some("tempo".to_string()),
                title: None,
                intent: None,
                target_volume_km: None,
                target_duration_min: None,
                target_pace_secs_per_km: None,
                rpe_target: None,
            },
        )
        .expect("update");
        assert_eq!(updated["session"]["date"], "2026-12-25");
        assert_eq!(updated["session"]["kind"], "tempo");
        assert_eq!(updated["session"]["id"], sid.as_str());
    }

    #[test]
    fn upcoming_sessions_window_filters() {
        let ctx = ctx();
        generate_plan(
            &ctx,
            GeneratePlanIn {
                request: minimal_request(),
                save: true,
            },
        )
        .expect("generate");
        let out = upcoming_sessions(
            &ctx,
            &UpcomingSessionsIn {
                from: None,
                days: Some(365),
            },
        )
        .expect("upcoming");
        let sessions = out["sessions"].as_array().expect("array");
        assert!(!sessions.is_empty(), "draft sessions are upcoming");
    }

    #[test]
    fn status_without_plan_reports_none() {
        let ctx = ctx();
        let out = get_status(&ctx, GetStatusIn {}).expect("status");
        assert!(out["activePlan"].is_null());
        assert!(out["latestMetrics"].is_null());
    }

    #[test]
    fn score_day_without_health_is_invalid() {
        let ctx = ctx();
        let err = score_day(&ctx, &ScoreDayIn { date: None }).expect_err("no health data");
        assert!(matches!(err, OpsError::Invalid(_)), "got {err:?}");
    }

    #[test]
    fn feedback_roundtrips_into_status_store() {
        let ctx = ctx();
        let out = record_feedback(
            &ctx,
            RecordFeedbackIn {
                date: Some("2026-10-08".to_string()),
                session_id: None,
                rpe: Some(14),
                mood: Some(4),
                legs: Some(3),
                motivation: None,
                notes: Some("sharp but flat".to_string()),
            },
        )
        .expect("feedback");
        assert!(out["feedbackId"].is_string());
        let rows = FeedbackRepo::list_range(
            &ctx.db,
            parse_date("2026-10-01").expect("d"),
            parse_date("2026-10-31").expect("d"),
        )
        .expect("rows");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].rpe, Some(14));
    }

    #[test]
    fn unknown_ids_are_invalid_or_not_found() {
        let ctx = ctx();
        let err = update_session(
            &ctx,
            UpdateSessionIn {
                session_id: "not-a-uuid".to_string(),
                date: None,
                start: None,
                kind: None,
                title: None,
                intent: None,
                target_volume_km: None,
                target_duration_min: None,
                target_pace_secs_per_km: None,
                rpe_target: None,
            },
        )
        .expect_err("bad id");
        assert!(matches!(err, OpsError::Invalid(_)));
        let missing = Uuid::now_v7().to_string();
        let err = update_session(
            &ctx,
            UpdateSessionIn {
                session_id: missing,
                date: None,
                start: None,
                kind: None,
                title: None,
                intent: None,
                target_volume_km: None,
                target_duration_min: None,
                target_pace_secs_per_km: None,
                rpe_target: None,
            },
        )
        .expect_err("unknown session");
        assert!(
            matches!(err, OpsError::Store(StoreError::NotFound { .. })),
            "got {err:?}"
        );
    }

    #[test]
    fn calendar_feed_requires_active_plan() {
        let ctx = ctx();
        let err = get_calendar_feed(&ctx, GetCalendarFeedIn {}).expect_err("no plan");
        assert!(matches!(err, OpsError::Invalid(_)));
    }
}
