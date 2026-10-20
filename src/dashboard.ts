import maplibregl from "maplibre-gl";
import "maplibre-gl/dist/maplibre-gl.css";
import { Protocol } from "pmtiles";
import { convertFileSrc } from "@tauri-apps/api/core";
import {
  tauri,
  el,
  fail,
  fmtDate,
  fmtDuration,
  fmtKm,
  fmtPace,
  inTauri,
} from "./lib/ipc";
import { fetchFlowState, nextAction, currentStreak } from "./lib/flow";
import type { FlowState } from "./lib/flow";
import { icons } from "./lib/icons";
import { qualityBand } from "./lib/plan-bands";
import { initCarousels } from "./lib/carousel";
import { onboardingPending, startOnboarding } from "./lib/onboarding";
import { initSegmented } from "./lib/segmented";
import { initSyncIndicator } from "./lib/sync";
import { openWorkout } from "./lib/workout-detail";
import { createSheet } from "./lib/overlay";
import type { Sheet } from "./lib/overlay";
import { crossPhrase, crossTraining, isRun, runKm } from "./lib/sport";
import type { CrossTraining } from "./lib/sport";
// Side-effect import: paints the saved colour scheme before first paint.
import "./lib/theme";
import "./lib/navigation";
import { initNavGate } from "./lib/navgate";
import type {
  Activity,
  ActivityDetail,
  DashboardPayload,
  HealthDaily,
  HeatPoint,
  LoadPoint,
  LoadRecommendation,
  PlanWorkout,
  RiskFactor,
  Settings,
  Thresholds,
  TimeRange,
  TrainingPlan,
} from "./lib/types";

// The dashboard no longer has a range switch: every window is fixed and
// labelled where it renders ("last 30 days", "this week"), so a number can
// never silently change meaning between visits.
const currentRange: TimeRange = "1mo";

/// Side data the dashboard composes from the existing commands — the week
/// strip, streak and race countdown all come from here.
///
/// `allActivities` is genuinely every sport the watch synced. Anything that
/// reports *mileage* must filter it through `isRun`/`runKm` first (see
/// lib/sport.ts); anything that reports *activity* — the streak — may not,
/// because a bike day is still a day you trained.
interface DashExtras {
  allActivities: Activity[];
  weekPlan: PlanWorkout[];
  plan: TrainingPlan | null;
}

// ---- today hero ------------------------------------------------------------
// The dashboard opens with the two answers a runner actually wants: how
// ready am I, and what should I do today. The readiness number keeps its
// three inputs as labelled chips (never a fourth number to mentally
// combine), and the right side carries the plan: today's session, the
// streak, and the race countdown.

function readinessVerdict(v: number): string {
  return v >= 70
    ? "Primed — good day to push."
    : v >= 45
      ? "Steady — a solid training day."
      : "Low — go easy or take the rest.";
}

function scoreTone(v: number, invert = false): "good" | "warn" | "bad" {
  const t = invert ? 100 - v : v;
  return t >= 70 ? "good" : t >= 45 ? "warn" : "bad";
}

const KIND_LABEL: Record<string, string> = {
  easy: "Easy run",
  tempo: "Tempo",
  intervals: "Intervals",
  longRun: "Long run",
  race: "Race day",
  rest: "Rest day",
};

/// Local-time YYYY-MM-DD — plan workouts are keyed by local day.
function ymd(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

/// Session titles come from the backend (and, after a proposal is applied,
/// from text that names a previous title), so anything interpolated into
/// innerHTML goes through here.
function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

/// Whole days from today until an ISO date, negative once it has passed.
function daysUntil(iso: string): number {
  const target = new Date(iso);
  const today = new Date();
  const ms =
    Date.UTC(target.getFullYear(), target.getMonth(), target.getDate()) -
    Date.UTC(today.getFullYear(), today.getMonth(), today.getDate());
  return Math.round(ms / 86_400_000);
}

/// Cross-training with the days it took up, for sentences. The figures come
/// from lib/sport.ts, where each sport carries only its own numbers.
function crossDays(c: CrossTraining): string {
  return `${crossPhrase(c)} over ${c.days} day${c.days === 1 ? "" : "s"}`;
}

/// The compact form for a chart caption: the biggest sport, in its own unit.
/// Naming every sport would not fit under a 7-column grid; the full breakdown
/// lives in the tooltip and the week tile.
function crossShort(c: CrossTraining): string {
  const s = c.bySport[0];
  return s.hasDistance && s.km >= 1 ? s.km.toFixed(0) : `${s.hours.toFixed(1)} h`;
}

/// Coach's brief: plain language answering "how am I, how did last week go,
/// and how hard can I push this week". Every clause comes from a decision the
/// backend already made (readiness, recovery band, status tone, build cycle) —
/// the prose only connects them, so it can never disagree with the panels.
function coachBrief(data: DashboardPayload, extras: DashExtras): string {
  if (!extras.allActivities.length) {
    return "Nothing has synced yet — connect your watch and this page fills in with how you're doing and how to train.";
  }
  const { scores } = data;
  const st = data.trainingStatus;
  const rec = data.loadRecommendation;
  const band = data.recoveryBand;

  // How you are today. The readiness verdict is "Primed — good day to push."
  // style; the brief only wants the leading word.
  const risk = scores.injuryRisk;
  const riskNote =
    risk <= 30
      ? "injury risk is low"
      : risk <= 55
        ? `injury risk is moderate (${risk}%)`
        : `injury risk is high (${risk}%)`;
  const mood = readinessVerdict(scores.globalScore).split(" ")[0].toLowerCase();
  const body = `You're ${mood}; recovery ${scores.recoveryScore}/100${band ? ` (${band.label.toLowerCase()})` : ""}, ${riskNote}.`;

  // How last week went, against the same target the mileage chart draws.
  const monday = mondayThisWeek();
  const lastFrom = new Date(monday);
  lastFrom.setDate(lastFrom.getDate() - 7);
  const lastTo = new Date(monday);
  lastTo.setDate(lastTo.getDate() - 1);
  const [fromKey, toKey] = [ymd(lastFrom), ymd(lastTo)];
  const lastWeek = extras.allActivities.filter((a) => {
    const k = a.startTime.slice(0, 10);
    return k >= fromKey && k <= toKey;
  });
  const lastKm = runKm(lastWeek);
  const lastDays = new Set(
    lastWeek.filter(isRun).map((a) => a.startTime.slice(0, 10)),
  ).size;
  // Cross-training gets named rather than folded into the mileage. A 90 km
  // spin is real training the plan cannot bank, so the brief credits it in its
  // own unit instead of either hiding it or inflating the run total.
  const lastCross = crossTraining(lastWeek);
  const last =
    lastKm > 0
      ? `Last week: ${lastKm.toFixed(1)} km over ${lastDays} run day${lastDays === 1 ? "" : "s"}${
          rec
            ? `, ${lastKm >= rec.target * 0.95 ? "on target" : `${(rec.target - lastKm).toFixed(1)} km below target`}`
            : ""
        }${lastCross ? `; ${crossDays(lastCross)}` : ""}.`
      : `Last week: no runs${lastCross ? `; ${crossDays(lastCross)}` : ""}.`;

  // How this week can go: the plan's target, what's banked, and one piece of
  // advice — the tone check order matters, the down week and high risk
  // override any "push" reading the load status might give.
  const banked = runKm(
    extras.allActivities.filter((a) => a.startTime.slice(0, 10) >= ymd(monday)),
  );
  let week: string;
  if (rec?.downWeek) {
    week = `Down week ${rec.weekInCycle}/${rec.cycleLength}: ${rec.target.toFixed(0)} km target, ${banked.toFixed(1)} km banked.`;
  } else if (risk > 55) {
    week = `Keep this week easy; skip extra speed and long-run pushes.`;
  } else if (st?.tone === "bad") {
    week = `Keep the easy days easy${rec ? `; ${rec.target.toFixed(0)} km target, ${banked.toFixed(1)} km banked` : ""}.`;
  } else if (st?.tone === "warn" || scores.recoveryScore < 50) {
    week = `Stick to the plan${rec ? `; ${rec.target.toFixed(0)} km target, ${banked.toFixed(1)} km banked` : ""}.`;
  } else {
    week = `Room to push: a hard session or two fits this week${rec ? `; ${rec.target.toFixed(0)} km target, ${banked.toFixed(1)} km banked` : ""}.`;
  }
  return `${body} ${last} ${week}`;
}

function openPlannedWorkout(event: MouseEvent): void {
  const button = (event.target as Element).closest<HTMLButtonElement>("[data-workout-id]");
  if (button) void openWorkout(button.dataset.workoutId!, button.dataset.workoutDate!).catch(fail);
}

function renderTodayHero(data: DashboardPayload, extras: DashExtras): void {
  el("today-hero").onclick = openPlannedWorkout;
  const { scores } = data;
  const tone = scoreTone(scores.globalScore);
  const todayKey = ymd(new Date());
  const todays = extras.weekPlan.filter((w) => w.date === todayKey);
  const session = todays.length
    ? todays
        .map((w) => {
          const label = KIND_LABEL[w.kind] ?? w.kind;
          const dist = w.distanceM ? ` · ${(w.distanceM / 1000).toFixed(1)} km` : "";
          const dur = w.durationMin ? ` · ${Math.round(w.durationMin)} min` : "";
          return `<button type="button" class="workout-link" data-workout-id="${escapeHtml(w.id)}" data-workout-date="${w.date}">${w.done ? "Done: " : ""}${label}${dist}${dur}</button>`;
        })
        .join(" + ")
    : null;
  // The streak counts days trained, not days run — a bike day keeps the
  // streak alive, which is why the chip says "active days" and not "days run".
  const streak = currentStreak(extras.allActivities);
  const plan = extras.plan;
  const raceIn = plan ? daysUntil(plan.targetDate) : null;
  const lastSync = data.health.length
    ? data.health
        .map((h) => h.date)
        .sort()
        [data.health.length - 1]
    : null;
  el("today-hero").innerHTML = `
    <div class="today-hero">
      <div class="readiness ${tone}">
        <div class="readiness-score">${scores.globalScore}</div>
        <div class="readiness-text">
          <div class="eyebrow">Readiness</div>
          <div class="readiness-verdict">${readinessVerdict(scores.globalScore)}</div>
          <div class="hero-chips">
            <span class="hero-chip tone-${scoreTone(scores.trainingScore)}"><i></i>Training ${scores.trainingScore}</span>
            <span class="hero-chip tone-${scoreTone(scores.recoveryScore)}"><i></i>Recovery ${scores.recoveryScore}${data.recoveryBand ? ` · ${data.recoveryBand.label}` : ""}</span>
            <span class="hero-chip tone-${scoreTone(scores.injuryRisk, true)}"><i></i>Injury risk ${scores.injuryRisk}</span>
          </div>
        </div>
      </div>
      <div class="today-action">
        <div class="eyebrow">Today</div>
        <div class="today-session">${session ?? "No session planned"}</div>
        <div class="hero-chips">
          <span class="hero-chip" title="Consecutive active days">${icons.flame} ${streak}-day streak</span>
          ${
            plan && raceIn != null && raceIn >= 0
              ? `<span class="hero-chip">${icons.flag} ${plan.distance} in ${raceIn} day${raceIn === 1 ? "" : "s"}</span>`
              : ""
          }
          ${lastSync ? `<span class="hero-chip">${icons.sync} synced ${fmtDate(lastSync)}</span>` : ""}
        </div>
      </div>
      <p class="hero-brief">${coachBrief(data, extras)}</p>
    </div>`;
}

// Apple Weather-style mini chart: a smooth curve (Catmull-Rom → cubic
// Bézier), a gradient area wash that fades to transparent, faint dashed
// gridlines, and a draw-in animation. Hovering slides a marker dot along
// the curve and reveals the value at that sample — like scrubbing the
// hourly temperature graph.

interface ChartSpec {
  values: number[];
  labels: string[];
  min: number;
  span: number;
  format: (v: number) => string;
}

const CHART_W = 100;
const CHART_H = 44;
const CHART_PAD = 7;
const chartSpecs = new Map<string, ChartSpec>();
let chartUid = 0;

/// Catmull-Rom control points converted to cubic Bézier segments.
function smoothPath(pts: [number, number][]): string {
  let d = `M${pts[0][0].toFixed(2)},${pts[0][1].toFixed(2)}`;
  for (let i = 0; i < pts.length - 1; i++) {
    const p0 = pts[Math.max(0, i - 1)];
    const p1 = pts[i];
    const p2 = pts[i + 1];
    const p3 = pts[Math.min(pts.length - 1, i + 2)];
    const c1x = p1[0] + (p2[0] - p0[0]) / 6;
    const c1y = p1[1] + (p2[1] - p0[1]) / 6;
    const c2x = p2[0] - (p3[0] - p1[0]) / 6;
    const c2y = p2[1] - (p3[1] - p1[1]) / 6;
    d += ` C${c1x.toFixed(2)},${c1y.toFixed(2)} ${c2x.toFixed(2)},${c2y.toFixed(2)} ${p2[0].toFixed(2)},${p2[1].toFixed(2)}`;
  }
  return d;
}

function chartY(v: number, min: number, span: number): number {
  return CHART_H - CHART_PAD - ((v - min) / span) * (CHART_H - 2 * CHART_PAD);
}

function areaChart(values: (number | null)[], format: (v: number) => string, dates: string[]): string {
  const samples = values.map((value, index) => ({ value, date: dates[index] }))
    .filter((sample): sample is { value: number; date: string } => sample.value != null);
  const vs = samples.map((sample) => sample.value);
  if (vs.length < 2) return "";
  const min = Math.min(...vs);
  const max = Math.max(...vs);
  const span = max - min || 1;
  const pts = vs.map(
    (v, i) => [(i / (vs.length - 1)) * CHART_W, chartY(v, min, span)] as [number, number],
  );
  const line = smoothPath(pts);
  const area = `${line} L${CHART_W},${CHART_H} L0,${CHART_H} Z`;
  const uid = `spark-${chartUid++}`;
  // Draw-in = a clipPath wipe, not a stroke dash. Dash-based draw-ins are
  // unfixable here: stroke-dasharray/offset and getTotalLength() resolve in
  // *opposite* coordinate spaces between engines when
  // `vector-effect="non-scaling-stroke"` sits inside a stretched viewBox
  // (Chrome: both screen-space; WebKit: dash against the unscaled viewBox,
  // length in screen px) — whichever value is measured, one engine's dash
  // window overshoots the path and the line stays invisible until the
  // animation's final stretch. A CSS transform on a userSpaceOnUse clip rect
  // is user-space in every engine, so the wipe is deterministic; if the
  // animation never runs, the untouched clip shows the chart fully drawn.
  // Stash the spec so the hover handler can map pointer x → sample value.
  chartSpecs.set(uid, { values: vs, labels: samples.map((sample) => fmtDate(sample.date)), min, span, format });
  return `<svg class="spark" viewBox="0 0 ${CHART_W} ${CHART_H}" preserveAspectRatio="none" aria-hidden="true">
    <defs>
      <linearGradient id="${uid}" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="currentColor" stop-opacity="0.34"/>
        <stop offset="0.7" stop-color="currentColor" stop-opacity="0.06"/>
        <stop offset="1" stop-color="currentColor" stop-opacity="0"/>
      </linearGradient>
      <clipPath id="${uid}-clip" clipPathUnits="userSpaceOnUse">
        <rect class="spark-wipe" style="--spark-w:${CHART_W + 4}" x="-2" y="-6" width="${CHART_W + 4}" height="${CHART_H + 12}"/>
      </clipPath>
    </defs>
    <line class="spark-grid" x1="0" x2="${CHART_W}" y1="${CHART_H * 0.34}" y2="${CHART_H * 0.34}"/>
    <line class="spark-grid" x1="0" x2="${CHART_W}" y1="${CHART_H * 0.68}" y2="${CHART_H * 0.68}"/>
    <g clip-path="url(#${uid}-clip)">
      <path d="${area}" fill="url(#${uid})"/>
      <path class="spark-line" d="${line}" fill="none" stroke="currentColor"
        stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
        vector-effect="non-scaling-stroke"/>
    </g>
  </svg>`;
}

/// One signal card: label, big value, sparkline, hint. Shared by the
/// readiness sub-scores and the recovery signals so both sections render
/// with the same markup and the same hover-scrub behaviour.
function signalCard(
  label: string,
  value: string,
  unit: string,
  hint: string,
  series: (number | null)[],
  tone: string,
  format: (v: number) => string,
  dates: string[],
): string {
  const uid = `spark-${chartUid}`;
  const chart = areaChart(series, format, dates);
  return `<div class="card signal ${tone}">
    <div class="signal-head"><span class="card-label">${label}</span></div>
    <div class="card-value"><span class="live-value">${value}<span class="unit">${unit}</span></span></div>
    ${chart ? `<div class="spark-wrap" data-chart="${uid}">${chart}
      <span class="chart-dot" hidden></span>
    </div>` : `<div class="chart-empty">Not enough history</div>`}
    <div class="score-hint">${hint}</div>
  </div>`;
}

/// Wire the scrub-on-hover behaviour onto every chart in the container.
interface ChartSample {
  label: string;
  value: string;
  x: number;
  y?: number;
}

function bindChartInspection(plot: HTMLElement, samples: ChartSample[], label: string, initialIndex = samples.length - 1): void {
  if (!samples.length) return;
  const readout = document.createElement("div");
  readout.className = "chart-readout";
  const date = document.createElement("span");
  const value = document.createElement("strong");
  readout.append(date, value);
  plot.before(readout);
  const cursor = document.createElement("span");
  cursor.className = "chart-cursor";
  cursor.setAttribute("aria-hidden", "true");
  plot.append(cursor);
  const dot = plot.querySelector<HTMLElement>(".chart-dot");
  const columns = [...plot.querySelectorAll<HTMLElement>(".bar-col")];
  plot.tabIndex = 0;
  plot.setAttribute("role", "slider");
  plot.setAttribute("aria-label", label);
  plot.setAttribute("aria-orientation", "horizontal");
  plot.setAttribute("aria-valuemin", "0");
  plot.setAttribute("aria-valuemax", String(samples.length - 1));
  let selected = initialIndex;
  const select = (index: number, inspecting = true) => {
    selected = Math.max(0, Math.min(samples.length - 1, index));
    const sample = samples[selected];
    date.textContent = sample.label;
    value.textContent = sample.value;
    plot.setAttribute("aria-valuenow", String(selected));
    plot.setAttribute("aria-valuetext", `${sample.label}: ${sample.value}`);
    cursor.style.left = `${sample.x}%`;
    cursor.hidden = !inspecting;
    if (dot && sample.y != null) {
      dot.style.left = `${sample.x}%`;
      dot.style.top = `${sample.y}%`;
      dot.hidden = !inspecting;
    }
    columns.forEach((column, columnIndex) => column.classList.toggle("inspected", inspecting && columnIndex === selected));
  };
  const reset = () => select(initialIndex, false);
  const inspectPointer = (event: PointerEvent) => {
    const rect = plot.getBoundingClientRect();
    const position = Math.max(0, Math.min(100, (event.clientX - rect.left) / rect.width * 100));
    const nearest = samples.reduce((best, sample, index) =>
      Math.abs(sample.x - position) < Math.abs(samples[best].x - position) ? index : best, 0);
    select(nearest);
  };
  plot.addEventListener("pointermove", inspectPointer);
  plot.addEventListener("pointerdown", (event) => {
    plot.focus({ preventScroll: true });
    inspectPointer(event);
  });
  plot.addEventListener("pointerleave", () => {
    if (document.activeElement !== plot) reset();
  });
  plot.addEventListener("pointercancel", reset);
  plot.addEventListener("focus", () => select(selected));
  plot.addEventListener("blur", reset);
  plot.addEventListener("keydown", (event) => {
    if (!["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End", "Escape"].includes(event.key)) return;
    event.preventDefault();
    if (event.key === "Escape") reset();
    else select(event.key === "Home" ? 0 : event.key === "End" ? samples.length - 1 : selected + (["ArrowRight", "ArrowUp"].includes(event.key) ? 1 : -1));
  });
  reset();
}

function bindChartHover(container: HTMLElement): void {
  container.querySelectorAll<HTMLElement>(".spark-wrap").forEach((wrap) => {
    const spec = chartSpecs.get(wrap.dataset.chart ?? "");
    if (!spec) return;
    const label = wrap.closest(".card")?.querySelector(".card-label")?.textContent
      ?? wrap.closest("section")?.querySelector("h2")?.textContent ?? "History";
    bindChartInspection(wrap, spec.values.map((value, index) => ({
      label: spec.labels[index],
      value: spec.format(value),
      x: index / (spec.values.length - 1) * 100,
      y: chartY(value, spec.min, spec.span) / CHART_H * 100,
    })), label);
  });
}

function fmtSleepHours(min: number | null): string {
  if (min == null) return "—";
  return `${Math.floor(min / 60)}h ${String(Math.round(min % 60)).padStart(2, "0")}m`;
}

function avg(xs: (number | null)[]): number | null {
  const v = xs.filter((x): x is number => x != null);
  return v.length ? v.reduce((s, x) => s + x, 0) / v.length : null;
}

/// Tone class from thresholds: good when low-ish, escalating with `warn/bad`.
function toneLow(value: number, warn: number, bad: number): string {
  return value >= bad ? "tone-bad" : value >= warn ? "tone-warn" : "tone-good";
}
function toneHigh(value: number, warn: number, bad: number): string {
  return value <= bad ? "tone-bad" : value <= warn ? "tone-warn" : "tone-good";
}

function renderWearables(health: HealthDaily[]): void {
  if (health.length === 0) {
    el("wearable-cards").innerHTML = `<p class="muted">No wearable data yet — sync COROS first.</p>`;
    return;
  }
  // Oldest → newest for sparklines.
  const asc = [...health].reverse();
  const dates = asc.map((sample) => sample.date);
  // Four signals, four charts — the ones that actually change the training
  // decision. Deep/REM minutes ride along as the sleep card's hint instead
  // of earning their own sparkline, and steps are dropped entirely.
  const sleep = avg(health.map((h) => h.sleepScore));
  const sleepDur = avg(health.map((h) => h.sleepMin));
  const deep = avg(health.map((h) => h.deepMin));
  const rhr = avg(health.map((h) => h.restingHr));
  const hrv = avg(health.map((h) => h.hrvAvg));
  const stress = avg(health.map((h) => h.stressAvg));
  const by = (f: (h: HealthDaily) => number | null) => asc.map(f);
  // One fixed window, stated on the card: the old range toggle made the same
  // card read "7-day avg" or "30-day avg" depending on a control elsewhere.
  const q = "30-day avg";

  const r0 = (v: number) => `${Math.round(v)}`;
  el("wearable-cards").innerHTML = [
    sleep != null
      ? signalCard("Sleep score", `${Math.round(sleep)}`, "", `${fmtSleepHours(sleepDur)} · ${fmtSleepHours(deep ?? 0)} deep · ${q}`, by((h) => h.sleepScore), toneHigh(sleep, 65, 50), r0, dates)
      : "",
    rhr != null
      ? signalCard("Resting HR", `${Math.round(rhr)}`, "bpm", `${q}, lower is fitter`, by((h) => h.restingHr), toneLow(rhr, 60, 68), (v) => `${Math.round(v)} bpm`, dates)
      : "",
    hrv != null
      ? signalCard("HRV", `${Math.round(hrv)}`, "ms", `${q}, higher is fresher`, by((h) => h.hrvAvg), toneHigh(hrv, 45, 35), (v) => `${Math.round(v)} ms`, dates)
      : "",
    stress != null
      ? signalCard("Stress", `${Math.round(stress)}`, "/100", `${q}, recovery balance`, by((h) => h.stressAvg), toneLow(stress, 40, 55), (v) => `${Math.round(v)}/100`, dates)
      : "",
  ].join("");
  bindChartHover(el("wearable-cards"));
}

// ---- this week -------------------------------------------------------------
// The week answers "is this week on track?" in one glance: seven day cells
// (Mon→Sun) where the filled part is what was actually run and a dashed
// tick marks what the plan asked for. Actual-vs-planned on the SAME bar is
// the honest comparison — two separate numbers invite arithmetic errors.

const DOW = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/// Monday of the current week, local time.
function mondayThisWeek(): Date {
  const d = new Date();
  const dow = (d.getDay() + 6) % 7; // Mon=0 … Sun=6
  d.setDate(d.getDate() - dow);
  d.setHours(0, 0, 0, 0);
  return d;
}

/// A glyph with no key is a puzzle. Both charts mark cross-training with the
/// same bicycle, so both get the same one-line key — and only when a flag is
/// actually on screen, so it never explains a mark the runner cannot find.
function crossLegend(id: string, shown: boolean): void {
  const node = el(id);
  node.hidden = !shown;
  node.innerHTML = shown
    ? `${icons.bike} cross-training (not counted in the kilometres)`
    : "";
}

function renderWeek(extras: DashExtras): void {
  const monday = mondayThisWeek();
  const days: {
    key: string;
    dow: string;
    date: number;
    actualKm: number;
    cross: CrossTraining | null;
    planKm: number;
    future: boolean;
    today: boolean;
  }[] = [];
  const todayKey = ymd(new Date());
  for (let i = 0; i < 7; i++) {
    const d = new Date(monday);
    d.setDate(monday.getDate() + i);
    const key = ymd(d);
    const dayActs = extras.allActivities.filter((a) => a.startTime.slice(0, 10) === key);
    // Two separate facts per day: what was run (the bar, the mileage the plan
    // can bank) and what else was done (a marker, credit that cannot). Adding
    // the second to the first is how a spin used to look like a long run.
    const actualKm = runKm(dayActs);
    const planKm =
      extras.weekPlan
        .filter((w) => w.date === key && w.kind !== "rest")
        .reduce((s, w) => s + (w.distanceM ?? 0), 0) / 1000;
    days.push({
      key,
      dow: DOW[i],
      date: d.getDate(),
      actualKm,
      cross: crossTraining(dayActs),
      planKm,
      future: key > todayKey,
      today: key === todayKey,
    });
  }
  const maxKm = Math.max(1, ...days.map((d) => Math.max(d.actualKm, d.planKm)));
  const weekActual = days.reduce((s, d) => s + d.actualKm, 0);
  const weekPlan = days.reduce((s, d) => s + d.planKm, 0);

  // Format the range from the local Date objects, not the YYYY-MM-DD keys —
  // new Date("2026-10-19") is UTC midnight, which a western timezone renders
  // as the previous day. The loop's dates are already local.
  const sunday = new Date(monday);
  sunday.setDate(monday.getDate() + 6);
  const dow = (x: Date) =>
    x.toLocaleDateString(undefined, { month: "short", day: "numeric" });
  el("week-meta").textContent = `${dow(monday)} – ${dow(sunday)}`;
  el("week-strip").innerHTML = `
    <div class="bar-axis">
      ${days.map((d) => `<span class="bar-cap${d.today ? " now" : ""}">${d.dow}</span>`).join("")}
    </div>
    <div class="bar-plot">
      ${days
        .map((d) => {
          const act = (d.actualKm / maxKm) * 100;
          const plan = (d.planKm / maxKm) * 100;
          const cls = d.future ? " future" : d.actualKm > 0 ? " done" : d.today ? " now" : "";
          return `<div class="bar-col${cls}">
            <div class="bar-track">
              <div class="bar-fill" style="height:${Math.min(100, act).toFixed(1)}%"></div>
              ${d.planKm ? `<span class="bar-tick" style="bottom:${Math.min(100, plan).toFixed(1)}%"></span>` : ""}
            </div>
          </div>`;
        })
        .join("")}
    </div>
    <div class="bar-axis">
      ${days
        .map(
          (d) =>
            `<span class="bar-label${d.today ? " now" : ""}">${
              // A rest day gets an em-dash, not a middle dot: a lone "·" in
              // the axis reads as a rendering glitch, while a dash is the
              // typographic way to say "nothing here". The km figure and the
              // cross marker are separate lines so "8.2" and "🚲 90" never
              // collide into one unreadable cluster.
              d.actualKm ? d.actualKm.toFixed(1) : "—"
            }${
              d.cross ? `<small class="cross" title="${crossDays(d.cross)} — not running">${icons.bike} ${crossShort(d.cross)}</small>` : ""
            }</span>`,
        )
        .join("")}
    </div>`;

  bindChartInspection(el("week-strip").querySelector<HTMLElement>(".bar-plot")!, days.map((day, index) => ({
    label: `${day.dow} ${day.date}`,
    value: `${day.actualKm.toFixed(1)} km${day.planKm ? ` / ${day.planKm.toFixed(1)} km planned` : ""}`,
    x: (index + 0.5) / days.length * 100,
  })), "Running distance this week", days.findIndex((day) => day.today));
  const pct = weekPlan > 0 ? weekActual / weekPlan : null;
  crossLegend("week-legend", days.some((d) => d.cross != null));
  const pace = extras.allActivities
    .filter((a) => isRun(a) && a.startTime.slice(0, 10) >= days[0].key && a.avgPaceSKm != null)
    .sort((a, b) => b.startTime.localeCompare(a.startTime))[0];
  const weekCross = crossTraining(extras.allActivities.filter((a) => a.startTime.slice(0, 10) >= days[0].key));
  el("week-tiles").innerHTML = [
    `<div class="tile center" title="Running distance only — cross-training is its own tile"><b>${weekActual.toFixed(1)} km</b><span>run so far</span></div>`,
    weekPlan
      ? `<div class="tile center"><b>${weekPlan.toFixed(1)} km</b><span>planned</span></div>`
      : "",
    pct != null
      ? `<div class="tile center"><b>${Math.round(pct * 100)}%</b><span>of plan</span></div>`
      : "",
    `<div class="tile center"><b>${days.filter((d) => d.actualKm > 0).length}</b><span>days run</span></div>`,
    // Credit the other training rather than hiding it — but in its own unit,
    // so it can never be misread as mileage the plan should have counted.
    weekCross
      ? `<div class="tile center cross-tile" title="${crossDays(weekCross)} — not included in the run total"><b>${icons.bike} ${crossPhrase(weekCross)}</b><span>cross-training</span></div>`
      : "",
    pace ? `<div class="tile center"><b>${fmtPace(pace.avgPaceSKm)}</b><span>latest pace</span></div>` : "",
  ].join("");
}

/// The week's plan, day by day — the detail the bar chart cannot carry.
/// Each day lists its sessions with the numbers a runner needs to execute
/// them: distance, time on feet, and the pace band the plan was built on.
/// A day with no session still gets a row (an intentional rest day reads
/// differently from a gap in the plan), and a double day stacks both
/// sessions rather than hiding one behind "2 sessions".
function renderWeekPlan(extras: DashExtras): void {
  const host = el("week-plan");
  host.onclick = openPlannedWorkout;
  // With no plan there is nothing to detail, and seven rows of "No session
  // planned" would be pure noise — the chart above already says the week is
  // empty. The block only exists once a plan is being followed.
  if (!extras.weekPlan.length) {
    host.innerHTML = "";
    host.hidden = true;
    return;
  }
  host.hidden = false;
  const monday = mondayThisWeek();
  const todayKey = ymd(new Date());
  const byDay = new Map<string, PlanWorkout[]>();
  for (const w of extras.weekPlan) {
    (byDay.get(w.date) ?? byDay.set(w.date, []).get(w.date)!).push(w);
  }

  const rows: string[] = [];
  for (let i = 0; i < 7; i++) {
    const d = new Date(monday);
    d.setDate(monday.getDate() + i);
    const key = ymd(d);
    const dow = DOW[i];
    const date = d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
    const cls = key === todayKey ? " is-today" : key < todayKey ? " is-past" : " is-future";
    const ws = (byDay.get(key) ?? []).sort((a, b) => a.date.localeCompare(b.date));

    let body: string;
    if (!ws.length) {
      body = `<span class="wp-none">No session planned</span>`;
    } else {
      body = ws
        .map((w) => {
          const label = KIND_LABEL[w.kind] ?? w.kind;
          const bits: string[] = [];
          if (w.distanceM) bits.push(`${(w.distanceM / 1000).toFixed(1)} km`);
          if (w.durationMin) bits.push(`${Math.round(w.durationMin)} min`);
          if (w.paceLoSKm != null && w.paceHiSKm != null) {
            const band = (extras.plan && qualityBand(extras.plan, w)) ?? [
              w.paceLoSKm,
              w.paceHiSKm,
            ];
            bits.push(`${fmtPace(band[0])}–${fmtPace(band[1])}/km`);
          }
          return `<div class="wp-session">
            <span class="chip chip-${w.kind}">${label}</span>
            <button type="button" class="wp-title workout-link" data-workout-id="${escapeHtml(w.id)}" data-workout-date="${w.date}">${escapeHtml(w.title)}</button>
            ${bits.length ? `<span class="wp-meta">${bits.join(" · ")}</span>` : ""}
            ${w.done ? `<span class="wp-state" title="Completed">${icons.check}</span>` : ""}
          </div>`;
        })
        .join("");
    }
    rows.push(`<div class="wp-day-row${cls}">
      <div class="wp-day"><b>${dow}</b><span>${date}</span></div>
      <div class="wp-sessions">${body}</div>
    </div>`);
  }
  el("week-plan").innerHTML = rows.join("");
}

/// Weekly mileage against the plan's build cycle: the last 8 complete weeks
/// plus the in-progress one, with the target as a dashed line. This replaces
/// the old three-metre load panel — the same question ("is this week sane?")
/// answered in the unit runners actually think in.
function renderVolume(extras: DashExtras, rec: LoadRecommendation | null): void {
  const monday = mondayThisWeek();
  const weeks: {
    label: string;
    km: number;
    cross: CrossTraining | null;
    target: number | null;
    current: boolean;
  }[] = [];
  for (let i = 7; i >= 0; i--) {
    const from = new Date(monday);
    from.setDate(monday.getDate() - i * 7);
    const to = new Date(from);
    to.setDate(from.getDate() + 6);
    const fromKey = ymd(from);
    const toKey = ymd(to);
    const inWeek = extras.allActivities.filter(
      (a) => a.startTime.slice(0, 10) >= fromKey && a.startTime.slice(0, 10) <= toKey,
    );
    // The bar is running mileage — the only quantity the target line and the
    // plan's build cycle are expressed in. Cross-training is flagged, never
    // stacked on top, so a week of 0 km with a bike flag reads as "a week of
    // cycling", not "a week of nothing" or "a week of 90 km".
    weeks.push({
      label: `${from.getDate()}/${from.getMonth() + 1}`,
      km: runKm(inWeek),
      cross: crossTraining(inWeek),
      target: rec?.target ?? null,
      current: i === 0,
    });
  }
  const maxKm = Math.max(1, ...weeks.map((w) => w.km), rec?.target ?? 0);
  crossLegend("volume-legend", weeks.some((w) => w.cross != null));
  const targetPct = rec ? ((rec.target / maxKm) * 100).toFixed(1) : null;
  el("volume-meta").textContent = rec
    ? `${rec.downWeek ? `Down week ${rec.weekInCycle}/${rec.cycleLength}` : `Build week ${rec.weekInCycle}/${rec.cycleLength}`} · target ${rec.target.toFixed(0)} km`
    : "Last 8 weeks";
  el("volume-panel").innerHTML = `
    <div class="bar-plot">
      ${targetPct ? `<span class="bar-line" style="bottom:${targetPct}%" title="This week's target"></span>` : ""}
      ${weeks
        .map(
          (w) => `<div class="bar-col${w.current ? " now" : " past"}">
            <div class="bar-track"><div class="bar-fill" style="height:${((w.km / maxKm) * 100).toFixed(1)}%"></div></div>
          </div>`,
        )
        .join("")}
    </div>
    <div class="bar-axis">
      ${weeks
        .map(
          (w) =>
            `<span class="bar-label${w.current ? " now" : ""}" title="Week of ${w.label}${
              w.cross ? ` — ${crossDays(w.cross)} (not run)` : ""
            }">${w.km ? w.km.toFixed(0) : "—"}${
              w.cross ? `<i class="cross-flag">${icons.bike}</i>` : ""
            }<small>${w.label}</small></span>`,
        )
        .join("")}
    </div>`;
  bindChartInspection(el("volume-panel").querySelector<HTMLElement>(".bar-plot")!, weeks.map((week, index) => ({
    label: `Week of ${week.label}`,
    value: `${week.km.toFixed(1)} km`,
    x: (index + 0.5) / weeks.length * 100,
  })), "Weekly running mileage");
}

/// A factor bar is per-signal severity, so an empty one is a verdict rather
/// than a hole in the chart — but only once the row says so. "Sleep quality
/// 78/100 avg" beside a bare track reads as broken; beside a check it reads as
/// good news. One predicate, shared by the row and the panel-wide note, so
/// they can never disagree about what "quiet" means.
const RISK_QUIET = 0.05;
const riskQuiet = (f: RiskFactor): boolean =>
  Math.min(1, Math.max(0, f.contribution)) < RISK_QUIET;

function riskFactorBars(factors: RiskFactor[], tagQuietRows: boolean): string {
  return factors
    .map((f) => {
      const v = Math.min(1, Math.max(0, f.contribution));
      const quiet = riskQuiet(f);
      // When every row is quiet the panel says it once, above the list — six
      // "quiet" tags under one sentence would be the same fact shouted twice.
      const tag = quiet && tagQuietRows;
      return `<div class="risk-bar${quiet ? " quiet" : ""}" title="${f.detail}">
        <span class="risk-bar-label">${f.label}</span>
        <span class="meter${quiet ? " muted" : ""}" style="--value:${v.toFixed(3)}"><i></i></span>
        <span class="risk-bar-detail">${f.detail}${
          tag ? `<i class="risk-row-quiet">${icons.check}quiet</i>` : ""
        }</span>
      </div>`;
    })
    .join("");
}

/// The injury-risk panel: the gauge answers "how risky is today", the band
/// chart underneath answers "has it been, and where's the line". Same band
/// machinery as training status and recovery, cut at the same 30/55 the
/// verdict above uses — the chart's bands and the gauge's tone are the one
/// decision shown twice, never two decisions.
function renderRiskPanel(data: DashboardPayload): void {
  const risk = data.scores.injuryRisk;
  const tone = risk <= 30 ? "good" : risk <= 55 ? "warn" : "bad";
  const verdict = risk <= 30 ? "Low — train normally" : risk <= 55 ? "Moderate — ease intensity" : "High — consider a rest day";
  // An all-empty set of severity bars is real news, not a rendering gap:
  // nothing is currently pushing the risk up. Stated once for the whole panel
  // in that case, and per row whenever at least one signal is loud.
  const allQuiet = data.riskFactors.length > 0 && data.riskFactors.every(riskQuiet);
  const bars = riskFactorBars(data.riskFactors, !allQuiet);
  const quiet = allQuiet
    ? `<p class="risk-quiet">Nothing is pushing your risk up right now — every signal below is quiet.</p>`
    : "";
  const series = data.scoreHistory.map((p) => p.risk);
  const chart =
    series.length >= 2
      ? `<div class="risk-chart">
          ${bandChart(series, [30, 55], ["zone-risk-low", "zone-risk-moderate", "zone-risk-high"], 0, 100, (v) => `${Math.round(v)}%`, data.scoreHistory.map((point) => point.date))}
          <div class="band-foot">
            <span class="chart-axis">Injury risk · ${fmtDate(data.scoreHistory[0].date)} → today</span>
            <div class="zone-legend">
              <span class="zone-key"><i class="zone-risk-high"></i>High</span>
              <span class="zone-key"><i class="zone-risk-moderate"></i>Moderate</span>
              <span class="zone-key"><i class="zone-risk-low"></i>Low</span>
            </div>
          </div>
        </div>`
      : "";
  // The panel itself carries `.signal.tone-*` (the line colour hook) and the
  // gauge number carries `.live-value`, so hovering the chart swaps the
  // gauge's figure for that day's risk — the same scrub the status chart
  // does with its verdict word. The tone class is re-stamped per render.
  const panel = el("risk-panel");
  panel.classList.remove("tone-good", "tone-warn", "tone-bad");
  panel.classList.add("signal", `tone-${tone}`);
  panel.innerHTML = `
    <div class="band-verdict">
      <div class="card-value band-value">${risk}%</div>
      <div class="score-hint band-reason">${verdict}</div>
    </div>
    ${chart || `<div class="chart-empty">Not enough history</div>`}
    <div class="risk-factors">${quiet}${bars}</div>`;
  bindChartHover(panel);
}

/// The 7-day load target row: this week's goal from the build cycle, measured
/// against the acute load already banked. The bar fills toward the target, so
/// "how much room is left this week" reads without arithmetic. On a down week
/// the target is deliberately below chronic load, which is the point of the
/// cycle — the row says so rather than letting an easy number look like a
/// shortfall.
function targetRow(rec: LoadRecommendation | null, acute: number): string {
  if (!rec) return "";
  const pct = rec.target > 0 ? acute / rec.target : 0;
  const tone = pct > 1.15 ? "bad" : pct > 1 ? "warn" : "good";
  const label = rec.downWeek
    ? `Down week (${rec.weekInCycle}/${rec.cycleLength})`
    : `Week ${rec.weekInCycle} of ${rec.cycleLength}`;
  return `<div class="load-bar-row">
      <span class="load-bar-label">${label}</span>
      <span class="meter ${tone}" style="--value:${Math.min(1, Math.max(0, pct)).toFixed(3)}"><i></i></span>
      <span class="load-bar-val chip ${tone}">${acute.toFixed(1)} / ${rec.target.toFixed(1)}</span>
    </div>`;
}

// ---- training status: the boundaries chart -------------------------------
// The COROS-style answer to "am I doing too much?": one line (acute ÷
// chronic load) drawn across horizontal bands, so the eye reads the zone
// before the number. The bands and the headline verdict come from the same
// backend decision (metrics::status_from_ratio), so chart and verdict can
// never disagree about where a boundary sits.

// The boundaries are never hardcoded here: they arrive on the payload as
// `data.thresholds`, straight from the same Rust `Thresholds` the verdicts are
// computed with. An earlier copy of this file kept a local [0.8, 1.3, 1.5] and
// drifted away from Settings the moment a runner edited a boundary.

/// The band chart: one line drawn across horizontal colour-coded zones, used
/// by all three "where am I on the scale?" panels — training status, recovery,
/// and injury risk. Unlike the sparklines the y-domain is anchored rather than
/// fitted to the data, because bands that moved with the series would stop
/// meaning anything; it only grows when a value actually climbs past the top,
/// which is exactly when it has to be visible. `bands` are the zone classes
/// bottom→top, one more than `bounds`, and they walk the green scale in strict
/// order so every band chart is the same dark→light ramp. Registers its spec
/// in `chartSpecs`, so the shared hover-scrub handler works unchanged.
function bandChart(
  vs: number[],
  bounds: number[],
  bands: string[],
  min: number,
  top: number,
  format: (v: number) => string,
  dates: string[],
): string {
  if (vs.length < 2) return "";
  const span = top - min;
  const y = (v: number) => chartY(v, min, span);
  const pts = vs.map(
    (v, i) => [(i / (vs.length - 1)) * CHART_W, y(v)] as [number, number],
  );
  const line = smoothPath(pts);
  const uid = `spark-${chartUid++}`;
  const band = (lo: number, hi: number, cls: string) =>
    `<rect class="zone ${cls}" x="0" y="${y(hi).toFixed(2)}" width="${CHART_W}" height="${(y(lo) - y(hi)).toFixed(2)}"/>`;
  chartSpecs.set(uid, { values: vs, labels: dates.map(fmtDate), min, span, format });
  // Boundary captions ride the same % geometry the hover dot uses, so they
  // stay glued to their lines at any rendered size. A caption is dropped when
  // it would land on top of the one below it — with boundaries set close
  // together (1.05 and 0.95 sit ~5% apart) the labels otherwise overlap into
  // mush. The dashed line still marks the boundary; only its number is hidden.
  const tick = (v: number) => (chartY(v, min, span) / CHART_H) * 100;
  const MIN_TICK_GAP = 7; // % of chart height — roughly one label's height
  let lastKept = -Infinity;
  // Sort by screen position rather than by value: chartY inverts the axis, and
  // reading top-to-bottom is what "does this label collide with the previous
  // one" actually means, whichever way the boundaries were set.
  const labels = bounds
    .map((b) => ({ b, t: tick(b) }))
    .sort((a, z) => a.t - z.t)
    .filter((x) => {
      if (x.t - lastKept < MIN_TICK_GAP) return false;
      lastKept = x.t;
      return true;
    })
    .map(
      (x) => `<span class="band-label" aria-hidden="true" style="top:${x.t.toFixed(1)}%">${x.b}</span>`,
    )
    .join("");
  const edges = [min, ...bounds, top];
  return `<div class="spark-wrap band-spark" data-chart="${uid}">
    <svg class="spark" viewBox="0 0 ${CHART_W} ${CHART_H}" preserveAspectRatio="none" aria-hidden="true">
      <defs>
        <clipPath id="${uid}-clip" clipPathUnits="userSpaceOnUse">
          <rect class="spark-wipe" style="--spark-w:${CHART_W + 4}" x="-2" y="-6" width="${CHART_W + 4}" height="${CHART_H + 12}"/>
        </clipPath>
      </defs>
      ${bands.map((cls, i) => band(edges[i], edges[i + 1], cls)).join("")}
      ${bounds.map(
        (b) => `<line class="zone-edge" x1="0" x2="${CHART_W}" y1="${y(b).toFixed(2)}" y2="${y(b).toFixed(2)}"/>`,
      ).join("")}
      <g clip-path="url(#${uid}-clip)">
        <path class="spark-line" d="${line}" fill="none" stroke="currentColor"
          stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
          vector-effect="non-scaling-stroke"/>
      </g>
    </svg>
    ${labels}
    <span class="chart-dot" hidden></span>
  </div>`;
}

/// Ratio line over the runner's zone bands. 1.7 rather than a rounder number
/// as the floor of the domain: it keeps the boundary labels far enough apart
/// to read, and the domain only grows past it when a ratio really does go
/// higher. Grows to clear the top boundary too, so a widened Optimized band
/// never pushes its own label off the chart.
function statusChart(points: LoadPoint[], th: Thresholds): string {
  const samples = points.filter((point) => point.ratio != null);
  const vs = samples.map((point) => point.ratio!);
  const bounds = [th.ratioDecreasingHi, th.ratioResumingHi, th.ratioMaintainingHi, th.ratioOptimizedHi];
  const top = Math.max(1.7, th.ratioOptimizedHi * 1.15, ...vs.map((v) => v * 1.12));
  return bandChart(
    vs,
    bounds,
    ["zone-decreasing", "zone-resuming", "zone-maintaining", "zone-optimized", "zone-excessive"],
    0.3,
    top,
    (v) => `${v.toFixed(2)}×`,
    samples.map((point) => point.date),
  );
}

function renderStatusPanel(data: DashboardPayload): void {
  const panel = el("status-panel");
  const st = data.trainingStatus;
  const history = data.loadHistory;
  if (!st || history.length < 2) {
    panel.innerHTML = `<p class="muted">No training load data yet — once your watch has synced a few runs, this chart shows your load against the zones.</p>`;
    return;
  }
  // COROS lists the scale high→low (Excessive first), which is also the
  // chart's top-to-bottom reading order — the legend follows it so the key
  // and the bands scan identically.
  const legend = [
    ["zone-excessive", "Excessive"],
    ["zone-optimized", "Optimized"],
    ["zone-maintaining", "Maintaining"],
    ["zone-resuming", "Resuming / Performance"],
    ["zone-decreasing", "Decreasing"],
  ]
    .map(([c, l]) => `<span class="zone-key"><i class="${c}"></i>${l}</span>`)
    .join("");
  // `.signal` + `.live-value` are what bindChartHover looks for: hovering the
  // chart swaps the verdict word for the ratio at that day, like the cards.
  // No eyebrow above the verdict — the section heading already says what this
  // is, so a second label would only echo it.
  // The acute/chronic readings that used to live in their own panel sit
  // under the verdict here — same numbers, one place, next to the chart
  // they explain. The zone word and colour come from the same backend
  // decision the bands are drawn from, so they can never disagree.
  const load = data.load;
  const meters = load
    ? `<div class="load-bars band-meters">
        <div class="load-bar-row">
          <span class="load-bar-label">Acute (7d)</span>
          <span class="meter" style="--value:${Math.min(1, (load.shortTerm ?? 0) / 120).toFixed(3)}"><i></i></span>
          <span class="load-bar-val">${(load.shortTerm ?? 0).toFixed(1)}</span>
          <span class="load-bar-hint">What you've run this week — how tired you are right now.</span>
        </div>
        <div class="load-bar-row">
          <span class="load-bar-label">Chronic (28d)</span>
          <span class="meter muted" style="--value:${Math.min(1, (load.longTerm ?? 0) / 120).toFixed(3)}"><i></i></span>
          <span class="load-bar-val">${(load.longTerm ?? 0).toFixed(1)}</span>
          <span class="load-bar-hint">Your average over the last month — what your body is used to.</span>
        </div>
        ${targetRow(data.loadRecommendation, load.shortTerm ?? 0)}
      </div>`
    : "";
  panel.innerHTML = `<div class="band-body signal tone-${st.tone}">
    <div class="band-verdict">
      <div class="card-value band-value"><span class="live-value">${st.label}</span></div>
      <div class="score-hint band-reason">${st.reason}</div>
      ${meters}
    </div>
    <div class="band-chart">
      ${statusChart(history, data.thresholds)}
      <div class="band-foot">
        <span class="chart-axis">Acute ÷ chronic · ${fmtDate(history[0].date)} → today</span>
        <div class="zone-legend">${legend}</div>
      </div>
    </div>
  </div>`;
  bindChartHover(panel);
}

/// The recovery score drawn across its own bands (Exhausted / Fatigued /
/// Normal / Fresh), the same chart language as training status. The bands cut
/// at the runner's own thresholds and the verdict word comes from the
/// backend's recovery_band, so chart and word can never disagree — and the
/// score history's newest point IS the headline number the readiness card
/// shows, so the line always ends inside the band it names.
function renderRecoveryPanel(data: DashboardPayload): void {
  const panel = el("recovery-panel");
  const band = data.recoveryBand;
  const series = data.scoreHistory.map((p) => p.recovery);
  if (!band || series.length < 2) {
    panel.innerHTML = `<p class="muted">No recovery data yet — your wearable needs a few nights of sleep data first.</p>`;
    return;
  }
  const th = data.thresholds;
  const legend = [
    ["zone-fresh", "Fresh"],
    ["zone-normal", "Normal"],
    ["zone-fatigued", "Fatigued"],
    ["zone-exhausted", "Exhausted"],
  ]
    .map(([c, l]) => `<span class="zone-key"><i class="${c}"></i>${l}</span>`)
    .join("");
  panel.innerHTML = `<div class="band-body signal tone-${band.tone}">
    <div class="band-verdict">
      <div class="card-value band-value"><span class="live-value">${band.label}</span></div>
      <div class="score-hint band-reason">Recovery ${data.scores.recoveryScore}/100 from sleep quality, resting HR and HRV.</div>
    </div>
    <div class="band-chart">
      ${bandChart(
        series,
        [th.recoveryExhausted + 1, th.recoveryFatigued + 1, th.recoveryNormal + 1],
        ["zone-exhausted", "zone-fatigued", "zone-normal", "zone-fresh"],
        0,
        100,
        (v) => `${Math.round(v)}/100`,
        data.scoreHistory.map((point) => point.date),
      )}
      <div class="band-foot">
        <span class="chart-axis">Recovery score · ${fmtDate(data.scoreHistory[0].date)} → today</span>
        <div class="zone-legend">${legend}</div>
      </div>
    </div>
  </div>`;
  bindChartHover(panel);
}

function renderDashboard(data: DashboardPayload, extras: DashExtras): void {
  chartSpecs.clear();
  const { fitness, activities } = data;
  renderTodayHero(data, extras);
  renderPainBanner(data);

  renderWeek(extras);
  renderWeekPlan(extras);
  renderVolume(extras, data.loadRecommendation);
  renderRecoveryPanel(data);
  renderRiskPanel(data);
  renderWearables(data.health);
  renderStatusPanel(data);

  const fitCards: string[] = [];
  if (fitness) {
    if (fitness.vo2max != null)
      fitCards.push(card("VO₂max", fitness.vo2max.toFixed(1)));
    if (fitness.thresholdSecKm != null)
      fitCards.push(card("Threshold", `${fmtPace(fitness.thresholdSecKm)}/km`));
    if (fitness.predict5kS != null) fitCards.push(card("5K pred.", timeHms(fitness.predict5kS)));
    if (fitness.predict10kS != null) fitCards.push(card("10K pred.", timeHms(fitness.predict10kS)));
    if (fitness.predictHalfS != null) fitCards.push(card("Half pred.", timeHms(fitness.predictHalfS)));
    if (fitness.predictMarathonS != null)
      fitCards.push(card("Marathon pred.", timeHms(fitness.predictMarathonS)));
  }
  el("fitness-cards").innerHTML =
    fitCards.join("") || `<p class="muted">No fitness data yet — sync COROS first.</p>`;

  const tbody = el("activities-table").querySelector("tbody")!;
  tbody.innerHTML = activities
    .map(
      (a) => `<tr data-id="${a.id}" tabindex="0" role="button" title="Open details"
        aria-label="${a.sport} on ${fmtDate(a.startTime)}, open details">
        <td>${fmtDate(a.startTime)}</td>
        <td>${a.sport}</td>
        <td>${fmtKm(a.distanceM)}</td>
        <td>${fmtDuration(a.durationS)}</td>
        <td>${a.avgHr ?? "—"}</td>
        <td>${fmtPace(a.avgPaceSKm)}/km</td>
      </tr>`,
    )
    .join("");
  if (activities.length === 0) {
    tbody.innerHTML = `<tr><td colspan="6" class="muted">No activities yet — connect and sync COROS.</td></tr>`;
  }
  // Cards are in the DOM now — measure every card row so the ones that
  // overflow show their paging buttons. (showDataSections re-measures once
  // #dash becomes visible, since rows in a hidden container measure zero.)
  initCarousels();
}

function card(label: string, value: string, hint = ""): string {
  return `<div class="card"><div class="card-value">${value}</div><div class="card-label">${label}</div>${
    hint ? `<div class="score-hint">${hint}</div>` : ""
  }</div>`;
}

/// The ask, right where training decisions get made: unresolved pain plus,
/// when one exists, the pending plan-change offer with Accept right here.
/// Accepting applies the edits; the full review lives on the Injuries page.
function renderPainBanner(data: DashboardPayload): void {
  const banner = el("pain-banner");
  const pain = data.activePain;
  const offer = data.pendingProposal;
  if (!pain && !offer) {
    banner.hidden = true;
    banner.innerHTML = "";
    return;
  }
  banner.hidden = false;
  banner.classList.toggle("has-proposal", offer != null);
  const label = (key: string) =>
    key.split(".").reverse().join(" ").replace(/_/g, " ");
  const painText = pain
    ? `${label(pain.bodyPart)} hurting at ${pain.intensity}/10 since ${fmtDate(pain.startedOn)}`
    : `${label(offer!.bodyPart)} is still in your journal`;
  banner.innerHTML = `
    <p>${painText}${offer ? ` — <strong>${offer.summary}</strong>` : ""}</p>
    <span class="pain-banner-actions">
      <button id="pain-review" class="secondary">Review</button>
      ${offer ? `<button id="pain-accept" class="primary">Accept change</button>` : ""}
    </span>`;
  el<HTMLButtonElement>("pain-review").addEventListener("click", () => {
    window.location.href = "injury.html";
  });
  if (offer) {
    el<HTMLButtonElement>("pain-accept").addEventListener("click", async (ev) => {
      const btn = ev.currentTarget as HTMLButtonElement;
      btn.disabled = true;
      btn.textContent = "Applying…";
      try {
        await tauri.applyProposal("coros", offer.id);
        banner.hidden = true;
        banner.innerHTML = "";
      } catch (err) {
        fail(err);
        btn.disabled = false;
        btn.textContent = "Accept change";
      }
    });
  }
}

function timeHms(s: number): string {
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = Math.round(s % 60);
  return h > 0
    ? `${h}:${String(m).padStart(2, "0")}:${String(sec).padStart(2, "0")}`
    : `${m}:${String(sec).padStart(2, "0")}`;
}

/// Basemap style for the configured pmtiles archive (offline) or OSM raster
/// tiles. Shared by the heatmap and the activity detail route map.
function buildMapStyle(settings: Settings): maplibregl.StyleSpecification {
  const pmtiles = settings.pmtilesPath;
  if (pmtiles && inTauri) {
    // Offline tiles: the pmtiles Protocol serves range requests from the
    // archive exposed through Tauri's asset protocol (supports HTTP Range).
    const protocol = new Protocol();
    maplibregl.addProtocol("pmtiles", protocol.tile);
    const archiveUrl = pmtiles.startsWith("~/")
      ? pmtiles // unlikely; backend resolves absolute paths only
      : convertFileSrc(pmtiles);
    const tileUrl = `pmtiles://${archiveUrl}/{z}/{x}/{y}`;
    return {
      version: 8,
      sources: {
        basemap: {
          type: "raster",
          tiles: [tileUrl],
          tileSize: 256,
          attribution: "PMTiles",
        },
      },
      layers: [
        { id: "bg", type: "background", paint: { "background-color": "#0c1210" } },
        { id: "basemap", type: "raster", source: "basemap" },
      ],
    };
  }
  return {
    version: 8,
    sources: {
      osm: {
        type: "raster",
        tiles: ["https://tile.openstreetmap.org/{z}/{x}/{y}.png"],
        tileSize: 256,
        attribution: "© OpenStreetMap contributors",
      },
    },
    layers: [
      { id: "bg", type: "background", paint: { "background-color": "#0c1210" } },
      { id: "osm", type: "raster", source: "osm" },
    ],
  };
}

/// The map's own zoom/compass/fit controls, drawn in the app's material.
/// MapLibre's built-ins are unusable here: their icons are baked into CSS
/// background images with a hardcoded colour, so they cannot follow the
/// light/dark scheme (the app themes off `[data-theme]`, not the media query),
/// and the global glass-pill `button` rule would restyle their boxes. This
/// control draws the same three actions with the app's Tabler icons and glass
/// tokens. `fit` is the one the library has no equivalent for: it returns the
/// view to the data the map was built around — the heatmap points, or the
/// run's track — which is the question a runner actually asks after panning
/// away. The object is structural (maplibre's `IControl` is not exported as a
/// type); `addControl` accepts anything with this shape.
function mapControls(fit: () => void) {
  const specs: { icon: string; title: string; run: (map: maplibregl.Map) => void }[] = [
    { icon: icons.plus, title: "Zoom in", run: (m) => m.zoomIn() },
    { icon: icons.minus, title: "Zoom out", run: (m) => m.zoomOut() },
    { icon: icons.fit, title: "Fit to data", run: () => fit() },
    { icon: icons.locate, title: "Reset north", run: (m) => m.resetNorth() },
  ];
  let container: HTMLElement | null = null;
  return {
    onAdd(map: maplibregl.Map): HTMLElement {
      const group = document.createElement("div");
      // `.maplibregl-ctrl` earns pointer-events and the corner margin from the
      // library; `.map-ctrl` is the app's own glass box.
      group.className = "maplibregl-ctrl map-ctrl";
      for (const s of specs) {
        const btn = document.createElement("button");
        btn.type = "button";
        btn.title = s.title;
        btn.setAttribute("aria-label", s.title);
        btn.innerHTML = s.icon;
        btn.addEventListener("click", () => s.run(map));
        group.appendChild(btn);
      }
      container = group;
      return group;
    },
    onRemove(): void {
      container?.remove();
      container = null;
    },
    getDefaultPosition: () => "top-right" as const,
  };
}

async function initMap(points: HeatPoint[], settings: Settings): Promise<void> {
  const mapEl = el("map");
  if (points.length === 0) {
    mapEl.innerHTML = `<div class="map-empty">Run heatmap appears once activities with GPS tracks are synced.</div>`;
    return;
  }

  const style = buildMapStyle(settings);

  // Bounds need no map, so they are built up front: that lets the fit control
  // close over them instead of reaching into the load handler.
  const bounds = new maplibregl.LngLatBounds(
    [points[0].lon, points[0].lat],
    [points[0].lon, points[0].lat],
  );
  for (const p of points) bounds.extend([p.lon, p.lat]);

  const map = new maplibregl.Map({
    container: "map",
    style,
    center: [points[0].lon, points[0].lat],
    zoom: 11,
  });
  map.addControl(mapControls(() => map.fitBounds(bounds, { padding: 40, maxZoom: 13 })));

  map.on("load", () => {
    const fc = {
      type: "FeatureCollection" as const,
      features: points.map((p) => ({
        type: "Feature" as const,
        properties: { weight: p.weight },
        geometry: { type: "Point" as const, coordinates: [p.lon, p.lat] },
      })),
    };
    map.addSource("heat", { type: "geojson", data: fc });
    map.addLayer({
      id: "heat",
      type: "heatmap",
      source: "heat",
      paint: {
        "heatmap-weight": ["interpolate", ["linear"], ["get", "weight"], 0, 0.2, 1, 1],
        "heatmap-intensity": ["interpolate", ["linear"], ["zoom"], 8, 0.6, 14, 2],
        "heatmap-radius": ["interpolate", ["linear"], ["zoom"], 8, 6, 14, 22],
        "heatmap-color": [
          "interpolate",
          ["linear"],
          ["heatmap-density"],
          0, "rgba(0,0,0,0)",
          0.25, "rgba(16,110,80,0.55)",
          0.55, "rgba(52,211,153,0.75)",
          0.8, "rgba(110,231,183,0.88)",
          1, "rgba(209,250,229,0.95)",
        ],
        "heatmap-opacity": 0.85,
      },
    });
    map.fitBounds(bounds, { padding: 40, maxZoom: 13 });
  });
}

// ---- activity detail sheet --------------------------------------------------
// Clicking a history row opens a slide-over with the run's GPS track, the
// session data, the recovery signals for that day, and the injury-risk
// heuristics evaluated *as of* that day.

let sheetMap: maplibregl.Map | null = null;
let activitySheet: Sheet | null = null;

/// The activity sheet is created once and reused. Its map is torn down only
/// when the sheet has actually finished closing (onClosed), never at the start
/// of the exit — so reversing a dismissal mid-slide keeps the map alive.
function getActivitySheet(): Sheet {
  if (!activitySheet) {
    activitySheet = createSheet({
      label: "Activity detail",
      onClosed: () => {
        sheetMap?.remove();
        sheetMap = null;
      },
    });
  }
  return activitySheet;
}

function statTile(value: string, label: string): string {
  return `<div class="tile"><b>${value}</b><span>${label}</span></div>`;
}

/// Fill the sheet with one activity and return the (fresh) map container so
/// the caller can mount the route map into it.
function renderActivitySheet(sheet: Sheet, d: ActivityDetail): HTMLElement {
  const a = d.activity;
  sheet.setTitle(
    `${a.sport} · ${new Date(a.startTime).toLocaleDateString(undefined, { weekday: "short", month: "short", day: "numeric" })}`,
    new Date(a.startTime).toLocaleString(undefined, { hour: "2-digit", minute: "2-digit" }),
  );
  sheet.body.innerHTML = `
    <section><h3>Route</h3><div class="sheet-map"></div></section>
    <section><h3>Session data</h3><div class="sheet-data tile-grid"></div></section>
    <section><h3>Injury risk that day</h3><div class="sheet-risk risk-panel"></div></section>`;
  // Look these up within the sheet's own subtree, never document-wide: the
  // sheet is a detached-then-attached component, so a global getElementById is
  // both racy (a reopen mid-fetch) and wrong (it would match a stale node).
  const mapEl = sheet.body.querySelector<HTMLElement>(".sheet-map")!;
  const dataEl = sheet.body.querySelector<HTMLElement>(".sheet-data")!;
  const riskEl = sheet.body.querySelector<HTMLElement>(".sheet-risk")!;

  // Session data + that day's recovery signals + fitness as of the day.
  const tiles: string[] = [
    statTile(fmtKm(a.distanceM), "distance"),
    statTile(fmtDuration(a.durationS), "time"),
    statTile(`${fmtPace(a.avgPaceSKm)}/km`, "avg pace"),
    statTile(a.avgHr != null ? `${a.avgHr}` : "—", "avg hr"),
    statTile(a.maxHr != null ? `${a.maxHr}` : "—", "max hr"),
    statTile(a.elevationGainM != null ? `${Math.round(a.elevationGainM)} m` : "—", "ascent"),
  ];
  if (d.health) {
    const h = d.health;
    if (h.sleepScore != null) tiles.push(statTile(`${h.sleepScore}`, "sleep score"));
    if (h.restingHr != null) tiles.push(statTile(`${h.restingHr}`, "resting hr"));
    if (h.hrvAvg != null) tiles.push(statTile(`${Math.round(h.hrvAvg)}`, "hrv ms"));
    if (h.stressAvg != null) tiles.push(statTile(`${h.stressAvg}/100`, "stress"));
  }
  if (d.fitness?.vo2max != null) tiles.push(statTile(d.fitness.vo2max.toFixed(1), "vo2max"));
  if (d.fitness?.thresholdSecKm != null)
    tiles.push(statTile(`${fmtPace(d.fitness.thresholdSecKm)}/km`, "threshold"));
  dataEl.innerHTML = tiles.join("");

  // Injury-risk heuristics as of that day (same gauge + factor bars as the
  // Training tab, evaluated on the day of the activity).
  const risk = d.injuryRisk;
  const tone = risk <= 30 ? "good" : risk <= 55 ? "warn" : "bad";
  const verdict =
    risk <= 30
      ? "Low — train normally"
      : risk <= 55
        ? "Moderate — ease intensity"
        : "High — consider a rest day";
  const bars = riskFactorBars(d.riskFactors, !d.riskFactors.every(riskQuiet));
  const quiet = d.riskFactors.every(riskQuiet)
    ? `<p class="risk-quiet">Nothing was pushing the risk up that day — every signal below was quiet.</p>`
    : "";
  riskEl.innerHTML = `
    <div class="risk-readout ${tone}">
      <div class="risk-gauge">
        <svg viewBox="0 0 120 120" aria-hidden="true">
          <circle class="gauge-track" cx="60" cy="60" r="52" fill="none" stroke-width="10"/>
          <circle class="gauge-fill" cx="60" cy="60" r="52" fill="none" stroke-width="10"
            stroke-dasharray="${(2 * Math.PI * 52).toFixed(1)}"
            stroke-dashoffset="${(2 * Math.PI * 52 * (1 - risk / 100)).toFixed(1)}"/>
        </svg>
        <div class="risk-num">${risk}<span>%</span></div>
      </div>
      <div class="risk-verdict">${verdict}</div>
    </div>
    <div class="risk-factors">${quiet}${bars}</div>`;

  return mapEl;
}

function renderSheetMap(d: ActivityDetail, settings: Settings, mapEl: HTMLElement): void {
  sheetMap?.remove();
  sheetMap = null;
  if (d.track.length === 0) {
    mapEl.innerHTML = `<div class="map-empty">No GPS track for this activity.</div>`;
    return;
  }
  mapEl.innerHTML = "";
  const first = d.track[0];
  const coords = d.track.map((p) => [p.lon, p.lat]) as [number, number][];
  // Built before the map so the fit control can restore the run's own extent.
  const bounds = new maplibregl.LngLatBounds(coords[0], coords[0]);
  for (const c of coords) bounds.extend(c);
  sheetMap = new maplibregl.Map({
    container: mapEl,
    style: buildMapStyle(settings),
    center: [first.lon, first.lat],
    zoom: 12,
  });
  sheetMap.addControl(mapControls(() => sheetMap?.fitBounds(bounds, { padding: 36, maxZoom: 16 })));
  sheetMap.on("load", () => {
    if (!sheetMap) return;
    // Start / finish markers carry a `kind` property so the circle layer can
    // colour them differently from the line.
    sheetMap.addSource("track", {
      type: "geojson",
      data: {
        type: "FeatureCollection",
        features: [
          {
            type: "Feature",
            properties: { kind: "line" },
            geometry: { type: "LineString", coordinates: coords },
          },
          {
            type: "Feature",
            properties: { kind: "start" },
            geometry: { type: "Point", coordinates: coords[0] },
          },
          {
            type: "Feature",
            properties: { kind: "finish" },
            geometry: { type: "Point", coordinates: coords[coords.length - 1] },
          },
        ],
      },
    });
    sheetMap.addLayer({
      id: "track-glow",
      type: "line",
      source: "track",
      filter: ["==", ["get", "kind"], "line"],
      paint: {
        "line-color": "#0c1210",
        "line-width": 6,
        "line-opacity": 0.55,
      },
    });
    sheetMap.addLayer({
      id: "track-line",
      type: "line",
      source: "track",
      filter: ["==", ["get", "kind"], "line"],
      paint: {
        "line-color": "#34d399",
        "line-width": 2.5,
        "line-opacity": 0.95,
      },
    });
    sheetMap.addLayer({
      id: "track-ends",
      type: "circle",
      source: "track",
      filter: ["!=", ["get", "kind"], "line"],
      paint: {
        "circle-radius": 5,
        // Start and finish are separated by depth within the green scale rather
        // than hue: the origin sits back as deep green, the destination comes
        // forward as the palest mint (the top stop of the heatmap ramp).
        "circle-color": [
          "match",
          ["get", "kind"],
          "start",
          "#059669",
          "finish",
          "#d1fae5",
          "#6ee7b7",
        ],
        "circle-stroke-color": "#0c1210",
        "circle-stroke-width": 1.5,
      },
    });
    sheetMap.fitBounds(bounds, { padding: 36, maxZoom: 16 });
  });
}

async function openActivitySheet(activityId: number): Promise<void> {
  const sheet = getActivitySheet();
  // Show the surface immediately with a loading shell, THEN fetch. The sheet
  // appearing on tap (not after the round-trip) is what makes it feel direct.
  sheet.body.innerHTML = `<p class="sheet-loading"><svg class="btn-spinner" viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" aria-hidden="true"><path d="M12 3a9 9 0 1 0 9 9" opacity="0.9"/></svg>Loading activity…</p>`;
  sheet.setTitle("Activity", "");
  sheet.open();
  const token = sheet.beginLoad();
  try {
    const [detail, settings] = await Promise.all([
      tauri.activityDetail(activityId, currentRange),
      tauri.getSettings(),
    ]);
    // A close/reopen happened while we were fetching — this response is stale.
    if (!sheet.isCurrent(token)) return;
    const mapEl = renderActivitySheet(sheet, detail);
    renderSheetMap(detail, settings, mapEl);
  } catch (e) {
    if (!sheet.isCurrent(token)) return;
    sheet.body.innerHTML = `<p class="notice bad">Could not load this activity.</p>`;
    fail(e);
  }
}

function initActivitySheet(): void {
  const tbody = el("activities-table").querySelector("tbody")!;
  const open = (e: Event) => {
    const tr = (e.target as HTMLElement).closest<HTMLElement>("tr[data-id]");
    if (tr) void openActivitySheet(Number(tr.dataset.id));
  };
  tbody.addEventListener("click", open);
  tbody.addEventListener("keydown", (e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      open(e);
    }
  });
}

const CTA: Record<string, { title: string; body: string; label: string; href: string }> = {
  connect: {
    title: "Connect your COROS watch",
    body: "Runalytics reads your runs, sleep and training load straight from the official COROS API. Nothing leaves your Mac.",
    label: "Connect device",
    href: "settings.html",
  },
  sync: {
    title: "Pull your data from COROS",
    body: "Connected! Now sync your recent activities, health and fitness so the dashboard has something to show.",
    label: "Sync now",
    href: "",
  },
  plan: {
    title: "Create your first plan",
    body: "Your history is in. A plan is what the calendar and the injury page describe — set one up from Settings, then build it on the Plan page.",
    label: "Set up a plan",
    href: "settings.html#setup",
  },
  train: {
    title: "You're all set — train!",
    body: "Plan pushed to COROS. Follow the calendar, keep syncing, and the adaptive engine softens sessions when recovery dips.",
    label: "Open calendar",
    href: "calendar.html",
  },
};

function renderEmptyState(step: string): void {
  const cta = CTA[step] ?? CTA.connect;
  const empty = el("empty");
  empty.hidden = false;
  // The sync step acts right here — no hop to Settings; everything else
  // links to the page that owns the next step.
  empty.innerHTML = `<div class="empty-state">
    <h3>${cta.title}</h3>
    <p>${cta.body}</p>
    ${
      cta.href
        ? `<a class="btn big" href="${cta.href}">${cta.label}</a>`
        : `<button class="big" type="button" id="empty-sync">${cta.label}</button>`
    }
  </div>`;
  // The empty state carries its own call to action — hide the dashboard so
  // the user sees exactly one path forward.
  el("dash").hidden = true;
  const syncBtn = document.getElementById("empty-sync");
  if (syncBtn) {
    syncBtn.addEventListener("click", async () => {
      const btn = syncBtn as HTMLButtonElement;
      btn.disabled = true;
      btn.textContent = "Syncing…";
      try {
        await tauri.syncNow("coros");
        // sync://completed re-loads the app data through the page listener.
      } catch (e) {
        fail(e);
        btn.disabled = false;
        btn.textContent = cta.label;
      }
    });
  }
}

function showDataSections(): void {
  el("empty").hidden = true;
  el("dash").hidden = false;
  // A row rendered while its container was hidden measured zero width and
  // looked non-overflowing; re-measure now that it is on screen.
  initCarousels();
}

let mapInitialized = false;
let pendingMapPoints: HeatPoint[] | null = null;
let pendingMapSettings: Settings | null = null;

/// The map sits at the bottom of the page: MapLibre needs a visible
/// container to size itself, so it initialises only once #dash is shown.
function maybeInitMap(): void {
  if (mapInitialized || !pendingMapPoints || !pendingMapSettings) return;
  mapInitialized = true;
  initMap(pendingMapPoints, pendingMapSettings).catch(fail);
}

async function main(): Promise<void> {
  initSyncIndicator();
  initSegmented();
  // The startup auto-sync usually finishes after the first render — when it
  // does, pull fresh data so the dashboard reflects it without a reload.
  window.addEventListener("runalytics:sync-completed", () => {
    void loadAppData().catch(() => undefined);
  });
  initActivitySheet();

  // Setup lives in Settings now; the dashboard just keeps the nav honest
  // about which tabs a plan-less account should show. The tutorial (first
  // run only) comes first, then the dashboard.
  const proceed = async () => {
    let flow;
    try {
      flow = await fetchFlowState();
    } catch (e) {
      // Outside Tauri or backend unreachable: show guidance, not an alert storm.
      try {
        renderEmptyState("connect");
      } catch {
        fail(e);
      }
      return;
    }
    await initNavGate(false);
    await loadAppData(flow);
  };

  if (onboardingPending()) {
    startOnboarding(() => void proceed());
    return;
  }
  await proceed();
}

async function loadAppData(given?: FlowState): Promise<void> {
  try {
    const flow = given ?? await fetchFlowState();
    const step = nextAction(flow);
    if (step !== "train" || !flow.pushed) {
      // Not everything is wired yet — guide the user, but still show data
      // if a sync has happened.
      if (flow.lastSync == null) {
        renderEmptyState(step);
        return;
      }
    }
    const monday = mondayThisWeek();
    const sunday = new Date(monday);
    sunday.setDate(monday.getDate() + 6);
    const [data, points, settings, allActivities, planPair, weekPlan] = await Promise.all([
      tauri.dashboard(currentRange),
      tauri.heatPoints(),
      tauri.getSettings(),
      tauri.allActivities().catch(() => [] as Activity[]),
      tauri.currentPlan().catch(() => null),
      tauri.workoutsBetween(ymd(monday), ymd(sunday)).catch(() => [] as PlanWorkout[]),
    ]);
    if (data.activities.length === 0 && data.load == null && data.fitness == null) {
      renderEmptyState(step);
      return;
    }
    const extras: DashExtras = { allActivities, weekPlan, plan: planPair?.[0] ?? null };
    showDataSections();
    renderDashboard(data, extras);
    // #dash is visible now — the map container can measure itself.
    pendingMapPoints = points;
    pendingMapSettings = settings;
    maybeInitMap();
  } catch (e) {
    // Outside Tauri or backend unreachable: show the empty-state guidance
    // instead of an alert storm.
    try {
      renderEmptyState("connect");
    } catch {
      fail(e);
    }
  }
}

main();
