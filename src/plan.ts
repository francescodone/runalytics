import { tauri, el, fail, fmtPace, fmtClock, fmtDate, parsePace, inTauri, ymdLocal } from "./lib/ipc";
import { listen } from "@tauri-apps/api/event";
import { initSyncIndicator } from "./lib/sync";
import { withBusy } from "./lib/feedback";
import { scrollIntoViewMotion } from "./lib/motion";
import { initGroupProgress, initScrollReveal } from "./lib/reveal";
import { escapeHtml, openWorkout } from "./lib/workout-detail";
import { qualityBand } from "./lib/plan-bands";
import { requirePlanOrRedirect, initNavGate } from "./lib/navgate";
// Side-effect import: paints the saved colour scheme before first paint.
import "./lib/theme";
import "./lib/navigation";
import type {
  Execution,
  IncreaseMethod,
  PacingReport,
  PlanDefaults,
  PlanKind,
  PlanProposal,
  PlanSyncProgress,
  PlanWorkout,
  RaceDistance,
  SessionReview,
  TrainingPlan,
} from "./lib/types";

const KIND_LABEL: Record<string, string> = {
  easy: "Easy",
  tempo: "Tempo",
  intervals: "Intervals",
  // Two spellings, one kind: PlanWorkout.kind arrives as "longrun" (serde
  // lowercase), the calendar's event_kind emits "longRun". Label both.
  longRun: "Long run",
  longrun: "Long run",
  race: "Race",
  rest: "Rest",
};

// --- create-plan form helpers -------------------------------------------

/// Device-derived form defaults (weekly mileage, threshold, race predictions),
/// fetched once on load; empty when the watch hasn't reported anything yet.
let defaults: PlanDefaults = {
  currentWeeklyKm: null,
  thresholdSecKm: null,
  predict5kS: null,
  predict10kS: null,
  predictHalfS: null,
  predictMarathonS: null,
  thisWeekKm: 0,
  totalKm: 0,
  weeksObserved: 0,
  lastSync: null,
  activePain: null,
};

/// True while the threshold field still holds the value the watch reported —
/// the hint says "from your watch" until the user types over it.
let thresholdFromDevice = false;

const DISTANCE_M: Record<RaceDistance, number> = {
  "5K": 5_000,
  "10K": 10_000,
  "21K": 21_097.5,
  "42K": 42_195,
};

function devicePrediction(d: RaceDistance): number | null {
  return d === "5K"
    ? defaults.predict5kS
    : d === "10K"
      ? defaults.predict10kS
      : d === "21K"
        ? defaults.predictHalfS
        : defaults.predictMarathonS;
}

/**
 * Current weekly mileage as the plan will see it: whatever is in the form
 * field, falling back to the average the watch reported. Read live so the
 * preview follows the user's edits.
 */
function baselineWeeklyKm(): number | null {
  const raw = parseFloat(el<HTMLInputElement>("plan-mileage").value);
  if (isFinite(raw) && raw > 0) return raw;
  return defaults.currentWeeklyKm;
}

/**
 * Riegel time prediction anchored on threshold pace: threshold is the pace
 * held for ~1 h, so T(race) = 3600 × (D / D_1h) ^ 1.06 with D_1h the distance
 * covered at threshold in one hour.
 */
function raceTimeS(thresholdSecKm: number, distance: RaceDistance): number {
  const oneHourKm = 3600 / thresholdSecKm;
  // DISTANCE_M is metres, oneHourKm is kilometres — the ratio needs one unit.
  const distanceKm = DISTANCE_M[distance] / 1000;
  return Math.round(3600 * Math.pow(distanceKm / oneHourKm, 1.06));
}

/// "Est. times" rows: two *current-fitness* predictions of the same race —
/// the watch's own model vs. the Riegel projection of the threshold pace.
/// Neither projects training gains (the plan adds no fitness model), so the
/// gap between them is model disagreement, never "faster" — an earlier
/// "(x faster)" note read as a promised improvement and was pure noise.
/// Empty string when nothing is known.
function estTimesHtml(distance: RaceDistance, thresholdSecKm: number | null): string {
  const today = devicePrediction(distance);
  const race = thresholdSecKm != null ? raceTimeS(thresholdSecKm, distance) : null;
  if (today == null && race == null) return "";
  return `<div class="est-times inset">
    <div class="est-row"><span class="est-title">Est. ${distance} today</span>
      <span class="est-val">${today != null ? fmtClock(today) : "—"}</span>
      <span>${today != null ? "from your watch" : "sync to get this"}</span></div>
    <div class="est-row"><span class="est-title">Est. ${distance} from threshold</span>
      <span class="est-val">${race != null ? fmtClock(race) : "—"}</span>
      <span>${race != null ? "from your threshold pace" : "set a threshold pace"}</span></div>
  </div>`;
}

function selectedDistance(): RaceDistance {
  const checked = document.querySelector<HTMLInputElement>(
    'input[name="plan-distance"]:checked',
  );
  return (checked?.value ?? "5K") as RaceDistance;
}

function selectedKind(): PlanKind {
  const checked = document.querySelector<HTMLInputElement>(
    'input[name="plan-kind"]:checked',
  );
  return (checked?.value ?? "race") as PlanKind;
}

const KIND_HINT: Record<PlanKind, string> = {
  race: "A race build ramps to a peak, tapers and ends on race day.",
  improvement:
    "An improvement block builds for a set number of weeks and finishes with a time trial over the distance, so you find out what the block bought you.",
  maintenance:
    "A maintenance block holds the mileage you already run — no ramp, no taper. Pick it between blocks, through winter, or while something niggles.",
};

const DISTANCE_LEGEND: Record<PlanKind, string> = {
  race: "Race distance",
  improvement: "Distance to improve on",
  maintenance: "Distance to keep an eye on",
};

const DISTANCE_HINT: Record<PlanKind, string> = {
  race: "The distance the plan builds toward — it sets the peak volume and the long runs.",
  improvement:
    "The time trial at the end of the block is run over this distance, and the plan builds toward it.",
  maintenance:
    "Nothing is built toward, so this only shapes the sessions — the long run grows around this distance.",
};

const DATE_LABEL: Record<PlanKind, string> = {
  race: "Race date",
  improvement: "Block end date",
  maintenance: "Block end date",
};

const DATE_HINT: Record<PlanKind, string> = {
  race: "At least two weeks out.",
  improvement: "The time trial happens on this day.",
  maintenance: "",
};

/// The end date a maintenance block gets from its length: today + N weeks.
/// Computed client-side because the backend takes only a date — and 16 weeks
/// is the cap it enforces for plans without a race.
function maintenanceTargetDate(): string {
  const weeks = parseInt(el<HTMLSelectElement>("plan-block-weeks").value, 10) || 10;
  const end = new Date();
  end.setDate(end.getDate() + weeks * 7);
  return ymdLocal(end);
}

/// The date the plan will actually end with, whichever kind is selected.
function effectiveTargetDate(): string {
  return selectedKind() === "maintenance"
    ? maintenanceTargetDate()
    : el<HTMLInputElement>("plan-target-date").value;
}

/**
 * Show only the fields the selected kind uses. Maintenance has no date to
 * pick (its length is the block selector) and no ramp (so no increase
 * method); the other two share everything but the label on the date.
 */
function applyKind(): void {
  const kind = selectedKind();
  el("plan-kind-hint").textContent = KIND_HINT[kind];
  el("plan-distance-legend").textContent = DISTANCE_LEGEND[kind];
  el("plan-distance-hint").textContent = DISTANCE_HINT[kind];
  el("plan-date-label").textContent = DATE_LABEL[kind];
  el("plan-date-field").hidden = kind === "maintenance";
  // Only maintenance picks a length; the other two pick the date they end on,
  // and a selector that the date already decides would be a second answer.
  el("plan-weeks-field").hidden = kind !== "maintenance";
  el("plan-increase-field").hidden = kind === "maintenance";
  if (kind !== "maintenance") {
    el("plan-date-hint").textContent = DATE_HINT[kind];
  }
}

/**
 * Preview of the backend engine's peak week (engine.rs). Mirrors the engine's
 * actual math: peak target = distance base × the increase method's multiplier
 * (0.85/1.0/1.15), floored at 110 % of the runner's current volume; the ramp
 * then reaches for that peak at the method's weekly cap over the *real*
 * number of build weeks (race, taper and down weeks don't multiply). Labelled
 * "est." — the generated plan is the real answer.
 */
function estimatePeakWeeklyKm(
  distance: RaceDistance,
  increase: string,
  frequency: number,
  weeks: number,
  kind: PlanKind = "race",
): number {
  const base = { "5K": 32, "10K": 42, "21K": 58, "42K": 80 }[distance];
  // engine.rs scales the base ±7.5 % per session away from a 4-session week.
  const freqFactor = Math.min(Math.max(1 + (frequency - 4) * 0.075, 0.85), 1.3);
  const mult = { conservative: 0.85, moderate: 1.0, ambitious: 1.15 }[increase] ?? 1.0;
  const cap = { conservative: 1.05, moderate: 1.08, ambitious: 1.12 }[increase] ?? 1.08;
  const start = baselineWeeklyKm() ?? base * 0.4;
  // A maintenance block holds the volume the runner arrived with: engine.rs
  // sets peak = requested and the ramp ratio to exactly 1, so there is no
  // peak to forecast. Without this branch the preview promised a build on a
  // plan whose whole point is not to build.
  if (kind === "maintenance") return Math.round(Math.max(start ?? 0, 10));
  // The engine never targets a peak below what the runner already does.
  const peak = Math.max(base * freqFactor * mult, start * 1.1);
  const baseline = Math.min(Math.max(start, 10), peak);
  // Count the weeks that really multiply, exactly like engine.rs: no race,
  // taper or down week applies the ramp ratio. Only a race build loses its
  // last week to the race, and only a race build tapers.
  const taperWeeks = kind === "race" ? (distance === "5K" || distance === "10K" ? 1 : 2) : 0;
  let buildWeeks = 0;
  for (let w = 0; w < weeks; w++) {
    const isRace = kind === "race" && w === weeks - 1;
    const tapering = weeks - 1 - w < taperWeeks && !isRace;
    const down = !isRace && !tapering && (w + 1) % 4 === 0;
    if (!isRace && !tapering && !down) buildWeeks++;
  }
  const steps = Math.max(buildWeeks - 1, 1);
  let ratio = Math.min(Math.pow(peak / baseline, 1 / steps), cap);
  if (weeks > 3) ratio = Math.max(ratio, 1.02);
  return Math.round(Math.max(baseline, Math.min(peak, baseline * Math.pow(ratio, steps))));
}

function setFieldError(input: HTMLInputElement | null, msg: string | null): void {
  if (!input) return;
  if (msg) input.setAttribute("aria-invalid", "true");
  else input.removeAttribute("aria-invalid");
}

function setPlanError(msg: string | null): void {
  const node = el("plan-error");
  node.textContent = msg ?? "";
  node.hidden = !msg;
}

function daysUntilRace(dateStr: string): number {
  const target = new Date(`${dateStr}T00:00:00`);
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  return Math.round((target.getTime() - today.getTime()) / 86_400_000);
}

/**
 * Fill the threshold explainer's table with the actual bands this threshold
 * produces, mirroring engine.rs: easy = threshold + 45-75 s, tempo = threshold
 * ±5 s, intervals centred on predicted 3K pace, marathon pace from the same
 * Riegel model. The generic text stays until a pace is typed — the table is a
 * consequence of the number, so it only becomes concrete with one.
 */
function updatePaceBands(thresholdSecKm: number | null): void {
  const set = (id: string, text: string) => {
    el(id).textContent = text;
  };
  if (thresholdSecKm == null) {
    set("pace-easy", "threshold + 45\u201375 s");
    set("pace-tempo", "threshold \u00b1 5 s");
    set("pace-intervals", "about 25 s faster than threshold");
    set("pace-mp", "marathon pace, from the same model");
    return;
  }
  // engine.rs: race_pace = threshold * (D / D_1h) ^ 0.06, always slower than
  // threshold, since threshold is by definition a one-hour pace.
  const oneHourKm = 3600 / thresholdSecKm;
  const derived = (metres: number) =>
    thresholdSecKm * Math.pow(metres / 1000 / oneHourKm, 0.06);
  const mp = derived(42_195);
  const interval = derived(3_000);
  set("pace-easy", `${fmtPace(thresholdSecKm + 45)}\u2013${fmtPace(thresholdSecKm + 75)}/km`);
  set("pace-tempo", `${fmtPace(thresholdSecKm - 5)}\u2013${fmtPace(thresholdSecKm + 5)}/km`);
  set("pace-intervals", `${fmtPace(interval - 5)}\u2013${fmtPace(interval + 5)}/km`);
  set("pace-mp", `${fmtPace(mp - 8)}\u2013${fmtPace(mp + 8)}/km`);
}

/**
 * Continuous feedback: as soon as the plan has an end date, show its shape
 * (weeks, sessions, estimated peak) before anything is generated. The copy
 * follows the kind — a maintenance block holds volume rather than building to
 * a peak, and only a plan that ends on a start line gets time predictions.
 */
function updatePlanPreview(): void {
  const preview = el("plan-preview");
  const thresholdInput = el<HTMLInputElement>("plan-threshold");
  const hint = el("plan-threshold-hint");
  const kind = selectedKind();
  const paceOk = parsePace(thresholdInput.value) != null;
  setFieldError(thresholdInput, thresholdInput.value && !paceOk ? "bad" : null);
  updatePaceBands(paceOk ? parsePace(thresholdInput.value) : null);
  hint.textContent =
    thresholdInput.value && !paceOk
      ? "Use min:sec, e.g. 4:45"
      : thresholdFromDevice
        ? "min/km — from your watch"
        : "min/km — e.g. 4:45";

  const dateStr = effectiveTargetDate();
  if (!dateStr) {
    preview.innerHTML = `<p class="inset-hint">Pick when this plan ends to see its shape.</p>`;
    return;
  }
  const days = daysUntilRace(dateStr);
  if (days < 15) {
    preview.innerHTML = `<p class="inset-hint inset-bad">That is ${days} day${days === 1 ? "" : "s"} out — plans need at least two weeks.</p>`;
    return;
  }
  const weeks = Math.ceil(days / 7);
  // engine.rs refuses a plan without a race date longer than the provider's
  // 16-week window; say so here rather than on submit.
  if (kind !== "race" && weeks > 16) {
    preview.innerHTML = `<p class="inset-hint inset-bad">A plan without a race date runs at most 16 weeks — that is the longest block COROS will hold.</p>`;
    return;
  }
  const freq = parseInt(el<HTMLInputElement>("plan-frequency").value, 10) || 4;
  const peak = estimatePeakWeeklyKm(
    selectedDistance(),
    el<HTMLSelectElement>("plan-increase").value,
    freq,
    weeks,
    kind,
  );
  const start = baselineWeeklyKm();
  const stat = (value: string, label: string) =>
    `<div class="inset-stat"><span class="inset-value">${value}</span><span class="eyebrow">${label}</span></div>`;
  const startStat =
    start != null ? stat(`${start.toFixed(0)} km`, "Current week") : stat("—", "Current week");
  const peakStat =
    kind === "maintenance"
      ? stat(`≈ ${peak} km`, "Held every week")
      : stat(`≈ ${peak} km`, "Peak week (est.)");
  const weekLabel =
    kind === "race" ? "Weeks to race" : kind === "improvement" ? "Weeks in the block" : "Weeks";
  preview.innerHTML = `
    ${stat(`${weeks}`, weekLabel)}
    ${startStat}
    ${stat(`${weeks * freq}`, "Sessions")}
    ${peakStat}
    ${kind === "maintenance" ? "" : estTimesHtml(selectedDistance(), paceOk ? parsePace(thresholdInput.value) : null)}`;
}

const PLAN_KIND_LABEL: Record<PlanKind, string> = {
  race: "Race build",
  improvement: "Improvement block",
  maintenance: "Maintenance block",
};

/// Injury-risk impact per done session, keyed by workout id — how many points
/// the run itself moved the runner's risk (risk with the session minus risk
/// without it), the same number the calendar description prints. Refreshed
/// with the plan; empty outside the desktop app or before any run has a
/// snapshot, which simply renders no badge.
let riskDeltaByWorkout = new Map<string, number>();

/// The small signed badge on a done session. An empty string means no
/// snapshot exists for this session yet (a future week, or a sync that has
/// not run) — not a zero.
function riskDeltaBadge(workoutId: string): string {
  const d = riskDeltaByWorkout.get(workoutId);
  if (d == null) return "";
  const tone = d > 0 ? "risk-up" : "risk-down";
  return `<span class="risk-delta ${tone}">${d > 0 ? "+" : ""}${d}%</span>`;
}

function renderPlan(plan: TrainingPlan, workouts: PlanWorkout[]): void {
  // Only a plan that ends on a start line has a finish time worth predicting:
  // a maintenance block never races its target date, so a predicted 42K there
  // would be a number attached to a day nothing happens on.
  const predicts = plan.kind !== "maintenance";
  el("plan-status").innerHTML = `<span class="chip status">${plan.status}</span>
    ${PLAN_KIND_LABEL[plan.kind]} · ${plan.distance} on ${plan.targetDate}
    · threshold ${fmtPace(plan.thresholdSecKm)}/km
    · ${plan.weeklyFrequency}×/week
    ${plan.baselineWeeklyKm != null ? `· from ${plan.baselineWeeklyKm.toFixed(0)} km/wk` : ""}${plan.pushedId ? ` · pushed (${plan.pushedId})` : ""}
    ${predicts ? estTimesHtml(plan.distance, plan.thresholdSecKm) : ""}`;

  // Group workouts by ISO week starting Monday. The key is built from local
  // date parts — toISOString() serialises in UTC and would label every week
  // Sunday in UTC+ timezones.
  const weeks = new Map<string, PlanWorkout[]>();
  for (const w of workouts) {
    const d = new Date(w.date + "T00:00:00");
    const day = (d.getDay() + 6) % 7; // Mon=0
    d.setDate(d.getDate() - day);
    const key = ymdLocal(d);
    (weeks.get(key) ?? weeks.set(key, []).get(key)!).push(w);
  }

  const todayKey = ymdLocal(new Date());
  const rows = [...weeks.entries()].map(([monday, ws], i) => {
    const km = ws.reduce((s, w) => s + (w.distanceM ?? 0), 0) / 1000;
    // The week you are living in gets the expanded treatment: its sessions
    // stack as full-width rows (the calendar agenda's shape) instead of
    // crowding as chips in a table cell, and the row takes an accent rim so
    // "this week" is findable without scanning dates. `monday` is this
    // week's Monday key, so today's key falling in [monday, monday+6] is
    // the whole test — string compare works because the keys are ISO.
    const sunday = new Date(new Date(`${monday}T12:00:00`).getTime() + 6 * 86_400_000);
    const isNow = todayKey >= monday && todayKey <= ymdLocal(sunday);
    const sorted = [...ws].sort((a, b) => a.date.localeCompare(b.date));
    const cells = sorted
      .map(
        (w) =>
          `<button type="button" class="chip workout-chip chip-${w.kind}${w.done ? " chip-done" : ""}" data-workout-id="${escapeHtml(w.id)}" data-workout-date="${w.date}" aria-label="${escapeHtml(`${w.date}: ${w.title}${w.done ? ", completed" : ""}`)}">${KIND_LABEL[w.kind] ?? w.kind}${
            w.distanceM ? ` ${(w.distanceM / 1000).toFixed(0)}km` : ""
          }${w.done ? riskDeltaBadge(w.id) : ""}</button>`,
      )
      .join(" ");
    // Expanded: one session per line in the calendar agenda's shape — kind
    // on the left, title, distance on the right. The weekday rides with the
    // distance, since stacking drops the chip order that was the dates'
    // only cue.
    const stacked = sorted
      .map(
        (w) =>
          `<button type="button" class="week-session agenda-row chip-${w.kind}${w.done ? " chip-done" : ""}" data-workout-id="${escapeHtml(w.id)}" data-workout-date="${w.date}" aria-label="${escapeHtml(`${w.date}: ${w.title}${w.done ? ", completed" : ""}`)}">
            <span class="agenda-kind">${KIND_LABEL[w.kind] ?? w.kind}</span>
            <span class="agenda-title">${escapeHtml(w.title)}</span>
            <span class="agenda-meta">${new Date(w.date + "T12:00:00").toLocaleDateString(undefined, { weekday: "short" })}${w.distanceM ? ` · ${(w.distanceM / 1000).toFixed(1)} km` : ""}${w.done ? ` ${riskDeltaBadge(w.id)}` : ""}</span>
          </button>`,
      )
      .join("");
    return `<tr${isNow ? ' class="plan-week-now"' : ""}><td>W${i + 1}<span class="muted"> ${monday}</span></td><td>${km.toFixed(1)} km</td><td>${isNow ? `<div class="week-sessions">${stacked}</div>` : cells}</td></tr>`;
  });
  el("plan-weeks").innerHTML = rows.length
    ? `<table id="weeks-table"><thead><tr><th>Week</th><th>Volume</th><th>Sessions</th></tr></thead><tbody>${rows.join("")}</tbody></table>`
    : `<p class="muted">No plan yet.</p>`;

  const pushBtn = el<HTMLButtonElement>("push-plan");
  const abortBtn = el<HTMLButtonElement>("abort-plan");
  // A refresh can land while a push/abort is still running (a background
  // sync completing, say). Never clobber a button mid-action — withBusy owns
  // its label and disabled state until the call settles.
  if (!pushBtn.classList.contains("btn-busy")) {
    pushBtn.disabled = plan.status === "pushed" || plan.status === "aborted";
  }
  if (!abortBtn.classList.contains("btn-busy")) {
    abortBtn.disabled = plan.status === "aborted";
    // A fresh render disarms the two-step confirm.
    abortBtn.textContent = "Abort plan";
  }
}

/**
 * Only one plan may exist at a time: while one is active the create form is
 * locked and the user must abort the current plan first.
 */
function setCreateFormEnabled(enabled: boolean): void {
  const form = el<HTMLFormElement>("create-plan");
  form.hidden = !enabled;
  el("plan-active-notice").hidden = enabled;
  // Linear journey: while a plan is active it is the page's subject, so it
  // moves above the (locked) create form instead of the user scrolling past
  // it. With no plan the form stays first — there is nothing to lead with.
  document
    .getElementById("app")!
    .classList.toggle("plan-active", !enabled);
}

async function refresh(): Promise<void> {
  const current = await tauri.currentPlan();
  // Done sessions show their run's injury-risk delta; a failed fetch (or no
  // desktop app) just means no badges rather than a broken plan page.
  try {
    const scores = await tauri.runScores();
    riskDeltaByWorkout = new Map(
      scores
        .filter((s) => s.workoutId != null)
        .map((s) => [s.workoutId as string, s.riskDelta]),
    );
  } catch {
    riskDeltaByWorkout = new Map();
  }
  if (current) {
    renderPlan(current[0], current[1]);
    setCreateFormEnabled(false);
    void refreshPacing(current[0], current[1]).catch(fail);
  } else {
    el("plan-status").innerHTML = `<p class="muted">No active plan — create one above.</p>`;
    el("plan-weeks").innerHTML = "";
    el<HTMLButtonElement>("push-plan").disabled = true;
    el<HTMLButtonElement>("abort-plan").disabled = true;
    setCreateFormEnabled(true);
    el("pacing-section").hidden = true;
  }
}

// ---- pacing: did the reps land in their range? -----------------------------

/// The plan currently rendered, kept so the range bars can apply its quality
/// offset to future sessions exactly the way the backend does.
let currentPlan: TrainingPlan | null = null;

const EXECUTION_LABEL: Record<Execution, string> = {
  ahead: "too fast",
  in: "in range",
  behind: "too slow",
};

/// A plan date (`YYYY-MM-DD`) as a short label. Parsed at local noon: `new
/// Date("2026-10-20")` is UTC midnight, which is still the 19th west of
/// Greenwich, so a session would be labelled the day before it was run.
function fmtDay(day: string): string {
  return fmtDate(`${day}T12:00:00`);
}

/// Fetch and paint the pacing panel. Called from `refresh` so a sync or a
/// hand-matched run re-grades the sessions without a page reload.
async function refreshPacing(plan: TrainingPlan, workouts: PlanWorkout[]): Promise<void> {
  currentPlan = plan;
  const section = el("pacing-section");
  const [report, offer] = await Promise.all([tauri.pacingReport(), tauri.pendingPacingProposal()]);
  const graded = report?.reviews ?? [];
  if (graded.length === 0) {
    // An adjustment was accepted since the last session was run, which retires
    // the sessions it was based on — they were read under the old ranges. The
    // panel stays, because "it is applied and here is when it shows" is worth
    // more than a panel that disappears the moment the runner acts on it.
    if (plan.qualityOffsetFrom != null && plan.qualityOffsetSecKm !== 0) {
      section.hidden = false;
      renderPacingApplied(plan);
      renderPacingProposal(null);
      return;
    }
    section.hidden = true;
    return;
  }
  section.hidden = false;
  renderPacing(report!, workouts);
  renderPacingProposal(offer);
}

/// The panel between accepting an adjustment and running the next quality
/// session: nothing to grade yet, so say what changed and when it is tested.
function renderPacingApplied(plan: TrainingPlan): void {
  const sharpen = plan.qualityOffsetSecKm < 0;
  el("pacing-body").innerHTML = `
    <p class="pacing-status">Adjustment applied — quality paces run ${
      sharpen ? "sharper" : "easier"
    } by ${Math.abs(Math.round(plan.qualityOffsetSecKm))} s/km from ${escapeHtml(
      fmtDay(plan.qualityOffsetFrom as string),
    )}. The next tempo or interval run will be graded against the new ranges.</p>
    <p class="pacing-legend muted">
      Sessions run before that date keep the range they were run against, so they no
      longer count toward an adjustment.
    </p>`;
}

function renderPacing(report: PacingReport, workouts: PlanWorkout[]): void {
  const byId = new Map(workouts.map((w) => [w.id, w]));
  const rows = report.reviews
    .slice()
    .reverse()
    .map((r) => {
      const workout = byId.get(r.workoutId);
      // The graded band came from the plan at review time; redraw it from the
      // workout so the bar and the plan's own ranges cannot drift apart.
      const band = (workout && qualityBand(currentPlan!, workout)) ?? [r.bandLoSKm, r.bandHiSKm];
      return pacingRow(r, band);
    })
    .join("");

  const streakLine = report.triggered
    ? `<p class="pacing-status tone-bad">${report.streak} quality sessions in a row ${
        report.direction === "ahead" ? "faster than" : "slower than"
      } their ranges — an adjustment is proposed above.</p>`
    : report.direction != null && report.streak > 0
      ? `<p class="pacing-status">${report.streak} of ${report.needed} sessions ${
          report.direction === "ahead" ? "faster" : "slower"
        } than their ranges so far. One more the same way and a plan adjustment will be proposed.</p>`
      : `<p class="pacing-status">Nothing to adjust — recent sessions are landing where they were asked to.</p>`;

  el("pacing-body").innerHTML = `
    ${streakLine}
    <div class="pacing-rows">${rows}</div>
    <p class="pacing-legend muted">
      The band is the range the session prescribed; the marker is the pace held over the
      reps themselves, not the session average — jog recoveries never count against it.
    </p>`;
}

/// One session: its prescribed band drawn to a shared pace axis, with a marker
/// where the reps actually landed. Pace runs left = faster, so the row reads
/// "too fast · in range · too slow" in the order you would say it.
function pacingRow(r: SessionReview, band: [number, number]): string {
  const lo = Math.min(band[0], r.actualSKm ?? band[0]) - 15;
  const hi = Math.max(band[1], r.actualSKm ?? band[1]) + 15;
  const pos = (p: number) => ((p - lo) / (hi - lo)) * 100; // slower pace → right
  const bandLeft = pos(band[0]).toFixed(1);
  const bandWidth = (pos(band[1]) - pos(band[0])).toFixed(1);
  const marker =
    r.actualSKm != null
      ? `<span class="pacing-marker" style="left:${pos(r.actualSKm).toFixed(1)}%" title="held ${fmtPace(r.actualSKm)}/km"></span>`
      : "";
  const held = r.actualSKm != null ? `held ${fmtPace(r.actualSKm)}/km` : "no track synced";
  return `<div class="pacing-row exec-${r.execution}">
    <span class="pacing-day">${escapeHtml(fmtDay(r.date))}</span>
    <span class="pacing-what">
      <span class="pacing-title">${escapeHtml(r.title)}</span>
      <span class="pacing-paces">${fmtPace(band[0])}–${fmtPace(band[1])} · ${held}</span>
    </span>
    <span class="pacing-track" role="img" aria-label="Range ${fmtPace(band[0])} to ${fmtPace(band[1])} per km, reps held ${r.actualSKm != null ? fmtPace(r.actualSKm) : "unknown"} per km — ${EXECUTION_LABEL[r.execution]}">
      <span class="pacing-band" style="left:${bandLeft}%;width:${bandWidth}%"></span>
      ${marker}
    </span>
    <span class="pacing-verdict">${EXECUTION_LABEL[r.execution]}</span>
  </div>`;
}

let pacingOffer: PlanProposal | null = null;

function renderPacingProposal(offer: PlanProposal | null): void {
  pacingOffer = offer;
  const box = el("pacing-proposal");
  if (!offer) {
    box.hidden = true;
    box.innerHTML = "";
    return;
  }
  box.hidden = false;
  box.innerHTML = `
    <p class="proposal-summary">${escapeHtml(offer.summary)}</p>
    <p class="muted">${escapeHtml(offer.reason)}</p>
    <div class="actions">
      <button id="pacing-accept" class="primary">Accept adjustment</button>
      <button id="pacing-dismiss" class="secondary">Not now</button>
    </div>`;
  el<HTMLButtonElement>("pacing-accept").addEventListener("click", async (ev) => {
    const btn = ev.currentTarget as HTMLButtonElement;
    try {
      await withBusy(btn, "Applying…", () => tauri.applyProposal("coros", pacingOffer!.id));
      pacingOffer = null;
      box.hidden = true;
      box.innerHTML = "";
      await refresh();
    } catch (err) {
      fail(err);
    }
  });
  el<HTMLButtonElement>("pacing-dismiss").addEventListener("click", async (ev) => {
    try {
      await withBusy(ev.currentTarget as HTMLButtonElement, "Dismissing…", () => tauri.dismissProposal(pacingOffer!.id, null));
      pacingOffer = null;
      box.hidden = true;
      box.innerHTML = "";
    } catch (err) {
      fail(err);
    }
  });
}

/// Feed a plan push/abort's `plan://sync` stages into a busy button's label.
/// The DOM (spinner, disable, restore) is owned by the shared `withBusy`; this
/// only adds the operation-specific part — subscribing to the backend's
/// progress events for the duration of the action and releasing the listener
/// when it settles.
async function withPlanSyncFeed<T>(
  report: (stage: string, percent?: number | null) => void,
  action: () => Promise<T>,
): Promise<T> {
  const unlisten = inTauri
    ? await listen<PlanSyncProgress>("plan://sync", (event) => {
        report(event.payload.stage, event.payload.percent);
      }).catch(() => null)
    : null;
  try {
    return await action();
  } finally {
    if (unlisten) unlisten();
  }
}

async function main(): Promise<void> {
  initSyncIndicator();
  // Same motion layer as the dashboard: the page's scroll rail (this page has
  // no group headings, so the listener only drives the rail) and the panels'
  // entrance held until they are scrolled to — the weeks table and the pacing
  // panel live well below the form.
  initGroupProgress();
  initScrollReveal();
  // The kind decides which fields exist at all, so it is applied before
  // anything reads the form — and re-applied on every change.
  applyKind();
  document.querySelectorAll<HTMLInputElement>('input[name="plan-kind"]').forEach((r) => {
    r.addEventListener("change", () => {
      applyKind();
      updatePlanPreview();
    });
  });
  el("plan-weeks").addEventListener("click", (event) => {
    const button = (event.target as Element).closest<HTMLButtonElement>("[data-workout-id]");
    if (button) void openWorkout(button.dataset.workoutId!, button.dataset.workoutDate!).catch(fail);
  });
  for (const event of ["runalytics:sync-completed", "runalytics:calendar-updated"]) {
    window.addEventListener(event, () => { void refresh().catch(fail); });
  }
  try {
    await refresh();
  } catch (e) {
    if (!inTauri) {
      el("plan-status").innerHTML =
        `<p class="muted">Runalytics needs the desktop app for data — run \`npm run tauri dev\` and open Plan there.</p>`;
    } else {
      fail(e);
    }
  }

  // Prefill threshold and current mileage with whatever the watch reported.
  try {
    defaults = await tauri.planDefaults();
  } catch {
    /* no device data yet — leave the form empty */
  }
  // A plan built on top of an unresolved injury tends to make it worse —
  // say so before the runner commits, without blocking them.
  if (defaults.activePain) {
    const p = defaults.activePain;
    const region = p.bodyPart.split(".").reverse().join(" ").replace(/_/g, " ");
    const warn = el("plan-pain-warning");
    warn.hidden = false;
    warn.innerHTML =
      `<p>You logged ${region} pain at ${p.intensity}/10 since ${fmtDate(p.startedOn)} and it is not resolved. ` +
      `Consider healing it first — <a href="injury.html">review the injury</a>.</p>`;
  }
  const thresholdInput = el<HTMLInputElement>("plan-threshold");
  if (defaults.thresholdSecKm != null && !thresholdInput.value) {
    thresholdInput.value = fmtPace(defaults.thresholdSecKm);
    thresholdFromDevice = true;
    thresholdInput.addEventListener(
      "input",
      () => {
        thresholdFromDevice = false;
      },
      { once: true },
    );
  }
  const mileageInput = el<HTMLInputElement>("plan-mileage");
  const mileageHint = el("plan-mileage-hint");
  if (defaults.currentWeeklyKm != null) {
    mileageInput.value = defaults.currentWeeklyKm.toFixed(1);
    const n = defaults.weeksObserved;
    const parts = [`Average of your last ${n} complete week${n === 1 ? "" : "s"}`];
    if (defaults.thisWeekKm > 0) {
      parts.push(`${defaults.thisWeekKm.toFixed(1)} km so far this week`);
    }
    if (defaults.lastSync) {
      parts.push(`synced ${fmtDate(defaults.lastSync)}`);
    }
    mileageHint.textContent = `${parts.join(" · ")}.`;
  } else if (defaults.totalKm > 0) {
    // Runs synced but no finished week yet (a fresh install mid-week).
    mileageHint.textContent =
      `No complete week synced yet — ${defaults.totalKm.toFixed(0)} km in the history. ` +
      `Leave empty and the plan will use your first full week.`;
  } else {
    mileageHint.textContent = "Sync your watch to fill this in, or type your weekly average.";
  }

  // The plan's end day can never be in the past; two weeks is the engine's
  // minimum. Maintenance skips this field entirely — its end date comes from
  // the block length, which cannot produce a past date.
  const dateInput = el<HTMLInputElement>("plan-target-date");
  const minDate = new Date();
  minDate.setDate(minDate.getDate() + 15);
  dateInput.min = ymdLocal(minDate);

  // Frequency stepper: press feedback is instant, bounds disable the arrows.
  const freqInput = el<HTMLInputElement>("plan-frequency");
  const syncStepper = () => {
    const v = parseInt(freqInput.value, 10) || 3;
    document.querySelectorAll<HTMLButtonElement>("#create-plan .plan-step").forEach((b) => {
      b.disabled = v <= 3 ? b.dataset.dir === "-1" : v >= 7 ? b.dataset.dir === "1" : false;
    });
  };
  document.querySelectorAll<HTMLButtonElement>("#create-plan .plan-step").forEach((b) => {
    b.addEventListener("click", () => {
      const v = Math.min(7, Math.max(3, (parseInt(freqInput.value, 10) || 4) + Number(b.dataset.dir)));
      freqInput.value = String(v);
      syncStepper();
      updatePlanPreview();
    });
  });
  syncStepper();

  const form = el<HTMLFormElement>("create-plan");
  if (!inTauri) form.querySelector<HTMLButtonElement>('button[type="submit"]')!.disabled = true;
  form.addEventListener("input", updatePlanPreview);
  form.addEventListener("change", updatePlanPreview);
  updatePlanPreview();

  form.addEventListener("submit", async (ev) => {
    ev.preventDefault();
    const btn = form.querySelector<HTMLButtonElement>('button[type="submit"]')!;
    if (btn.disabled) return;
    setPlanError(null);
    const thresholdInput = el<HTMLInputElement>("plan-threshold");
    const threshold = parsePace(thresholdInput.value);
    if (threshold == null) {
      setFieldError(thresholdInput, "bad");
      setPlanError("Threshold pace must look like 4:45.");
      thresholdInput.focus();
      return;
    }
    const kind = selectedKind();
    const dateStr = effectiveTargetDate();
    const days = dateStr ? daysUntilRace(dateStr) : 0;
    if (!dateStr || days < 15) {
      setFieldError(dateInput, "bad");
      setPlanError("Pick an end date at least two weeks out.");
      if (kind !== "maintenance") dateInput.focus();
      return;
    }
    try {
      const mileageRaw = parseFloat(el<HTMLInputElement>("plan-mileage").value);
      const [plan, workouts] = await withBusy(btn, "Generating plan…", () => tauri.createPlan({
        distance: selectedDistance(),
        kind,
        targetDate: dateStr,
        thresholdSecKm: threshold,
        weeklyFrequency: parseInt(freqInput.value, 10),
        longrunDay: parseInt(el<HTMLSelectElement>("plan-longrun-day").value, 10),
        increase: el<HTMLSelectElement>("plan-increase").value as IncreaseMethod,
        currentWeeklyKm: isFinite(mileageRaw) && mileageRaw > 0 ? mileageRaw : null,
      }));
      renderPlan(plan, workouts);
      // This may be the first plan the account has ever had — that is exactly
      // the moment the rest of the nav becomes real.
      void initNavGate(true).catch(() => undefined);
      scrollIntoViewMotion(el("plan-view"), { block: "start" });
    } catch (e) {
      fail(e);
    }
  });

  // Push and abort both round-trip to COROS, which takes tens of seconds for
  // a whole plan. Without feedback the button looked dead, so each runs
  // through `withBusy`: the button is disabled and reports the backend's
  // `plan://sync` stages (with a percentage) until the call settles.
  const pushBtn = el<HTMLButtonElement>("push-plan");
  pushBtn.addEventListener("click", async () => {
    try {
      await withBusy(pushBtn, "Pushing to COROS", (report) =>
        withPlanSyncFeed(report, () => tauri.pushPlan("coros")),
      );
      await refresh();
    } catch (e) {
      fail(e);
    }
  });

  // Two-step confirm on the button itself: window.confirm() shows no dialog
  // in Tauri's WKWebView and returns falsy, which made Abort a no-op.
  const abortBtn = el<HTMLButtonElement>("abort-plan");
  let abortArmed = false;
  let abortArmTimer = 0;
  const setAbortLabel = () => {
    abortBtn.textContent = abortArmed ? "Really abort?" : "Abort plan";
  };
  abortBtn.addEventListener("click", async () => {
    if (!abortArmed) {
      abortArmed = true;
      setAbortLabel();
      clearTimeout(abortArmTimer);
      abortArmTimer = window.setTimeout(() => {
        abortArmed = false;
        setAbortLabel();
      }, 4000);
      return;
    }
    abortArmed = false;
    clearTimeout(abortArmTimer);
    // Reset the label before withBusy captures it, or it would restore
    // "Really abort?" once the call finishes.
    setAbortLabel();
    try {
      await withBusy(abortBtn, "Aborting plan", (report) =>
        withPlanSyncFeed(report, () => tauri.abortPlan()),
      );
      // The plan is gone either way — refresh so the page drops it and the
      // create form unlocks. Without this the abort looked like a no-op.
      await refresh();
    } catch (e) {
      // The plan is deleted locally even when the COROS cancel failed — the
      // error says the watch may still show it, so refresh before showing.
      await refresh();
      fail(e);
    }
  });
}

// The Plan page is the single place a plan can be created, so unlike the other
// gated tabs it does not redirect when there is no plan — it stays open and the
// gate reveals it as the current page with a way back to Setup.
async function boot(): Promise<void> {
  await requirePlanOrRedirect("plan");
  await main();
}

void boot();
