// Dedicated Achievements view: streaks (current, best, 12-week heatmap) and
// the full badge wall with progress bars. Data is the whole synced history,
// so the dashboard's range toggle does not apply here.

import { tauri, el, fail, inTauri } from "./lib/ipc";
import { icons } from "./lib/icons";
import { fetchFlowState, gamificationStats } from "./lib/flow";
import type { GamificationStats } from "./lib/flow";
import { initSyncIndicator } from "./lib/sync";
import { requirePlanOrRedirect } from "./lib/navgate";
// Side-effect import: paints the saved colour scheme before first paint.
import "./lib/theme";
import "./lib/navigation";

function reducedMotion(): boolean {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

// Entrance motion belongs to the first paint, not to every refresh. A sync can
// complete while this page is open, and re-counting every figure from zero (or
// replaying the whole badge wall's reveal) would read as the data changing when
// only one number moved. So the count-up and the scroll-reveal run once; later
// renders show stable final values.
let hasRendered = false;
let badgeObserver: IntersectionObserver | null = null;

/// Count a stat value up from zero when it mounts. The markup already holds
/// the final number, so with reduced motion (or no rAF) nothing changes —
/// the animation is pure garnish, never the source of truth. Each tick checks
/// the node is still in the document, so a refresh that replaces the DOM
/// mid-count stops the loop instead of writing to a detached element.
function countUp(container: HTMLElement): void {
  if (reducedMotion()) return;
  container.querySelectorAll<HTMLElement>("[data-to]").forEach((node, index) => {
    const to = Number(node.dataset.to);
    if (!isFinite(to)) return;
    const decimals = Number(node.dataset.decimals ?? 0);
    const suffix = node.dataset.suffix ?? "";
    const duration = 700;
    const startAt = performance.now() + index * 60; // follow the card stagger
    const tick = (now: number) => {
      if (!node.isConnected) return; // replaced by a refresh — stop cleanly
      const t = Math.min(1, (now - startAt) / duration);
      if (t > 0) {
        const eased = 1 - Math.pow(1 - t, 3); // ease-out cubic, settles soft
        node.textContent = (to * eased).toFixed(decimals) + suffix;
      }
      if (t < 1) requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  });
}

/// The value slot of a stat card: final text rendered eagerly, with the
/// parts countUp needs as data attributes.
function statValue(to: number, decimals = 0, suffix = ""): string {
  return `<span class="stat-value" data-to="${to}" data-decimals="${decimals}" data-suffix="${suffix}">${to.toFixed(decimals) + suffix}</span>`;
}

function statCard(icon: string, value: string, label: string): string {
  return `<div class="card stat-card">
    <span class="stat-icon" aria-hidden="true">${icon}</span>
    ${value}
    <span class="stat-label">${label}</span>
  </div>`;
}

function renderStreaks(s: GamificationStats): void {
  // The window the heatmap below actually covers, stated in the header slot
  // every other panel uses — so the grid's date range is never a guess.
  const active = s.recent.filter((d) => d.km > 0 || d.cross).length;
  el("streak-meta").textContent = `${active} active of the last ${s.recent.length} days`;
  el("streak-stats").innerHTML = [
    `<div class="card stat-card streak-current">
      <span class="streak-flame" aria-hidden="true">${icons.flame}</span>
      ${statValue(s.streak)}
      <span class="stat-label">day streak right now</span>
    </div>`,
    statCard(icons.trophy, statValue(s.bestStreak), "day best streak"),
    // "all-time distance" would be a lie next to a cross-training card — the
    // figure is running only, so the label says running.
    statCard(icons.run, statValue(s.totalKm, 0, " km"), "all-time running"),
    statCard(icons.stack, statValue(s.sessions), "sessions synced"),
    // Sessions and hours count every sport: a bike day is a session, and the
    // streak it keeps alive is an activity streak. Distance may not join them.
    statCard(icons.watch, statValue(s.totalHours, 0, " h"), "time training"),
    s.crossSessions > 0
      ? statCard(icons.bike, statValue(s.crossHours, 0, " h"), "cross-training")
      : "",
  ].join("");
  if (!hasRendered) countUp(el("streak-stats"));
}

// GitHub-style contribution heatmap: one column per week, rows = weekdays
// (Mon…Sun). Cells are shaded by the distance logged that day.
function renderHeatmap(s: GamificationStats): void {
  const days = s.recent;
  if (days.length === 0) {
    el("streak-heatmap").innerHTML = "";
    return;
  }
  const maxKm = Math.max(...days.map((d) => d.km), 1);
  // Shade in four steps so light days are still visible.
  const level = (km: number): number =>
    km <= 0 ? 0 : km >= maxKm * 0.66 ? 4 : km >= maxKm * 0.33 ? 3 : km >= maxKm * 0.1 ? 2 : 1;
  // Pad the start so the first column begins on Monday.
  const lead = (new Date(`${days[0].date}T00:00:00Z`).getUTCDay() + 6) % 7;
  const cells: string[] = [];
  for (let i = 0; i < lead; i++) cells.push(`<span class="hm-cell empty"></span>`);
  days.forEach((d, i) => {
    // A day with no running kilometres is only a rest day if nothing else
    // happened. Shade by running distance (the ramp means running) and mark
    // the days trained another way, so "I cycled" never reads as "I skipped".
    const title = `${d.date}: ${
      d.km > 0 ? `${d.km.toFixed(1)} km` : d.cross ? "cross-training" : "rest day"
    }${d.km > 0 && d.cross ? " + cross-training" : ""}`;
    // --i drives a per-week-column fade-in: the heatmap wipes left to right.
    const col = Math.floor((lead + i) / 7);
    cells.push(
      `<span class="hm-cell lv${level(d.km)}${d.cross ? " cross" : ""}" style="--i:${col}" title="${title}" aria-label="${title}"></span>`,
    );
  });
  const rows = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
  const grid = `<div class="hm-labels">${rows.map((r) => `<span>${r}</span>`).join("")}</div>
    <div class="hm-grid">${cells.join("")}</div>`;
  el("streak-heatmap").innerHTML = grid;
}

function badgeCard(a: GamificationStats["achievements"][number]): string {
  const pct =
    a.target != null && a.target > 0
      ? Math.min(100, ((a.progress ?? 0) / a.target) * 100)
      : a.earned
        ? 100
        : 0;
  // The fill fraction is the shared .meter atom's --value, so the grow-in
  // animation runs on the compositor and lands exactly on the number.
  // Whole numbers for county units, one decimal only where fractions matter (km).
  const decimals = a.unit === "km" ? 1 : 0;
  const progress =
    a.target != null
      ? `<span class="meter sm" role="progressbar" aria-valuenow="${Math.round(pct)}"
             aria-valuemin="0" aria-valuemax="100" style="--value:${(pct / 100).toFixed(3)}"><i></i></span>
         <span class="meter-label">${(a.progress ?? 0).toFixed(decimals)} / ${a.target} ${a.unit}</span>`
      : "";
  // A badge that is nearly there earns a stronger visual pull than the rest
  // of the locked wall — hint the user toward the next win.
  const almost = !a.earned && pct >= 80;
  return `<div class="badge-card ${a.earned ? "earned" : "locked"}${almost ? " almost" : ""}">
    <span class="badge-medal medallion${a.earned ? "" : " muted"}" aria-hidden="true">${a.earned ? a.icon : icons.lock}</span>
    <span class="badge-body">
      <span class="badge-name">${a.name}</span><br />
      <span class="badge-desc">${a.desc}</span>
      ${progress}
    </span>
  </div>`;
}

function renderBadges(s: GamificationStats): void {
  const earned = s.achievements.filter((a) => a.earned).length;
  el("badge-summary").innerHTML = `<b>${earned}</b> of ${s.achievements.length} earned — keep going.`;
  const grid = el("badge-cards");
  // With a hundred-plus badges the wall is long: earned badges lead, then
  // the closest-to-earning, so the next wins are above the fold. Stable
  // sort keeps the catalog order within each tier.
  const pctOf = (a: GamificationStats["achievements"][number]): number =>
    a.target != null && a.target > 0 ? Math.min(100, ((a.progress ?? 0) / a.target) * 100) : a.earned ? 100 : 0;
  const ordered = [...s.achievements].sort((x, y) => {
    const tier = (a: GamificationStats["achievements"][number]) => (a.earned ? 0 : pctOf(a) >= 80 ? 1 : 2);
    return tier(x) - tier(y) || pctOf(y) - pctOf(x);
  });
  grid.innerHTML = ordered.map(badgeCard).join("");
  // The badge wall can run below the fold; reveal cards as they enter the
  // viewport instead of animating everything at page load. Progressive
  // enhancement: without IO (or with reduced motion) cards are simply there.
  // Only the first render reveals — a later refresh replaces the wall with
  // final cards already in place, so a mid-session sync never replays the
  // whole entrance. The previous observer is disconnected so it can't leak.
  badgeObserver?.disconnect();
  badgeObserver = null;
  if (hasRendered) return;
  if ("IntersectionObserver" in window && !reducedMotion()) {
    grid.classList.add("scroll-reveal");
    const io = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) continue;
          entry.target.classList.add("in");
          io.unobserve(entry.target);
        }
      },
      { rootMargin: "0px 0px -5% 0px" },
    );
    badgeObserver = io;
    grid.querySelectorAll(".badge-card").forEach((card) => io.observe(card));
  }
}

async function refresh(): Promise<void> {
  try {
    const [flow, activities] = await Promise.all([
      fetchFlowState(),
      tauri.allActivities().catch(() => []),
    ]);
    const stats = gamificationStats(activities, flow.plan, flow.pushed);
    renderStreaks(stats);
    renderHeatmap(stats);
    renderBadges(stats);
    // Only after the first full render: entrance motion is a first-paint
    // affordance, so a later sync-completed refresh shows final values.
    hasRendered = true;
  } catch (e) {
    if (!inTauri) {
      el("streak-stats").innerHTML =
        '<p class="muted">Streaks and badges appear once your data is synced from the desktop app.</p>';
      return;
    }
    fail(e);
  }
}

async function main(): Promise<void> {
  initSyncIndicator();
  window.addEventListener("runalytics:sync-completed", () => {
    void refresh();
  });
  await refresh();
}

// Badges are earned against a plan's weeks, so the page waits for one too.
async function boot(): Promise<void> {
  if (await requirePlanOrRedirect("gated")) return;
  await main();
}

void boot();
