// Dedicated Achievements view: streaks (current, best, 12-week heatmap) and
// the full badge wall with progress bars. Data is the whole synced history,
// so the dashboard's range toggle does not apply here.

import { tauri, el, fail, inTauri, fmtDateTime, fmtKm, fmtDuration, fmtPace } from "./lib/ipc";
import type { Activity, CalendarEvent } from "./lib/types";
import { isRun } from "./lib/sport";
import { performanceLayout } from "./lib/performance-layout";
import { routeThumbnail } from "./lib/route-thumbnail";
import { icons } from "./lib/icons";
import { fetchFlowState, gamificationStats } from "./lib/flow";
import type { GamificationStats } from "./lib/flow";
import { initSyncIndicator } from "./lib/sync";
import { initCarousels } from "./lib/carousel";
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
let history: Activity[] = [];
let performanceStarted = false;
let routeObserver: IntersectionObserver | null = null;
const routeCache = new Map<number, string>();
let historyEvents = new Map<number, CalendarEvent>();

function renderHistory(): void {
  const type = el<HTMLSelectElement>("history-type").value;
  const year = el<HTMLSelectElement>("history-year").value;
  const month = el<HTMLSelectElement>("history-month").value;
  const query = el<HTMLInputElement>("history-search").value.trim().toLocaleLowerCase();
  const terms = query.split(/\s+/).filter(Boolean);
  const filtered = query.length > 0 || type !== "all" || year !== "all" || month !== "all";
  el("history-reset").hidden = !filtered;
  const selected = history.filter((activity) => {
    const date = new Date(activity.startTime);
    const linked = historyEvents.get(activity.id);
    const matchesType = type === "all" || (type === "run" ? isRun(activity)
      : type === "cross" ? !isRun(activity)
      : type === "imported" ? !linked?.workoutId
      : linked?.workoutId != null && linked.kind === type);
    return matchesType
      && (year === "all" || date.getFullYear() === Number(year))
      && (month === "all" || date.getMonth() === Number(month))
      && terms.every(term => `${linked?.title ?? ""} ${activity.sport} ${activity.provider}`.toLocaleLowerCase().includes(term));
  });
  const distance = selected.filter(isRun).reduce((total, activity) => total + activity.distanceM, 0);
  const duration = selected.reduce((total, activity) => total + activity.durationS, 0);
  el("history-summary").textContent = `${selected.length} activities · ${fmtKm(distance)} running · ${fmtDuration(duration)}`;
  const list = el("history-list");
  routeObserver?.disconnect();
  routeObserver = new IntersectionObserver((entries) => {
    for (const entry of entries) {
      if (!entry.isIntersecting) continue;
      routeObserver?.unobserve(entry.target);
      const row = entry.target as HTMLElement;
      const id = Number(row.dataset.activityId);
      void tauri.activityDetail(id, "1mo").then((detail) => {
        const thumbnail = routeThumbnail(detail.track);
        if (!thumbnail) return;
        routeCache.set(id, thumbnail);
        if (row.isConnected) row.querySelector(".activity-icon")!.innerHTML = thumbnail;
      }).catch(() => undefined);
    }
  }, { rootMargin: "100px" });
  list.replaceChildren();
  if (!selected.length) {
    list.innerHTML = filtered
      ? '<div class="empty-state"><h3>No matching workouts</h3><p>No activities match these filters.</p><button type="button" class="secondary" id="history-empty-reset">Clear filters</button></div>'
      : '<div class="empty-state"><h3>No workouts yet</h3><p>Your synced activities will appear here.</p><a href="settings.html#setup">Watch connections</a></div>';
    document.getElementById("history-empty-reset")?.addEventListener("click", clearHistoryFilters);
  }
  for (const activity of selected) {
    const row = document.createElement("button");
    row.type = "button";
    row.className = "activity-row";
    const linked = historyEvents.get(activity.id);
    if (linked?.workoutId) row.classList.add(`chip-${linked.kind}`);
    row.dataset.activityId = String(activity.id);
    row.innerHTML = `<span class="activity-icon" aria-hidden="true">${isRun(activity) ? icons.run : icons.dumbbell}</span><span><h3></h3><p></p><span class="activity-metrics"><span>${fmtKm(activity.distanceM)}<small>Distance</small></span><span>${fmtDuration(activity.durationS)}<small>Time</small></span><span>${fmtPace(activity.avgPaceSKm)}/km<small>Avg pace</small></span></span></span><span aria-hidden="true">${icons.chevronRight}</span>`;
    row.querySelector("h3")!.textContent = linked?.workoutId ? linked.title : activity.sport;
    row.querySelector("p")!.textContent = `${fmtDateTime(activity.startTime)} · ${activity.provider.toUpperCase()}`;
    row.addEventListener("click", async () => {
      const { openActivitySheet } = await import("./dashboard");
      await openActivitySheet(activity.id);
    });
    list.append(row);
    if (routeCache.has(activity.id)) row.querySelector(".activity-icon")!.innerHTML = routeCache.get(activity.id)!;
    else if (activity.hasTrack) routeObserver.observe(row);
  }
}

function clearHistoryFilters(): void {
  el<HTMLInputElement>("history-search").value = "";
  for (const id of ["history-type", "history-year", "history-month"]) el<HTMLSelectElement>(id).value = "all";
  renderHistory();
  el("history-search").focus();
}

function initProgressTabs(): void {
  el("performance-analysis").innerHTML = performanceLayout;
  for (const detail of document.querySelectorAll<HTMLDetailsElement>(".analysis-detail")) {
    detail.addEventListener("toggle", () => {
      if (detail.open) {
        window.dispatchEvent(new Event("resize"));
        initCarousels();
      }
    });
  }
  const tabs = [el<HTMLButtonElement>("workouts-tab"), el<HTMLButtonElement>("performance-tab"), el<HTMLButtonElement>("milestones-tab")];
  const select = async (index: number) => {
    tabs.forEach((tab, position) => {
      tab.setAttribute("aria-selected", String(position === index));
      tab.tabIndex = position === index ? 0 : -1;
    });
    el("progress-workouts").hidden = index !== 0;
    el("progress-performance").hidden = index !== 1;
    el("progress-milestones").hidden = index !== 2;
    if (index === 1) window.dispatchEvent(new Event("resize"));
    if (index === 1 && !performanceStarted) {
      performanceStarted = true;
      const { startPerformance } = await import("./dashboard");
      await startPerformance();
    }
  };
  tabs.forEach((tab, index) => {
    tab.addEventListener("click", () => void select(index));
    tab.addEventListener("keydown", (event) => {
      if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
      event.preventDefault();
      const next = (index + (event.key === "ArrowRight" ? 1 : -1) + tabs.length) % tabs.length;
      tabs[next].focus();
      void select(next);
    });
  });
  el("history-reset").innerHTML = icons.close;
  el("history-reset").addEventListener("click", clearHistoryFilters);
  el("history-search").addEventListener("input", renderHistory);
  for (const id of ["history-type", "history-year", "history-month"]) el(id).addEventListener("change", renderHistory);
  if (["#performance", "#recovery", "#risk", "#routes"].includes(location.hash)) {
    void select(1).then(() => {
      const detail = document.getElementById(location.hash.slice(1));
      if (detail instanceof HTMLDetailsElement) {
        detail.open = true;
        detail.scrollIntoView({ block: "start" });
      }
    });
  }
  else if (location.hash === "#milestones") void select(2);
}

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
      ? statCard(icons.dumbbell, statValue(s.crossHours, 0, " h"), "cross-training")
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
    history = activities.sort((left, right) => right.startTime.localeCompare(left.startTime));
    const events = await tauri.calendarEvents("1970-01-01", "2100-12-31").catch(() => [] as CalendarEvent[]);
    historyEvents = new Map(events.filter((event) => event.activityId != null).map((event) => [event.activityId!, event]));
    const years = [...new Set(history.map((activity) => new Date(activity.startTime).getFullYear()))].sort((left, right) => right - left);
    const yearSelect = el<HTMLSelectElement>("history-year");
    const selectedYear = yearSelect.value;
    yearSelect.replaceChildren(new Option("All years", "all"), ...years.map((year) => new Option(String(year), String(year))));
    yearSelect.value = years.includes(Number(selectedYear)) ? selectedYear : "all";
    renderHistory();
    renderStreaks(stats);
    renderHeatmap(stats);
    renderBadges(stats);
    // Only after the first full render: entrance motion is a first-paint
    // affordance, so a later sync-completed refresh shows final values.
    hasRendered = true;
  } catch (e) {
    if (!inTauri) {
      el("history-summary").textContent = "Connect your watch in the app to see your workouts.";
      el("history-list").innerHTML = '<a class="btn" href="settings.html#setup">Watch connections</a>';
      el("streak-stats").innerHTML =
        '<div class="empty-state"><h3>Your milestones live in the desktop app</h3><p>Open Runalytics on your Mac and sync your watch to see streaks, training history, and badges.</p></div>';
      el("streaks").querySelector<HTMLElement>(".heatmap-row")!.hidden = true;
      el("badges").hidden = true;
      return;
    }
    fail(e);
  }
}

async function main(): Promise<void> {
  initProgressTabs();
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
