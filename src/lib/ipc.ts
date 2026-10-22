import { invoke } from "@tauri-apps/api/core";
import type {
  Activity,
  ActivityDetail,
  CalendarEvent,
  CalendarPublishing,
  DashboardPayload,
  HeatPoint,
  LlmStatus,
  PainEntry,
  PainMessage,
  PacingReport,
  PlanDefaults,
  PlanProposal,
  PlanRequest,
  PlanWorkout,
  ProviderKind,
  ProviderStatus,
  RunScore,
  Settings,
  SyncReport,
  TimeRange,
  TrainingPlan,
} from "./types";

/** True when running inside a Tauri webview. */
export const inTauri = "__TAURI_INTERNALS__" in window;

// Mark every page after the first as "arrived by navigation" so the CSS can
// skip the per-section load entrance (see `html.nav-in` in app.css). The
// cross-document view transition snapshots the incoming page mid-entrance,
// which is what caused the blank-then-pop flicker between pages. This runs at
// module-eval — before first paint and before the transition's snapshot — and
// is a file module, so it stays inside the production CSP (no inline script).
try {
  if (sessionStorage.getItem("runalytics.nav.v1")) {
    document.documentElement.classList.add("nav-in");
  }
  sessionStorage.setItem("runalytics.nav.v1", "1");
} catch {
  /* storage blocked (private mode): entrances still work, just no skip. */
}

type Command = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

/**
 * All data comes from the Rust backend talking to the COROS API — there is
 * no mock layer. Outside Tauri (plain browser) the commands fail with a
 * clear message pointing at `npm run tauri dev`.
 */
const call: Command = async <T>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
  if (!inTauri) {
    throw new Error(
      "Runalytics needs the desktop app for data — run `npm run tauri dev` and open it there.",
    );
  }
  return invoke<T>(cmd, args);
};

export const tauri = {
  providerStatus: () => call<ProviderStatus[]>("provider_status"),
  checkProviderApi: (provider: ProviderKind) =>
    call<string[]>("check_provider_api", { provider }),
  connectProvider: (provider: ProviderKind) =>
    call<string>("connect_provider", { provider }),
  disconnectProvider: (provider: ProviderKind) =>
    call<void>("disconnect_provider", { provider }),
  syncNow: (provider: ProviderKind) => call<SyncReport>("sync_now", { provider }),
  syncStatus: () => call<boolean>("sync_status"),
  dashboard: (range: TimeRange) => call<DashboardPayload>("dashboard", { range }),
  heatPoints: () => call<HeatPoint[]>("heat_points"),
  activityDetail: (activityId: number, range: TimeRange) =>
    call<ActivityDetail>("activity_detail", { activityId, range }),
  allActivities: () => call<Activity[]>("all_activities"),
  runScores: () => call<RunScore[]>("run_scores"),
  createPlan: (request: PlanRequest) =>
    call<[TrainingPlan, PlanWorkout[]]>("create_plan", { request }),
  planDefaults: () => call<PlanDefaults>("plan_defaults"),
  currentPlan: () =>
    call<[TrainingPlan, PlanWorkout[]] | null>("current_plan"),
  pushPlan: (provider: ProviderKind) =>
    call<string>("push_plan_to_provider", { provider }),
  abortPlan: () => call<void>("abort_plan"),
  workoutsBetween: (from: string, to: string) =>
    call<PlanWorkout[]>("workouts_between", { from, to }),
  icsPath: () => call<string>("ics_path"),
  publishCalendar: () => call<string>("publish_calendar"),
  calendarEvents: (from: string, to: string) =>
    call<CalendarEvent[]>("calendar_events", { from, to }),
  calendarPublishing: () => call<CalendarPublishing>("calendar_publishing"),
  saveCalendarPublishing: (config: CalendarPublishing, password: string | null) =>
    call<CalendarPublishing>("save_calendar_publishing", { config, password }),
  matchCalendarActivity: (workoutId: string, activityId: number) =>
    call<void>("match_calendar_activity", { workoutId, activityId }),
  moveWorkout: (workoutId: string, date: string) =>
    call<void>("move_workout", { workoutId, date }),
  getSettings: () => call<Settings>("get_settings"),
  saveSettings: (settings: Settings) => call<void>("save_settings", { settings }),
  // ---- injury journal ----
  savePainEntry: (bodyPart: string, intensity: number, startedOn: string, notes: string) =>
    call<[PainEntry, PlanProposal | null]>("save_pain_entry", {
      bodyPart,
      intensity,
      startedOn,
      notes,
    }),
  deletePainEntry: (id: string) => call<void>("delete_pain_entry", { id }),
  resolvePain: (id: string, resolvedOn: string | null) =>
    call<void>("resolve_pain", { id, resolvedOn }),
  markPainFollowup: (id: string) => call<void>("mark_pain_followup", { id }),
  painEntries: () => call<PainEntry[]>("pain_entries"),
  painDays: (from: string, to: string) =>
    call<[string, number][]>("pain_days", { from, to }),
  painByRegion: (days: number) =>
    call<[string, number][]>("pain_by_region", { days }),
  // ---- plan proposals ----
  pendingProposal: () => call<PlanProposal | null>("pending_proposal"),
  pendingPacingProposal: () =>
    call<PlanProposal | null>("pending_pacing_proposal"),
  pacingReport: () => call<PacingReport | null>("pacing_report"),
  proposals: (limit: number) => call<PlanProposal[]>("proposals", { limit }),
  applyProposal: (provider: ProviderKind, proposalId: string) =>
    call<PlanWorkout[]>("apply_proposal", { provider, proposalId }),
  dismissProposal: (proposalId: string, reason: string | null) =>
    call<void>("dismiss_proposal", { proposalId, reason }),
  // ---- injury assistant ----
  llmStatus: () => call<LlmStatus>("llm_status"),
  setLlmApiKey: (key: string) => call<void>("set_llm_api_key", { key }),
  clearLlmApiKey: () => call<void>("clear_llm_api_key"),
  askAssistant: (entryId: string, question: string) =>
    call<PainMessage>("ask_assistant", { entryId, question }),
  painMessages: (entryId: string) =>
    call<PainMessage[]>("pain_messages", { entryId }),
  clearPainMessages: (entryId: string) =>
    call<void>("clear_pain_messages", { entryId }),
};

export function fmtPace(sKm: number | null | undefined): string {
  if (sKm == null || !isFinite(sKm)) return "—";
  const m = Math.floor(sKm / 60);
  const s = Math.round(sKm % 60);
  return `${m}:${String(s).padStart(2, "0")}`;
}

export function fmtDuration(s: number): string {
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  return h > 0 ? `${h}h ${String(m).padStart(2, "0")}m` : `${m}m`;
}

export function fmtKm(m: number): string {
  return `${(m / 1000).toFixed(1)} km`;
}

export function fmtDate(iso: string): string {
  return new Date(iso).toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
  });
}

/// Local calendar day as `YYYY-MM-DD`. Never use
/// `date.toISOString().slice(0, 10)` for this: toISOString serialises in UTC,
/// so in UTC+ timezones a local midnight lands on the *previous* day — weeks
/// silently start on Sunday and streaks lose a day.
export function ymdLocal(d: Date): string {
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${m}-${day}`;
}

export function fmtClock(s: number | null): string {
  if (s == null) return "—";
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = Math.round(s % 60);
  return h > 0
    ? `${h}:${String(m).padStart(2, "0")}:${String(sec).padStart(2, "0")}`
    : `${m}:${String(sec).padStart(2, "0")}`;
}

export function parsePace(text: string): number | null {
  const m = text.trim().match(/^(\d{1,2}):(\d{1,2})$/);
  if (!m) return null;
  const mm = parseInt(m[1], 10);
  const ss = parseInt(m[2], 10);
  if (ss > 59) return null;
  return mm * 60 + ss;
}

export function el<T extends HTMLElement = HTMLElement>(id: string): T {
  const node = document.getElementById(id);
  if (!node) throw new Error(`missing element #${id}`);
  return node as T;
}

export function fail(err: unknown): void {
  const msg = typeof err === "string" ? err : String(err);
  console.error(err);
  // Non-blocking toast instead of an alert storm.
  let toast = document.getElementById("toast");
  if (!toast) {
    toast = document.createElement("div");
    toast.id = "toast";
    toast.setAttribute("role", "status");
    toast.setAttribute("aria-live", "polite");
    document.body.appendChild(toast);
  }
  toast.textContent = msg;
  toast.classList.add("show");
  window.clearTimeout((toast as HTMLElement & { _t?: number })._t);
  (toast as HTMLElement & { _t?: number })._t = window.setTimeout(
    () => toast?.classList.remove("show"),
    6000,
  );
}
