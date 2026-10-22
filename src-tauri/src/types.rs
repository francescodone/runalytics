use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Coros,
    Garmin,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatus {
    pub provider: ProviderKind,
    pub connected: bool,
    pub last_sync: Option<DateTime<Utc>>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub id: i64,
    pub provider: ProviderKind,
    pub provider_record_id: String,
    pub sport: String,
    pub start_time: DateTime<Utc>,
    pub duration_s: f64,
    pub distance_m: f64,
    pub avg_hr: Option<i64>,
    pub max_hr: Option<i64>,
    /// Average pace in seconds per km (lower is faster).
    pub avg_pace_s_km: Option<f64>,
    pub elevation_gain_m: Option<f64>,
    pub has_track: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeatPoint {
    pub lat: f64,
    pub lon: f64,
    pub weight: f64,
}

/// One GPS sample of an activity track (unix seconds, WGS-84).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackPoint {
    pub ts: i64,
    pub lat: f64,
    pub lon: f64,
    pub hr: Option<i64>,
}

/// Everything the activity detail sheet shows: the GPS track, the raw
/// session data, the recovery signals recorded for that day, and the
/// injury-risk heuristics evaluated *as of* that day.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityDetail {
    pub activity: Activity,
    pub track: Vec<TrackPoint>,
    pub health: Option<HealthDaily>,
    pub load: Option<TrainingLoad>,
    pub fitness: Option<FitnessAssessment>,
    pub risk_factors: Vec<RiskFactor>,
    /// 0..=100 headline risk for that day (same formula as the dashboard).
    pub injury_risk: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthDaily {
    pub date: NaiveDate,
    pub steps: Option<i64>,
    pub avg_hr: Option<i64>,
    pub resting_hr: Option<i64>,
    pub stress_avg: Option<i64>,
    pub sleep_score: Option<i64>,
    pub sleep_min: Option<i64>,
    pub deep_min: Option<i64>,
    pub rem_min: Option<i64>,
    pub hrv_avg: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FitnessAssessment {
    pub date: NaiveDate,
    pub vo2max: Option<f64>,
    pub threshold_sec_km: Option<f64>,
    pub predict_5k_s: Option<i64>,
    pub predict_10k_s: Option<i64>,
    pub predict_half_s: Option<i64>,
    pub predict_marathon_s: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingLoad {
    pub date: NaiveDate,
    pub short_term: Option<f64>,
    pub long_term: Option<f64>,
    pub ratio: Option<f64>,
    pub comment: Option<String>,
}

/// One day of the load history behind the training-status chart, oldest→newest.
/// `ratio` is the stored value when the provider sent one, otherwise derived
/// from short/long so the chart has no holes on days the watch only reported
/// the two loads.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadPoint {
    pub date: NaiveDate,
    pub short_term: Option<f64>,
    pub long_term: Option<f64>,
    pub ratio: Option<f64>,
}

/// Where the recent load sits against the boundaries — the answer to "am I
/// training too little, too much, or right?". Uses COROS's published Intensity
/// Trend vocabulary (Decreasing / Resuming / Performance / Maintaining /
/// Optimized / Excessive) rather than inventing a second one.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingStatus {
    /// Machine key: `decreasing` | `resuming` | `performance` | `maintaining`
    /// | `optimized` | `excessive`.
    pub state: String,
    /// Short headline shown on the chart.
    pub label: String,
    /// One sentence quoting the runner's actual ratio.
    pub reason: String,
    /// `good` | `warn` | `bad` — the shared tone vocabulary.
    pub tone: String,
    /// The acute:chronic ratio this verdict was derived from.
    pub ratio: f64,
}

/// Progress of a plan push/cancel toward the watch, emitted on the
/// `plan://sync` event so the Plan page can show a live status instead of a
/// dead button. COROS takes a whole plan in one call, so the percentage
/// tracks the steps on this side of that call rather than sessions sent.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanSyncProgress {
    /// `push` | `cancel` — which operation is running.
    pub op: String,
    /// 0..=100.
    pub percent: u8,
    /// Short status shown beside the spinner.
    pub stage: String,
}

/// Race goal the plan is built for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RaceDistance {
    #[serde(rename = "5K")]
    K5,
    #[serde(rename = "10K")]
    K10,
    #[serde(rename = "21K")]
    K21,
    #[serde(rename = "42K")]
    K42,
}

impl RaceDistance {
    pub fn distance_m(self) -> f64 {
        match self {
            RaceDistance::K5 => 5_000.0,
            RaceDistance::K10 => 10_000.0,
            RaceDistance::K21 => 21_097.5,
            RaceDistance::K42 => 42_195.0,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            RaceDistance::K5 => "5K",
            RaceDistance::K10 => "10K",
            RaceDistance::K21 => "Half marathon",
            RaceDistance::K42 => "Marathon",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IncreaseMethod {
    /// ~+5 % weekly volume growth.
    Conservative,
    /// ~+8 % weekly volume growth.
    Moderate,
    /// ~+12 % weekly volume growth.
    Ambitious,
}

impl IncreaseMethod {
    pub fn weekly_factor(self) -> f64 {
        match self {
            IncreaseMethod::Conservative => 1.05,
            IncreaseMethod::Moderate => 1.08,
            IncreaseMethod::Ambitious => 1.12,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlanStatus {
    Draft,
    Active,
    Pushed,
    Completed,
    Aborted,
}

/// What the plan is *for*, which decides its whole shape rather than just its
/// label: only a race build gets a taper and a race day, only an improvement
/// build ramps volume, and a maintenance block holds the runner's current
/// volume instead of building.
///
/// `distance` means something different in each: the race being entered, the
/// distance an improvement block is sharpening toward, and the distance a
/// maintenance block keeps its long run and quality sessions tuned to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlanKind {
    /// Build to a race on a fixed date: ramp, taper, race day.
    Race,
    /// Build fitness toward a distance over a chosen horizon, no race entered.
    Improvement,
    /// Hold current volume with the distance's session mix, no ramp.
    Maintenance,
}

impl PlanKind {
    pub fn label(self) -> &'static str {
        match self {
            PlanKind::Race => "Race",
            PlanKind::Improvement => "Improvement",
            PlanKind::Maintenance => "Maintenance",
        }
    }

    /// Whether the plan's `target_date` is an actual race start. Only then do
    /// a taper, a shakeout and a race-day session belong in the schedule.
    pub fn has_race(self) -> bool {
        self == PlanKind::Race
    }

    /// Serde default: every plan written before kinds existed was a race build.
    pub fn default_race() -> PlanKind {
        PlanKind::Race
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkoutKind {
    Rest,
    Easy,
    Tempo,
    Intervals,
    LongRun,
    Race,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanRequest {
    pub distance: RaceDistance,
    /// What the plan is built for. Absent on requests stored before plan
    /// kinds existed, and on any caller that only speaks race dates: both
    /// mean a race build, which is what every plan in the database is.
    #[serde(default = "PlanKind::default_race")]
    pub kind: PlanKind,
    pub target_date: NaiveDate,
    /// Current threshold pace, seconds per km.
    pub threshold_sec_km: f64,
    /// Runs per week (3..=7).
    pub weekly_frequency: u32,
    /// ISO weekday of the long run (1 = Mon .. 7 = Sun).
    pub longrun_day: u32,
    pub increase: IncreaseMethod,
    /// Runner's current weekly mileage (km). When absent the backend
    /// derives it from the last weeks of synced activities.
    #[serde(default)]
    pub current_weekly_km: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingPlan {
    pub id: String,
    pub distance: RaceDistance,
    /// What the plan is built for; `Race` for plans stored before kinds existed.
    #[serde(default = "PlanKind::default_race")]
    pub kind: PlanKind,
    pub target_date: NaiveDate,
    pub status: PlanStatus,
    pub weekly_frequency: u32,
    pub longrun_day: u32,
    pub increase: IncreaseMethod,
    pub threshold_sec_km: f64,
    pub created_at: DateTime<Utc>,
    /// Plan id assigned by the provider when pushed (COROS plan id).
    pub pushed_id: Option<String>,
    /// Weekly mileage (km) the plan was generated from — the runner's actual
    /// volume at creation time, or the value typed into the form. `None` for
    /// plans stored before this was recorded.
    pub baseline_weekly_km: Option<f64>,
    /// Seconds per km added to every **quality** pace band from
    /// `quality_offset_from` onward. Negative means the ranges were sharpened.
    ///
    /// This is the plan's memory of the pacing adjustments the runner accepted:
    /// a tempo or interval range that keeps getting run down is evidence the
    /// range was set too easy, and the fix is to move the range, not to rewrite
    /// the runner's threshold. Easy runs, long runs and race pace are
    /// deliberately untouched — a session being too easy says nothing about
    /// what an easy day should feel like.
    #[serde(default)]
    pub quality_offset_sec_km: f64,
    /// First day the offset above applies to. `None` means no adjustment has
    /// ever been accepted, and — importantly — that sessions already run keep
    /// being reconstructed with the ranges they were actually set, so a
    /// proposal accepted today cannot retroactively move last week's target.
    #[serde(default)]
    pub quality_offset_from: Option<NaiveDate>,
}

/// Which side of a session's prescribed pace range the runner landed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Execution {
    /// Faster than the fast end of the range — the session was too easy.
    #[serde(rename = "ahead")]
    Ahead,
    /// Inside the range, which is the point of having one.
    #[serde(rename = "in")]
    InRange,
    /// Slower than the slow end — the session asked for more than was there.
    #[serde(rename = "behind")]
    Behind,
}

impl Execution {
    /// True when this verdict is evidence the range needs moving (either way).
    pub fn off_range(self) -> bool {
        matches!(self, Execution::Ahead | Execution::Behind)
    }

    pub fn label(self) -> &'static str {
        match self {
            Execution::Ahead => "too fast",
            Execution::InRange => "in range",
            Execution::Behind => "too slow",
        }
    }
}

/// A completed quality session graded against the range the plan set for it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionReview {
    pub workout_id: String,
    pub date: NaiveDate,
    pub kind: WorkoutKind,
    pub title: String,
    /// The range the runner was asked to hold, seconds per km.
    pub band_lo_s_km: i64,
    pub band_hi_s_km: i64,
    /// Pace actually held over the main set, and how much of the run that
    /// covered. `None` when the activity's track has not synced yet — the
    /// session's overall average pace is *not* a substitute, because it is
    /// dragged down by the warm-up and cool-down and would read as "too slow"
    /// for a session that was executed perfectly.
    pub actual_s_km: Option<f64>,
    pub measured_m: f64,
    pub execution: Execution,
}

/// What the pacing engine made of a run of quality sessions, for the UI.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PacingReport {
    /// The most recent run of out-of-range sessions, newest last. Sessions
    /// whose track has not synced are absent rather than guessed at.
    pub reviews: Vec<SessionReview>,
    /// How many of them leaned the same way, and which way.
    pub streak: usize,
    pub direction: Option<Execution>,
    /// The streak needed before an offer is made.
    pub needed: u32,
    /// True when the streak is long enough that an offer exists or was made.
    pub triggered: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanWorkout {
    pub id: String,
    pub plan_id: String,
    pub date: NaiveDate,
    pub kind: WorkoutKind,
    pub title: String,
    pub distance_m: Option<f64>,
    pub duration_min: Option<f64>,
    pub pace_lo_s_km: Option<i64>,
    pub pace_hi_s_km: Option<i64>,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardScores {
    /// 0..=100 training-structure score (load balance + consistency).
    pub training_score: u32,
    /// 0..=100 injury-risk score (higher = riskier).
    pub injury_risk: u32,
    /// 0..=100 recovery score.
    pub recovery_score: u32,
    /// 0..=100 weighted global score.
    pub global_score: u32,
    pub notes: Vec<String>,
}

/// One day of the readiness sub-scores, oldest→newest, for the dashboard
/// sparkline cards: the same formulas as [`DashboardScores`], re-evaluated
/// as of each day.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScorePoint {
    pub date: NaiveDate,
    pub training: u32,
    pub recovery: u32,
    /// 0..=100 injury risk (higher = riskier), same polarity as the headline.
    pub risk: u32,
}

/// Where the runner's recovery score currently sits. The four bands and their
/// percentage ranges follow COROS's published recovery scale (0–19 Exhausted,
/// 20–69 Fatigued, 70–89 Normal, 90–100 Fresh); the boundaries are editable in
/// Settings, so the band is always derived from the live thresholds rather
/// than hard-coded at a call site.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryBand {
    /// `exhausted` | `fatigued` | `normal` | `fresh`.
    pub state: String,
    pub label: String,
    /// `good` | `warn` | `bad` — the shared tone vocabulary.
    pub tone: String,
    /// The inclusive percentage range this band covers, for display.
    pub min: u32,
    pub max: u32,
}

/// The 7-day Training Load the runner should be aiming at, and where they sit
/// in the build cycle. COROS derives its recommendation from fitness level and
/// recent weeks on a 3:1 cycle: three weeks of increasing load, then a fourth
/// deliberately lower for recovery.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadRecommendation {
    /// Target 7-day load for the current week, in load points.
    pub target: f64,
    /// True when the cycle says this week should be a down week.
    pub down_week: bool,
    /// 1-based position inside the cycle, e.g. 3 of 4.
    pub week_in_cycle: u32,
    /// Cycle length the position is counted against (build weeks + down week).
    pub cycle_length: u32,
    /// One sentence explaining the number.
    pub note: String,
}

/// Editable decision boundaries for the three COROS-style metrics. Every
/// consumer (scoring, the status chart, the recovery read-out, the adaptive
/// engine) reads these instead of a literal, so moving a boundary in Settings
/// moves it everywhere at once.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Thresholds {
    /// Top of the Decreasing band; below it, base fitness is declining.
    pub ratio_decreasing_hi: f64,
    /// Top of the shared Resuming/Performance band; above it, Maintaining.
    pub ratio_resuming_hi: f64,
    /// Top of the Maintaining band; above it, Optimized.
    pub ratio_maintaining_hi: f64,
    /// Top of the Optimized band; above it, Excessive. Also the point where
    /// injury risk starts to climb and the adaptive engine softens hard days.
    pub ratio_optimized_hi: f64,
    /// Recovery at or below this is Exhausted (rest recommended).
    pub recovery_exhausted: u32,
    /// Recovery at or below this is Fatigued; above it, Normal.
    pub recovery_fatigued: u32,
    /// Recovery at or below this is Normal; above it, Fresh.
    pub recovery_normal: u32,
    /// Consecutive build weeks before a down week (COROS uses 3 → a 3:1 cycle).
    pub build_weeks: u32,
    /// A down week targets this fraction of the normal weekly load.
    pub down_week_factor: f64,
    /// How far outside a session's pace range a block has to land before it
    /// counts as out of range, in seconds per km. Zero would mean "any part
    /// outside", which flags a session that missed by one second of GPS noise;
    /// the default asks for a miss you could actually feel.
    pub pace_tolerance_sec_km: f64,
    /// How many consecutive quality sessions have to land out of range the same
    /// way before the app offers to move the plan. One session off range is a
    /// bad day, a hill, or a group run — it is the run that is evidence.
    pub pace_streak: u32,
}

impl Default for Thresholds {
    fn default() -> Self {
        Thresholds {
            // COROS's published Intensity Trend scale: Decreasing <50%,
            // Resuming/Performance 50-79%, Maintaining 80-99%, Optimized
            // 100-149%, Excessive >150%.
            ratio_decreasing_hi: 0.5,
            ratio_resuming_hi: 0.8,
            ratio_maintaining_hi: 1.0,
            ratio_optimized_hi: 1.5,
            recovery_exhausted: 19,
            recovery_fatigued: 69,
            recovery_normal: 89,
            build_weeks: 3,
            down_week_factor: 0.7,
            pace_tolerance_sec_km: 4.0,
            pace_streak: 3,
        }
    }
}

impl Thresholds {
    /// Clamped and ordered. Settings values arrive from the UI, and a file
    /// edited by hand can hold anything — a boundary above the next one would
    /// silently empty a band and push every verdict into the wrong zone, so
    /// the invariants are repaired here once rather than defended at every
    /// comparison. The four ratio boundaries form a strictly increasing chain,
    /// each forced at least a step above the previous.
    pub fn normalized(&self) -> Thresholds {
        let clamp = |v: f64, lo: f64, hi: f64| v.clamp(lo, hi);
        let decreasing = clamp(self.ratio_decreasing_hi, 0.2, 0.7);
        let resuming = clamp(self.ratio_resuming_hi, 0.3, 0.95).max(decreasing + 0.05);
        let maintaining = clamp(self.ratio_maintaining_hi, 0.5, 1.2).max(resuming + 0.05);
        let optimized = clamp(self.ratio_optimized_hi, 0.9, 3.0).max(maintaining + 0.05);
        let exhausted = self.recovery_exhausted.min(95);
        let fatigued = self.recovery_fatigued.clamp(exhausted + 1, 97);
        let normal = self.recovery_normal.max(fatigued + 1).min(99);
        Thresholds {
            ratio_decreasing_hi: decreasing,
            ratio_resuming_hi: resuming,
            ratio_maintaining_hi: maintaining,
            ratio_optimized_hi: optimized,
            recovery_exhausted: exhausted,
            recovery_fatigued: fatigued,
            recovery_normal: normal,
            build_weeks: self.build_weeks.clamp(1, 8),
            down_week_factor: self.down_week_factor.clamp(0.4, 1.0),
            pace_tolerance_sec_km: self.pace_tolerance_sec_km.clamp(0.0, 20.0),
            pace_streak: self.pace_streak.clamp(2, 6),
        }
    }
}

/// Dashboard time window selected in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeRange {
    #[serde(rename = "1d")]
    D1,
    #[serde(rename = "7d")]
    D7,
    #[serde(rename = "1mo")]
    M1,
}

impl TimeRange {
    pub fn days(self) -> i64 {
        match self {
            TimeRange::D1 => 1,
            TimeRange::D7 => 7,
            TimeRange::M1 => 30,
        }
    }
}

/// Aggregated training stats for the selected time range.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RangeStats {
    pub distance_m: f64,
    pub duration_s: f64,
    pub sessions: i64,
    pub avg_pace_s_km: Option<f64>,
    pub avg_hr: Option<i64>,
}

/// Running mileage rolled up from synced activities. Distances are metres;
/// `weekly_avg_m` is `None` until at least one complete week has been synced.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MileageStats {
    /// Mean weekly distance over the last up-to-4 complete weeks.
    pub weekly_avg_m: Option<f64>,
    /// Distance logged in the current (in-progress) week.
    pub this_week_m: f64,
    /// Every running metre ever synced.
    pub total_m: f64,
    /// Running sessions in the history.
    pub sessions: i64,
    /// Complete weeks the average is taken over (1..=4).
    pub weeks_observed: i64,
    /// When the watch last synced, so the UI can date-stamp the number.
    pub last_sync: Option<DateTime<Utc>>,
}

/// One heuristic contributing to the injury-risk percentage.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskFactor {
    pub label: String,
    pub detail: String,
    /// 0..=1 severity of this factor on its own scale — 0 means the signal is
    /// quiet, 1 means it alone is maxed out. Not a share of the total.
    pub contribution: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    pub activities_new: u32,
    pub health_days: u32,
    pub fitness_updated: bool,
    pub tracks_downloaded: u32,
    pub messages: Vec<String>,
}

// ---- injury journal --------------------------------------------------------

/// One pain report typed by the runner. `body_part` is a coarse region key
/// from the body map (`knee.left`, `lower_back`, `other`); it is stored as free
/// text so the region list can evolve without a migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PainEntry {
    pub id: String,
    pub body_part: String,
    /// Runner-rated severity, 0..=10.
    pub intensity: u32,
    /// When the runner says it started — not when it was logged.
    pub started_on: NaiveDate,
    pub notes: String,
    pub created_at: DateTime<Utc>,
    /// Set once the runner confirms the pain went away. `None` = still hurting.
    pub resolved_on: Option<NaiveDate>,
    /// When the resolution follow-up was last asked (once per episode).
    pub followup_at: Option<DateTime<Utc>>,
}

/// One turn of the assistant thread for one pain entry. `body_part` stays on
/// the row because the model context and the legacy region-keyed threads are
/// region-scoped; `entry_id` is the thread key.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PainMessage {
    pub id: String,
    pub body_part: String,
    /// The journal entry this turn belongs to. `serde(default)` keeps rows
    /// written before per-entry threads existed deserialising.
    #[serde(default)]
    pub entry_id: Option<String>,
    /// "user" | "assistant".
    pub role: String,
    pub content: String,
    pub ts: DateTime<Utc>,
}

/// A single concrete change to one planned workout. Proposals are lists of
/// these, which keeps an offer reviewable as a diff and applying it a plain
/// loop over `update_workout`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkoutEdit {
    pub workout_id: String,
    pub kind: WorkoutKind,
    pub title: String,
    pub distance_m: Option<f64>,
    pub duration_min: Option<f64>,
    pub pace_lo_s_km: Option<i64>,
    pub pace_hi_s_km: Option<i64>,
    /// The workout's date and original kind/title/distance at offer time, so
    /// the review card can show a labelled was → now diff. `serde(default)`
    /// keeps proposals stored before these fields existed readable.
    #[serde(default)]
    pub date: Option<NaiveDate>,
    #[serde(default)]
    pub old_kind: Option<WorkoutKind>,
    #[serde(default)]
    pub old_title: Option<String>,
    #[serde(default)]
    pub old_distance_m: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProposalStatus {
    Pending,
    Applied,
    Dismissed,
    /// Superseded by a newer offer (the pain changed, or the plan moved on).
    Superseded,
}

/// What an offer is a response to. Stored per proposal so the Injuries page
/// and the Plan page each show only their own offers instead of reading
/// whatever happens to be pending.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProposalSource {
    /// A change offered because the runner logged pain.
    #[default]
    Pain,
    /// A change offered because recent tempo/interval sessions kept landing
    /// outside the range the plan set for them.
    Pacing,
}

/// An offered plan change, generated deterministically from the pain entry and
/// the current plan. The assistant can explain it, but never authors it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanProposal {
    pub id: String,
    pub body_part: String,
    /// What generated the offer. Pain offers and pacing offers review the same
    /// way and apply through the same path, but they belong on different pages
    /// and must never be described as each other's reason.
    #[serde(default)]
    pub source: ProposalSource,
    /// For a pacing offer: the plan's quality-pace offset to store once the
    /// offer is accepted. `None` for every other kind of offer, and the plan is
    /// then left alone.
    #[serde(default)]
    pub plan_quality_offset_sec_km: Option<f64>,
    /// For a pacing offer: the first day that offset applies to.
    #[serde(default)]
    pub plan_quality_offset_from: Option<NaiveDate>,
    /// `tier:region:weekAnchor` — identical offers collapse instead of nagging.
    pub signature: String,
    /// One sentence in plain language, shown as the card headline.
    pub summary: String,
    /// Why, quoting the runner's actual numbers.
    pub reason: String,
    pub edits: Vec<WorkoutEdit>,
    pub status: ProposalStatus,
    pub created_at: DateTime<Utc>,
    pub acted_at: Option<DateTime<Utc>>,
}
