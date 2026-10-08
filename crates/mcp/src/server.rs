//! The rmcp server: tool declarations over the [`ops`](crate::ops) layer.
//!
//! Each tool handler is three lines: deserialize, call the pure op, encode
//! the result. Failures become MCP tool-error results carrying
//! `{"error": "..."}` so an agent can read *why* and correct the call,
//! rather than a protocol-level error that clients often surface as a
//! generic failure.

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{ServerHandler, tool, tool_handler, tool_router};
use serde_json::Value;

use crate::ops::{
    self, ActivatePlanIn, Context, GeneratePlanIn, GetCalendarFeedIn, GetPlanIn, GetStatusIn,
    ListPlansIn, OpsError, RecordFeedbackIn, ScoreDayIn, ScoreSessionIn, SyncCalendarIn,
    UpcomingSessionsIn, UpdateSessionIn,
};

/// The Runalytics MCP service. Cheap to clone; every clone shares the same
/// database connection pool and configuration.
#[derive(Clone)]
pub struct RunalyticsServer {
    ctx: Context,
}

impl RunalyticsServer {
    /// Wrap a ready context.
    #[must_use]
    pub fn new(ctx: Context) -> Self {
        Self { ctx }
    }

    /// Encode an op result for the wire: pretty JSON as text content.
    fn ok(value: &Value) -> CallToolResult {
        CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string()),
        )])
    }

    /// Encode an op failure as a tool error with a readable message.
    fn fail(err: &OpsError) -> CallToolResult {
        let body = serde_json::json!({ "error": err.to_string() });
        CallToolResult::error(vec![ContentBlock::text(body.to_string())])
    }
}

#[tool_router]
impl RunalyticsServer {
    /// Generate a training plan from a PlanRequest and store it as a draft.
    #[tool(
        name = "generate_plan",
        description = "Generate a deterministic training plan (marathon/half/10k/5k/recovery/maintain, horizon or race-date anchored) from a PlanRequest, and store it as a draft. Returns weeks, sessions and engine notes."
    )]
    async fn generate_plan(&self, Parameters(input): Parameters<GeneratePlanIn>) -> CallToolResult {
        match ops::generate_plan(&self.ctx, input) {
            Ok(v) => Self::ok(&v),
            Err(e) => Self::fail(&e),
        }
    }

    /// Read a stored plan in full.
    #[tool(
        name = "get_plan",
        description = "Return a stored plan in full by id; defaults to the active plan."
    )]
    async fn get_plan(&self, Parameters(input): Parameters<GetPlanIn>) -> CallToolResult {
        match ops::get_plan(&self.ctx, input) {
            Ok(v) => Self::ok(&v),
            Err(e) => Self::fail(&e),
        }
    }

    /// List stored plans.
    #[tool(
        name = "list_plans",
        description = "List all stored plans (id, name, goal, status, window, session count), newest first."
    )]
    async fn list_plans(&self, Parameters(input): Parameters<ListPlansIn>) -> CallToolResult {
        match ops::list_plans(&self.ctx, input) {
            Ok(v) => Self::ok(&v),
            Err(e) => Self::fail(&e),
        }
    }

    /// Get a fillable PlanRequest skeleton.
    #[tool(
        name = "plan_request_template",
        description = "Return a complete, valid PlanRequest JSON skeleton (placeholder athlete, 8-week half-marathon horizon). Edit the fields you know and pass it to generate_plan — every constraints/athlete field is required in JSON, so start here."
    )]
    async fn plan_request_template(&self) -> CallToolResult {
        match ops::plan_request_template(&self.ctx) {
            Ok(v) => Self::ok(&v),
            Err(e) => Self::fail(&e),
        }
    }

    /// Activate a plan.
    #[tool(
        name = "activate_plan",
        description = "Make a plan active; the previously active plan is paused. Calendar sync and scoring follow the active plan."
    )]
    async fn activate_plan(&self, Parameters(input): Parameters<ActivatePlanIn>) -> CallToolResult {
        match ops::activate_plan(&self.ctx, &input) {
            Ok(v) => Self::ok(&v),
            Err(e) => Self::fail(&e),
        }
    }

    /// Update one planned session.
    #[tool(
        name = "update_session",
        description = "Move or rewrite one planned session (date, start, kind, title, intent, volume, duration, pace, RPE). The session keeps its id, so calendar entries follow the change on the next sync_calendar."
    )]
    async fn update_session(
        &self,
        Parameters(input): Parameters<UpdateSessionIn>,
    ) -> CallToolResult {
        match ops::update_session(&self.ctx, input) {
            Ok(v) => Self::ok(&v),
            Err(e) => Self::fail(&e),
        }
    }

    /// List upcoming sessions.
    #[tool(
        name = "upcoming_sessions",
        description = "List scheduled sessions from a day (default today) forward, for plan-aware coaching."
    )]
    async fn upcoming_sessions(
        &self,
        Parameters(input): Parameters<UpcomingSessionsIn>,
    ) -> CallToolResult {
        match ops::upcoming_sessions(&self.ctx, &input) {
            Ok(v) => Self::ok(&v),
            Err(e) => Self::fail(&e),
        }
    }

    /// Publish the active plan to calendars.
    #[tool(
        name = "sync_calendar",
        description = "Publish the active plan to Calendar.app (and the ICS file when configured). Idempotent: moved sessions update in place via their stable uid."
    )]
    async fn sync_calendar(&self, Parameters(input): Parameters<SyncCalendarIn>) -> CallToolResult {
        match ops::sync_calendar(&self.ctx, input).await {
            Ok(v) => Self::ok(&v),
            Err(e) => Self::fail(&e),
        }
    }

    /// Get the raw ICS feed.
    #[tool(
        name = "get_calendar_feed",
        description = "Return the raw ICS text for the active plan, for webcal-style consumption."
    )]
    async fn get_calendar_feed(
        &self,
        Parameters(input): Parameters<GetCalendarFeedIn>,
    ) -> CallToolResult {
        match ops::get_calendar_feed(&self.ctx, input) {
            Ok(v) => Self::ok(&v),
            Err(e) => Self::fail(&e),
        }
    }

    /// Today's dashboard.
    #[tool(
        name = "get_status",
        description = "Today's dashboard: active plan, latest load metrics, readiness and injury risk for today, next session, provider accounts and recent sync runs."
    )]
    async fn get_status(&self, Parameters(input): Parameters<GetStatusIn>) -> CallToolResult {
        match ops::get_status(&self.ctx, input) {
            Ok(v) => Self::ok(&v),
            Err(e) => Self::fail(&e),
        }
    }

    /// Score a day.
    #[tool(
        name = "score_day",
        description = "Compute and persist readiness, injury risk and load metrics for one day (default today), from synced health and activity data."
    )]
    async fn score_day(&self, Parameters(input): Parameters<ScoreDayIn>) -> CallToolResult {
        match ops::score_day(&self.ctx, &input) {
            Ok(v) => Self::ok(&v),
            Err(e) => Self::fail(&e),
        }
    }

    /// Score an executed session.
    #[tool(
        name = "score_session",
        description = "Score an activity against its planned session (TSS, adherence, component breakdown) and link the two. Persists the score."
    )]
    async fn score_session(&self, Parameters(input): Parameters<ScoreSessionIn>) -> CallToolResult {
        match ops::score_session_tool(&self.ctx, &input) {
            Ok(v) => Self::ok(&v),
            Err(e) => Self::fail(&e),
        }
    }

    /// Record athlete feedback.
    #[tool(
        name = "record_feedback",
        description = "Store the athlete's subjective feedback (RPE, mood, legs, motivation, notes) for a day or session; feeds subjective quality and coaching context."
    )]
    async fn record_feedback(
        &self,
        Parameters(input): Parameters<RecordFeedbackIn>,
    ) -> CallToolResult {
        match ops::record_feedback(&self.ctx, input) {
            Ok(v) => Self::ok(&v),
            Err(e) => Self::fail(&e),
        }
    }
}

// The macro-generated ServerHandler methods are async by trait shape but
// contain no `.await`, which trips this lint inside the expansion.
#[allow(clippy::unused_async_trait_impl)]
#[tool_handler(
    name = "runalytics",
    instructions = "Runalytics: training plans, scoring and calendars for runners. \
                    Plans: generate_plan -> activate_plan -> sync_calendar. \
                    Coaching loop: get_status, score_day, upcoming_sessions, \
                    update_session to reshape the plan, score_session after a run. \
                    All dates are YYYY-MM-DD in the athlete's time zone; paces are seconds per km."
)]
impl ServerHandler for RunalyticsServer {}
