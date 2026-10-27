/* The review of one recorded run: route, headline numbers, laps, splits.

   Two surfaces show this — the dashboard's activity sheet and the calendar's
   workout modal — and both must draw it identically, which is why the drawing
   lives here rather than in either page.

   The lap chart is the reason the whole thing exists. A quality session has
   always been judged by one number (the pace held over the main set), which is
   the right shape for adapting a plan and the wrong shape for reviewing a run:
   it cannot show that rep 3 was the one that went. Here each bar is a lap the
   prescription asked for, and the dashed box over it is the range it asked for,
   so "did I hit it" is read as bar-top-inside-box rather than from a sentence.

   Bar height is pace, taller being faster — a profile, not a magnitude, so
   there is no zero line and the axis is labelled as such. Only the work is
   graded: a warm-up, a cool-down, or a jog recovery carries no box, because a
   target that was never set should not be drawn as one. The plan does state
   easy paces, but they exist for the load maths rather than as a range the
   runner is asked to hit. */

import { fmtDuration, fmtKm, fmtPace, fmtPaceHeld } from "./ipc";
import { mountRouteMap } from "./route-map";
import { escapeHtml } from "./workout-detail";
import type { ActivityReview, LapSlice, Settings } from "./types";

/// What each verdict is called, in the same voice as the session verdict on the
/// Plan page ("in range", "too slow").
const VERDICT_LABEL: Record<string, string> = {
  ahead: "too fast",
  in: "in range",
  behind: "too slow",
};

/// The range a lap asked for, in the runner's terms. Only a work lap has one;
/// everything else returns an empty string, which is the chart's way of saying
/// nothing was asked for.
export function lapBandLabel(lap: LapSlice): string {
  if (lap.bandLoSKm == null) return "";
  if (lap.bandHiSKm == null) return `at ${fmtPace(lap.bandLoSKm)}/km`;
  return `at ${fmtPace(lap.bandLoSKm)}\u2013${fmtPace(lap.bandHiSKm)}/km`;
}

/// The pace axis: every lap pace and every band edge shares one scale, so a bar
/// and a box drawn in different columns are still comparable.
function paceDomain(laps: LapSlice[]): { min: number; max: number } {
  const values: number[] = [];
  for (const lap of laps) {
    if (lap.avgPaceSKm != null) values.push(lap.avgPaceSKm);
    if (lap.bandLoSKm != null) values.push(lap.bandLoSKm);
    if (lap.bandHiSKm != null) values.push(lap.bandHiSKm);
  }
  if (values.length === 0) return { min: 0, max: 1 };
  const min = Math.min(...values);
  const max = Math.max(...values);
  // A run of identical laps would put every bar at 0% or 100%; a floor of a
  // minute either side keeps the shape readable instead of saturating it.
  const pad = Math.max((max - min) * 0.18, 30);
  return { min: Math.max(1, min - pad), max: max + pad };
}

/// Height as a percentage, inverted: faster (fewer seconds per km) is taller.
function paceHeight(pace: number, domain: { min: number; max: number }): number {
  const span = domain.max - domain.min;
  if (span <= 0) return 50;
  return Math.min(100, Math.max(0, ((domain.max - pace) / span) * 100));
}

/// The laps chart: one column per prescribed lap, width proportional to the
/// distance that lap actually covered, so the shape of the session — a long
/// warm-up, four short reps, a long cool-down — is visible in the bars
/// themselves and not only in their labels.
export function renderLapChart(host: HTMLElement, review: ActivityReview): void {
  const laps = review.laps;
  if (laps.length === 0) {
    host.innerHTML =
      review.hasTrack && review.splits.length > 0
        ? `<p class="muted">The track under this run was too thin to cut into laps.</p>`
        : `<p class="muted">No GPS track synced for this run, so there are no laps to show. The numbers above come from the activity summary.</p>`;
    return;
  }
  const domain = paceDomain(laps);
  const columns = laps.map((l) => `${Math.max(l.meters, 1) / 100}fr`).join(" ");
  const bars = laps
    .map((lap) => {
      const fill =
        lap.avgPaceSKm == null
          ? `<div class="lap-none" title="No track under this lap"></div>`
          : `<div class="bar-fill" style="height:${paceHeight(lap.avgPaceSKm, domain).toFixed(1)}%"></div>`;
      // The box is the range, drawn where a pace inside it would end. Only the
      // work has a range, so only the work gets a box — everything else is an
      // unmarked bar, which is the chart saying "nothing was asked here".
      let band = "";
      if (lap.bandLoSKm != null && lap.bandHiSKm != null) {
        const top = paceHeight(lap.bandLoSKm, domain);
        const bottom = paceHeight(lap.bandHiSKm, domain);
        band = `<div class="lap-band" style="bottom:${bottom.toFixed(1)}%;height:${(top - bottom).toFixed(1)}%"></div>`;
      }
      const verdict = lap.verdict ? VERDICT_LABEL[lap.verdict] : "";
      // Only the work carries a band, so the parts are collected and joined
      // rather than concatenated: an unconditional separator would print
      // "Warm-up 2 km easy ·  · @ 6:12/km" for every ungraded lap.
      const tip = [
        lap.label,
        lapBandLabel(lap),
        lap.avgPaceSKm != null ? fmtPaceHeld(lap.avgPaceSKm) : "",
        verdict,
      ]
        .filter(Boolean)
        .join(" · ");
      return `<div class="bar-col lap-col kind-${lap.kind}${lap.verdict ? ` lap-${lap.verdict}` : ""}"
        title="${escapeHtml(tip)}">
        <div class="bar-track">${band}${fill}</div>
      </div>`;
    })
    .join("");
  // A recovery lap is a few hundred metres wide, so its caption would be
  // ellipsised to a stray digit — and its pace was never the question anyway.
  // Runna leaves these bars unlabelled for the same reason. The cell is still
  // emitted, because the axis row shares the column template and a missing
  // cell would slide every later caption out from under its bar.
  const labels = laps
    .map((lap) =>
      lap.kind === "recovery"
        ? `<div class="bar-label" aria-hidden="true"></div>`
        : `<div class="bar-label"><span class="bar-cap">${fmtPace(lap.avgPaceSKm)}</span><small>${escapeHtml(shortLabel(lap))}</small></div>`,
    )
    .join("");
  host.innerHTML = `<div class="lap-scroll"><div class="lap-inner" style="min-width:${laps.length * 44}px">
    <div class="bar-plot lap-plot" style="grid-template-columns:${columns}">${bars}</div>
    <div class="bar-axis" style="grid-template-columns:${columns}">${labels}</div>
  </div></div>
  <p class="lap-legend">${review.prescribed
    ? `<span class="lap-key"><span class="lap-key-box" aria-hidden="true"></span>prescribed range</span>`
    : `<span class="lap-key">per kilometre — no session was planned for this run</span>`}
    <span class="lap-key">taller is faster</span></p>
  ${lapVerdictSummary(laps)}`;
}

/// The sentence under the chart: which laps the plan graded, and where each
/// one landed. The chart shows the shape, this names the misses — "Rep 2 too
/// fast" — so the answer does not depend on reading bar-tops against boxes.
/// When every graded lap landed inside its range the sentence says that too:
/// silence after a good session reads as a missing feature, not as praise.
function lapVerdictSummary(laps: LapSlice[]): string {
  const graded = laps.filter((l) => l.verdict);
  if (graded.length === 0) return "";
  const misses = graded.filter((l) => l.verdict !== "in");
  if (misses.length === 0) {
    return `<p class="lap-verdict-summary lap-verdict-all-in">All ${graded.length} ${graded.length === 1 ? "lap" : "laps"} the plan graded landed in the prescribed range.</p>`;
  }
  const parts = misses.map((l) => `${l.kind === "kilometre" ? "Km" : "Lap"} ${l.index} ${VERDICT_LABEL[l.verdict ?? "in"]}`);
  return `<p class="lap-verdict-summary">${escapeHtml(parts.join(" · "))}.</p>`;
}

/// The axis caption has a few characters to work with, so it carries the lap
/// number; the lap's name stays in the tooltip. A per-kilometre fallback layout
/// says "km" instead, since there the number is a distance, not a rep count.
function shortLabel(lap: LapSlice): string {
  return lap.kind === "kilometre" ? `${lap.index} km` : `${lap.index}`;
}

/// Kilometre splits. The delta column is against the PREVIOUS kilometre — the
/// reference is printed in the header, because a bare "+/-" is read as
/// "against target", and on a surging run the two are opposite signs.
export function renderSplitsTable(host: HTMLElement, review: ActivityReview): void {
  if (review.splits.length === 0) {
    host.innerHTML = `<p class="muted">No kilometre splits without a GPS track.</p>`;
    return;
  }
  const rows = review.splits
    .map((s) => {
      const delta =
        s.deltaPrevSKm == null
          ? `<td class="muted">—</td>`
          : `<td class="${s.deltaPrevSKm > 0 ? "slower" : "faster"}">${s.deltaPrevSKm > 0 ? "+" : "\u2212"}${fmtPace(Math.abs(s.deltaPrevSKm))}</td>`;
      return `<tr>
        <td>${s.km}</td>
        <td>${fmtPace(s.avgPaceSKm)}</td>
        ${delta}
        <td>${s.avgHr ?? "—"}</td>
      </tr>`;
    })
    .join("");
  host.innerHTML = `<table class="splits-table">
    <thead><tr><th scope="col">km</th><th scope="col">avg pace</th>
      <th scope="col">vs previous km</th><th scope="col">hr</th></tr></thead>
    <tbody>${rows}</tbody></table>`;
}

function statTile(value: string, label: string): string {
  return `<div class="tile"><b>${value}</b><span>${label}</span></div>`;
}

/// Headline numbers for a run, from the activity summary — the part of a review
/// that exists even with no track.
export function renderRunStats(host: HTMLElement, review: ActivityReview): void {
  const a = review.activity;
  const tiles = [
    statTile(fmtKm(a.distanceM), "distance"),
    statTile(fmtDuration(a.durationS), "time"),
    statTile(`${fmtPace(a.avgPaceSKm)}/km`, "avg pace"),
    statTile(a.avgHr != null ? `${a.avgHr}` : "—", "avg hr"),
    statTile(a.maxHr != null ? `${a.maxHr}` : "—", "max hr"),
    statTile(
      a.elevationGainM != null ? `${Math.round(a.elevationGainM)} m` : "—",
      "ascent",
    ),
  ];
  host.innerHTML = tiles.join("");
}

/// The whole review into `host`: stats, route, laps, splits. Returns a teardown
/// that must run when the surface closes, or the map leaks its canvas and its
/// theme observer.
export function renderRunReview(
  host: HTMLElement,
  review: ActivityReview,
  settings: Settings,
): () => void {
  host.innerHTML = `
    <div class="tile-grid run-stats"></div>
    <div class="run-map"></div>
    <h3 class="run-review-head">Laps</h3>
    <div class="run-laps"></div>
    <h3 class="run-review-head">Splits</h3>
    <div class="run-splits"></div>`;
  renderRunStats(
    host.querySelector<HTMLElement>(".run-stats")!,
    review,
  );
  const mapEl = host.querySelector<HTMLElement>(".run-map")!;
  let map: ReturnType<typeof mountRouteMap> = null;
  if (review.track.length > 0) {
    map = mountRouteMap(mapEl, review.track, settings);
  } else {
    mapEl.innerHTML = `<div class="map-empty">No GPS track for this activity.</div>`;
  }
  renderLapChart(host.querySelector<HTMLElement>(".run-laps")!, review);
  renderSplitsTable(host.querySelector<HTMLElement>(".run-splits")!, review);
  return () => {
    map?.remove();
    map = null;
  };
}
