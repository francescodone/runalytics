// Mirrors the camelCase serde types in src-tauri/src/types.rs.

export type ProviderKind = "coros" | "garmin";

export interface ProviderStatus {
  provider: ProviderKind;
  connected: boolean;
  lastSync: string | null;
  detail: string;
}

export interface Activity {
  id: number;
  provider: ProviderKind;
  providerRecordId: string;
  sport: string;
  startTime: string;
  durationS: number;
  distanceM: number;
  avgHr: number | null;
  maxHr: number | null;
  avgPaceSKm: number | null;
  elevationGainM: number | null;
  hasTrack: boolean;
}

export interface HeatPoint {
  lat: number;
  lon: number;
  weight: number;
}

/// One GPS sample of an activity track (unix seconds, WGS-84).
export interface TrackPoint {
  ts: number;
  lat: number;
  lon: number;
  hr: number | null;
}

/// Everything the activity detail sheet shows.
export interface ActivityDetail {
  activity: Activity;
  track: TrackPoint[];
  health: HealthDaily | null;
  load: TrainingLoad | null;
  fitness: FitnessAssessment | null;
  riskFactors: RiskFactor[];
  /// 0..100 headline injury risk evaluated as of that day.
  injuryRisk: number;
}

export interface FitnessAssessment {
  date: string;
  vo2max: number | null;
  thresholdSecKm: number | null;
  predict5kS: number | null;
  predict10kS: number | null;
  predictHalfS: number | null;
  predictMarathonS: number | null;
}

export interface TrainingLoad {
  date: string;
  shortTerm: number | null;
  longTerm: number | null;
  ratio: number | null;
  comment: string | null;
}

/// One day of the load history behind the training-status chart, oldest→newest.
export interface LoadPoint {
  date: string;
  shortTerm: number | null;
  longTerm: number | null;
  ratio: number | null;
}

/// Which side of the training boundaries recent load falls on — COROS
/// Intensity Trend status derived from the acute:chronic ratio.
export interface TrainingStatus {
  /// `decreasing` | `resuming` | `performance` | `maintaining` | `optimized`
  /// | `excessive` — the machine key for the current zone.
  state: string;
  /// Short headline shown on the chart, e.g. "Optimized".
  label: string;
  /// One sentence quoting the runner's actual ratio.
  reason: string;
  /// Shared tone vocabulary: `good` | `warn` | `bad`.
  tone: string;
  /// The acute:chronic ratio this verdict was derived from.
  ratio: number;
}

/// The decision boundaries behind every zone in the app, editable in Settings.
/// They cut the acute:chronic ratio into COROS's six Intensity Trend bands.
/// The backend clamps and orders these on read, so the values that arrive here
/// are always valid and the UI must never re-derive them.
export interface Thresholds {
  /// Top of the Decreasing band; below it, base fitness is declining.
  ratioDecreasingHi: number;
  /// Top of the shared Resuming/Performance band; above it, Maintaining.
  ratioResumingHi: number;
  /// Top of the Maintaining band; above it, Optimized.
  ratioMaintainingHi: number;
  /// Top of the Optimized band; above it reads as Excessive. Also the point
  /// where injury risk starts to climb.
  ratioOptimizedHi: number;
  /// Recovery score at or below this reads as Exhausted.
  recoveryExhausted: number;
  /// At or below this: Fatigued.
  recoveryFatigued: number;
  /// At or below this: Normal; above it is Fresh.
  recoveryNormal: number;
  /// How many increasing weeks before a down week (COROS uses a 3:1 cycle).
  buildWeeks: number;
  /// A down week targets this fraction of the chronic load.
  downWeekFactor: number;
  /// How far (s/km) reps may miss the prescribed range and still count as
  /// in range — the margin between GPS noise and a miss you could feel.
  paceToleranceSecKm: number;
  /// How many quality sessions in a row, all out of range the same way,
  /// before a plan adjustment is offered. One bad session is never a trend.
  paceStreak: number;
}

/// Which recovery band a score falls in.
export interface RecoveryBand {
  state: string;
  label: string;
  tone: string;
  /// Inclusive score range of the band, so a legend can print it.
  min: number;
  max: number;
}

/// This week's 7-day load target, from the 3:1 build cycle.
export interface LoadRecommendation {
  /// Target 7-day training load in COROS load units.
  target: number;
  /// True when this is the recovery week of the cycle.
  downWeek: boolean;
  /// 1-based position of the current week in the cycle.
  weekInCycle: number;
  cycleLength: number;
  note: string;
}

export type RaceDistance = "5K" | "10K" | "21K" | "42K";
export type IncreaseMethod = "conservative" | "moderate" | "ambitious";
export type PlanStatus = "draft" | "active" | "pushed" | "completed" | "aborted";
export type WorkoutKind = "rest" | "easy" | "tempo" | "intervals" | "longRun" | "race";

/// What a plan is built for, which decides its shape rather than its label:
/// only a race build gets a taper and a race day, only a build ramps volume,
/// and a maintenance block holds the runner's current mileage.
export type PlanKind = "race" | "improvement" | "maintenance";

/// Progress of a plan push/cancel toward the watch, emitted on `plan://sync`.
/// COROS takes a whole plan in one call, so the percentage tracks the steps
/// on this side of that call rather than individual sessions sent.
export interface PlanSyncProgress {
  op: "push" | "cancel";
  percent: number;
  stage: string;
}

export interface PlanRequest {
  distance: RaceDistance;
  /// What the plan is for; the backend defaults an absent kind to "race".
  kind: PlanKind;
  targetDate: string;
  thresholdSecKm: number;
  weeklyFrequency: number;
  longrunDay: number;
  increase: IncreaseMethod;
  /// Current weekly mileage (km); null lets the backend derive it from syncs.
  currentWeeklyKm: number | null;
}

/// Device-derived starting values for the create-plan form.
export interface PlanDefaults {
  /// Mean of the last up-to-4 *complete* weeks; null until a week has finished.
  currentWeeklyKm: number | null;
  thresholdSecKm: number | null;
  predict5kS: number | null;
  predict10kS: number | null;
  predictHalfS: number | null;
  predictMarathonS: number | null;
  /// Distance logged in the week currently in progress.
  thisWeekKm: number;
  /// Every running kilometre synced so far.
  totalKm: number;
  /// Complete weeks the average is built from (0..4); 0 means no history.
  weeksObserved: number;
  /// ISO timestamp of the last watch sync.
  lastSync: string | null;
  /// Unresolved pain, so the create-plan form can warn before committing
  /// to a plan built on top of an injury.
  activePain: PainEntry | null;
}

export interface TrainingPlan {
  id: string;
  distance: RaceDistance;
  /// What the plan is built for; "race" for plans stored before kinds existed.
  kind: PlanKind;
  targetDate: string;
  status: PlanStatus;
  weeklyFrequency: number;
  longrunDay: number;
  increase: IncreaseMethod;
  thresholdSecKm: number;
  createdAt: string;
  pushedId: string | null;
  /// Weekly mileage (km) the plan was generated from; null for older plans.
  baselineWeeklyKm: number | null;
  /// How far (s/km) the quality-pace ranges have been moved by an accepted
  /// pacing offer. Negative means sharpened; 0 means never adjusted.
  qualityOffsetSecKm: number;
  /// The first date the offset applies to. Sessions before it keep the range
  /// they were run against, which is also what resets the evidence streak.
  qualityOffsetFrom: string | null;
}

export interface PlanWorkout {
  id: string;
  planId: string;
  date: string;
  /** The day the plan generated this session into. Never edited, so a
   *  reschedule stays reversible and visible: `date !== plannedDate` is what
   *  makes a session a deviation. */
  plannedDate: string;
  kind: WorkoutKind;
  title: string;
  distanceM: number | null;
  durationMin: number | null;
  paceLoSKm: number | null;
  paceHiSKm: number | null;
  done: boolean;
  /** `skipped` means the runner decided not to run it — which is not the same
   *  as a lapse, and is why adherence can tell the two apart. */
  status: "planned" | "skipped";
  /** When the runner moved it; null if it has never moved. */
  movedAt: string | null;
}

/// One session a "reset this week" preview is offering to put back.
export interface WeekDeviation {
  workoutId: string;
  title: string;
  /** Where the session sits now. */
  from: string;
  /** The day the plan generated it for. */
  to: string;
  /** False when the session is run, raced, skipped, or its generated day has
   *  already passed — the reset will refuse it, so the sheet says so. */
  restorable: boolean;
  /** Restoring it would land it on today. */
  isToday: boolean;
}

/// A banked run's frozen score snapshot (see the backend's `run_scores`).
/// `riskDelta` is the run's own impact on injury risk: risk with the session
/// minus risk without it. Every run has one.
export interface RunScore {
  activityId: number;
  workoutId: string | null;
  load: number | null;
  risk: number;
  riskDelta: number;
}

export interface CalendarEvent {
  id: string;
  workoutId: string | null;
  activityId: number | null;
  date: string;
  kind: WorkoutKind | "completed";
  title: string;
  distanceM: number | null;
  durationMin: number | null;
  notes: string;
  start: string;
  end: string;
  /** Planned sessions are all-day until matched to a recorded run. */
  allDay: boolean;
  /** The runner dragged this session off its generated day. */
  moved: boolean;
  status: "planned" | "skipped";
  /** The day the plan generated it into; null on a run with no plan behind it. */
  plannedDate: string | null;
  /** Its day passed without a run or a skip. Derived in the backend so every
   *  surface counts a lapse identically. */
  missed: boolean;
  /** Why the plan softened this session on its own, when it did. The chip
   *  tooltip, the agenda row and the sheet all read the same string: an
   *  adjustment nobody explained reads as a bug. */
  adjusted: string | null;
}

export interface CalendarPublishing {
  enabled: boolean;
  uploadUrl: string;
  subscriptionUrl: string;
  username: string;
  hasPassword: boolean;
  lastPublished: string | null;
  lastError: string | null;
}

/// The cross-device plan channel. `canPublish` is decided by the platform, not
/// the user: the machine that builds plans is the one that publishes them.
export interface PlanSync {
  enabled: boolean;
  gistId: string | null;
  hasToken: boolean;
  canPublish: boolean;
  lastPushed: string | null;
  lastPulled: string | null;
  installedRev: number | null;
  lastError: string | null;
}

/// The only part of the config the interface owns: everything else is derived
/// from the platform or recorded by the backend.
export interface PlanSyncPatch {
  enabled: boolean;
}

export interface PlanSyncStatus {
  config: PlanSync;
  /// False until COROS has been connected — that account id is how the two
  /// devices recognise each other, so without it there is no channel.
  accountLinked: boolean;
  /// The anonymous channel code. Shown so both devices can be compared when
  /// a sync appears to do nothing.
  code: string | null;
}

export interface PullOutcome {
  installed: boolean;
  planName: string;
  workouts: number;
  /// Set when a different plan is already on this device and the import was
  /// not forced — the caller asks before overwriting.
  conflict: string | null;
}

/// How the published plan relates to this device, as reported by a read-only
/// check. The writer reads `upToDate` as "published" and `unpublished` as
/// "not yet pushed"; the importer as "up to date" and "import available".
export type SyncPillStatus =
  | "upToDate"
  | "unpublished"
  | "updateAvailable"
  | "otherPlan"
  | "importAvailable";

export interface RemotePlan {
  status: SyncPillStatus;
  /// The published plan described the way an import confirmation names it.
  planName: string;
  workouts: number;
  /// Seconds since epoch when it was published.
  syncedAt: number;
}

export interface DashboardScores {
  trainingScore: number;
  injuryRisk: number;
  recoveryScore: number;
  globalScore: number;
  notes: string[];
}

/// One day of the readiness sub-scores (0..100), oldest→newest, for the
/// sparkline cards under the readiness hero.
export interface ScorePoint {
  date: string;
  training: number;
  recovery: number;
  /// Higher = riskier, same polarity as the headline injury risk.
  risk: number;
}

export type TimeRange = "1d" | "7d" | "1mo";

/// One day of wearable data (COROS watch / ring).
export interface HealthDaily {
  date: string;
  steps: number | null;
  avgHr: number | null;
  restingHr: number | null;
  stressAvg: number | null;
  sleepScore: number | null;
  sleepMin: number | null;
  deepMin: number | null;
  remMin: number | null;
  hrvAvg: number | null;
}

/// Aggregated training stats for the selected time range.
export interface RangeStats {
  distanceM: number;
  durationS: number;
  sessions: number;
  avgPaceSKm: number | null;
  avgHr: number | null;
}

/// One heuristic contributing to the injury-risk percentage.
export interface RiskFactor {
  label: string;
  detail: string;
  /// 0..1 severity of this factor on its own scale — 0 = quiet, 1 = maxed out.
  contribution: number;
}

export interface DashboardPayload {
  scores: DashboardScores;
  /// Per-day sub-score trend for the readiness cards, spanning the range.
  scoreHistory: ScorePoint[];
  fitness: FitnessAssessment | null;
  load: TrainingLoad | null;
  /// Daily acute/chronic load for the training-status chart, oldest→newest.
  loadHistory: LoadPoint[];
  /// Which side of the boundaries recent training falls on.
  trainingStatus: TrainingStatus | null;
  /// The boundaries those zones were computed with — the chart draws its bands
  /// from here so it can never disagree with Settings.
  thresholds: Thresholds;
  /// Which recovery band the recovery score falls in.
  recoveryBand: RecoveryBand | null;
  /// This week's 7-day load target from the build cycle.
  loadRecommendation: LoadRecommendation | null;
  activities: Activity[];
  range: TimeRange;
  stats: RangeStats;
  health: HealthDaily[];
  riskFactors: RiskFactor[];
  /// Most recent unresolved pain entry, for the Today-tab banner.
  activePain: PainEntry | null;
  /// An unanswered plan-change offer, surfaced with Accept right there.
  pendingProposal: PlanProposal | null;
}

export interface SyncReport {
  activitiesNew: number;
  healthDays: number;
  fitnessUpdated: boolean;
  tracksDownloaded: number;
  messages: string[];
}

export type ThemeChoice = "system" | "light" | "dark";

export type PaletteId = "amethyst" | "glacier" | "evergreen" | "sakura" | "ember";

export interface Settings {
  pmtilesPath: string | null;
  lastProvider: ProviderKind | null;
  /// "system" | "light" | "dark". Null behaves as "system".
  theme: ThemeChoice | null;
  /// Colour palette id; null means the default (Amethyst Dusk).
  palette?: PaletteId | null;
  /// Injury assistant: an OpenAI-compatible chat endpoint.
  llmEnabled: boolean;
  llmEndpoint: string | null;
  llmModel: string | null;
  /// Editable decision boundaries. The backend normalises them on read.
  thresholds: Thresholds;
}

// ---- injury journal --------------------------------------------------------

/// One pain report typed by the runner. `bodyPart` is a coarse region key
/// from the body map ("knee.left", "lower_back", "other").
export interface PainEntry {
  id: string;
  bodyPart: string;
  /// Runner-rated severity, 0..=10.
  intensity: number;
  /// When the runner says it started — not when it was logged.
  startedOn: string;
  notes: string;
  createdAt: string;
  /// Set once the runner confirms the pain went away. null = still hurting.
  resolvedOn: string | null;
  /// When the resolution follow-up was last asked (once per episode).
  followupAt: string | null;
}

/// One turn of the assistant thread for one pain entry.
export interface PainMessage {
  id: string;
  bodyPart: string;
  /// The journal entry whose chat this turn belongs to.
  entryId: string | null;
  role: "user" | "assistant";
  content: string;
  ts: string;
}

/// A single concrete change to one planned workout; proposals are lists of
/// these so an offer reads as a reviewable diff.
export interface WorkoutEdit {
  workoutId: string;
  kind: WorkoutKind;
  title: string;
  distanceM: number | null;
  durationMin: number | null;
  paceLoSKm: number | null;
  paceHiSKm: number | null;
  /** Date + original kind/title/distance at offer time, for the was → now diff. */
  date: string | null;
  oldKind: WorkoutKind | null;
  oldTitle: string | null;
  oldDistanceM: number | null;
}

export type ProposalStatus = "pending" | "applied" | "dismissed" | "superseded";

/// What generated an offer. Pain offers rewrite individual days; pacing offers
/// move the plan's quality-pace ranges. They share the accept/dismiss lifecycle
/// but belong on different pages.
export type ProposalSource = "pain" | "pacing";

/// Where a session's reps landed against the prescribed range.
export type Execution = "ahead" | "in" | "behind";

/// One completed quality session graded against the range it prescribed.
/// `actualSkm` is the pace held over the reps, never the session average.
export interface SessionReview {
  workoutId: string;
  date: string;
  kind: WorkoutKind;
  title: string;
  bandLoSKm: number;
  bandHiSKm: number;
  actualSKm: number | null;
  measuredM: number;
  execution: Execution;
}

/// One line of the plan receipt: the decision the runner made, and what it
/// did to the plan. Ordered by influence, so the list reads as the plan's
/// basis rather than a summary of its contents.
export interface ReceiptLine {
  input: string;
  sentence: string;
}

/// Where recent quality sessions landed, and how many in a row a plan
/// adjustment needs. Shown whether or not an offer exists yet.
export interface PacingReport {
  reviews: SessionReview[];
  streak: number;
  direction: Execution | null;
  needed: number;
  triggered: boolean;
}

/// What a stretch of a recorded run was for. Decides how a lap is drawn: only
/// a work lap is given a target cap, an easy or recovery lap is not.
export type LapKind = "easy" | "work" | "recovery" | "kilometre";

/// How one lap landed against the range it asked for. Only the work asks for a
/// range, so a warm-up, cool-down, or jog recovery never carries a verdict.
export type LapVerdict = "ahead" | "in" | "behind";

/// One stretch of a recorded run, cut where the prescription says the shape of
/// the run changes. `bandLoSKm`/`bandHiSKm` are set on the work only, which is
/// why an easy or recovery lap is drawn without a target cap.
export interface LapSlice {
  index: number;
  label: string;
  kind: LapKind;
  startM: number;
  meters: number;
  durationS: number;
  avgPaceSKm: number | null;
  avgHr: number | null;
  bandLoSKm: number | null;
  bandHiSKm: number | null;
  verdict: LapVerdict | null;
}

/// One kilometre of a run, cut by the distance rather than by the plan.
/// `deltaPrevSKm` is against the PREVIOUS kilometre — positive means the pace
/// dropped. The UI must name that reference in the column header.
export interface KmSplit {
  km: number;
  meters: number;
  durationS: number;
  avgPaceSKm: number | null;
  avgHr: number | null;
  deltaPrevSKm: number | null;
  /// Set only under a prescribed work lap — a kilometre under a warm-up,
  /// recovery, or cool-down stays unmarked.
  bandLoSKm: number | null;
  bandHiSKm: number | null;
}

/// The review of one recorded run: the run cut into laps and kilometres.
/// Every run gets one; `prescribed` says whether the laps carry target caps.
export interface ActivityReview {
  activity: Activity;
  hasTrack: boolean;
  /** The route, so a review surface can draw the map without a second fetch. */
  track: TrackPoint[];
  workoutId: string | null;
  prescribed: boolean;
  laps: LapSlice[];
  splits: KmSplit[];
}

/// An offered plan change, generated deterministically from the pain entry
/// and the current plan. The assistant can explain it, never author it.
export interface PlanProposal {
  id: string;
  bodyPart: string;
  signature: string;
  summary: string;
  reason: string;
  edits: WorkoutEdit[];
  status: ProposalStatus;
  createdAt: string;
  actedAt: string | null;
  source: ProposalSource;
  /// For a pacing offer: the offset to store when it is accepted, and the
  /// first date it applies to. Null on pain offers.
  planQualityOffsetSecKm: number | null;
  planQualityOffsetFrom: string | null;
}

/// What the assistant is configured to do right now (never the key itself).
export interface LlmStatus {
  configured: boolean;
  hasKey: boolean;
  endpoint: string | null;
  model: string | null;
}
