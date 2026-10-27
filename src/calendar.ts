import { tauri, el, fail, inTauri, fmtPace } from "./lib/ipc";
import { initSyncIndicator } from "./lib/sync";
import { requirePlanOrRedirect } from "./lib/navgate";
// Side-effect import: paints the saved colour scheme before first paint.
import "./lib/theme";
import "./lib/navigation";
import { hydrateIcons } from "./lib/icons";
import { animateSlideIn } from "./lib/motion";
import { escapeHtml, openCalendarEvent, adjustedLabel } from "./lib/workout-detail";
import { moveSession, openResetWeekSheet } from "./lib/schedule";
import type { CalendarEvent } from "./lib/types";

let cursor = new Date();
cursor.setDate(1);
let monthEvents: CalendarEvent[] = [];
let renderVersion = 0;
/// Race day of the plan on screen, as YYYY-MM-DD. `null` while no plan is
/// loaded, which is the fail-closed state: with no plan there is nothing that
/// could be rescheduled.
let planEnd: string | null = null;
/// Mondays of the plan's recovery weeks, as YYYY-MM-DD. Read once at boot: the
/// cycle is a property of the generated schedule, so it cannot change without
/// the plan changing. Empty until the read lands, which is the honest state —
/// an unlabelled dip beats a wrong label.
let downWeeks = new Set<string>();

const KIND_LABEL: Record<string, string> = {
  easy: "Easy",
  tempo: "Tempo",
  intervals: "Intervals",
  longRun: "Long",
  race: "Race",
  rest: "Rest",
  completed: "Done",
};

function ymd(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

function todayKey(): string {
  return ymd(new Date());
}

/// Monday of the week containing `d`, at local midnight. The grid runs Monday
/// first, so two days are in the same row exactly when this agrees for both.
function mondayOf(d: Date): Date {
  const monday = new Date(d.getFullYear(), d.getMonth(), d.getDate());
  monday.setDate(monday.getDate() - ((monday.getDay() + 6) % 7));
  return monday;
}

/// Every day from `from` to `to`, both ends included, at local midnight.
function* eachDay(from: Date, to: Date): Generator<Date> {
  for (let day = new Date(from); day <= to; day.setDate(day.getDate() + 1)) {
    yield new Date(day.getFullYear(), day.getMonth(), day.getDate());
  }
}

/// First and last day (YYYY-MM-DD) a session may be moved to: from today to
/// race day. The week a session was generated into is a suggestion, and the
/// reason to drag one at all is that real life pushed it out of its slot — so
/// the only walls are the past and the end of the plan.
function isDroppableDate(date: string): boolean {
  return planEnd !== null && date >= todayKey() && date <= planEnd;
}

/// Only planned sessions still to run can move; completed runs are history
/// and race day is fixed. A session whose day already slipped past still
/// moves — dragging it forward is exactly how a missed week is recovered.
function isMovable(w: CalendarEvent): boolean {
  return !!w.workoutId && w.activityId == null && w.kind !== "race";
}

/// The classes a session's chip (grid) or row (agenda) carries beyond its
/// kind. One function for both views so a skipped or moved session can never
/// be marked in one layout and plain in the other — they are rendered from the
/// same events, and a marker that appears in only one of them is a bug report
/// waiting to happen.
function eventStateClasses(w: CalendarEvent): string {
  const classes: string[] = [];
  // A matched run is history: it happened on the day it happened, so a
  // `moved` flag on the session behind it is not a fact about this row.
  if (w.moved && w.activityId == null) classes.push("moved");
  if (w.status === "skipped") classes.push("chip-skipped");
  else if (w.missed) classes.push("chip-missed");
  return classes.length ? ` ${classes.join(" ")}` : "";
}

/// What a session's tooltip says. A moved session is asked about first —
/// "where was it before?" — and a session the plan softened needs its reason
/// in the same place, because the chip's own label is the softened kind and a
/// value with no explanation reads as a bug. A session that is both gets both.
/// A session with neither is simply draggable, which is the hint the grid has
/// always given.
function eventTitle(w: CalendarEvent): string | null {
  const parts: string[] = [];
  if (w.moved && w.activityId == null) {
    parts.push(w.plannedDate ? `Moved from ${fromYmdLabel(w.plannedDate)}` : "Moved from its planned day");
  }
  if (w.adjusted) parts.push(adjustedLabel(w.adjusted));
  if (isMovable(w)) parts.push(parts.length ? "drag to a new day" : "Drag to a new day");
  return parts.length ? parts.join(" · ") : null;
}

/// A `YYYY-MM-DD` rendered as a short local date. Parsed at noon so a
/// west-of-UTC reader does not see the previous day.
function fromYmdLabel(value: string): string {
  return new Date(`${value}T12:00:00`).toLocaleDateString(undefined, {
    weekday: "short",
    day: "numeric",
    month: "short",
  });
}

async function renderMonth(dir: "next" | "back" | "none" = "none"): Promise<void> {
  const version = ++renderVersion;
  const year = cursor.getFullYear();
  const month = cursor.getMonth();
  el("cal-title").textContent = cursor.toLocaleDateString(undefined, {
    month: "long",
    year: "numeric",
  });

  const first = new Date(year, month, 1);
  const last = new Date(year, month + 1, 0);
  // The grid is laid out in whole weeks: the first row starts on the Monday of
  // the week the 1st falls in, the last row ends on the Sunday of the week the
  // month ends in. Those neighbour-month days are real days rather than blank
  // gaps, so their sessions are fetched too — a day you can see must never
  // look empty when something is scheduled on it.
  const lead = (first.getDay() + 6) % 7;
  const trail = (6 - ((last.getDay() + 6) % 7)) % 7;
  const gridStart = new Date(year, month, 1 - lead);
  const gridEnd = new Date(year, month, last.getDate() + trail);
  const [workouts, painPairs] = await Promise.all([
    tauri.calendarEvents(ymd(gridStart), ymd(gridEnd)),
    tauri.painDays(ymd(gridStart), ymd(gridEnd)).catch(() => [] as [string, number][]),
  ]);
  if (version !== renderVersion) return;
  monthEvents = workouts;
  renderAdjusted(workouts);
  const painByDay = new Map(painPairs);
  const byDay = new Map<string, CalendarEvent[]>();
  for (const w of workouts) {
    (byDay.get(w.date) ?? byDay.set(w.date, []).get(w.date)!).push(w);
  }

  const cells: string[] = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"].map(
    (d) => `<div class="cal-dow">${d}</div>`,
  );
  const today = todayKey();
  // The week the runner is in right now, matched by its Monday. Computed once
  // so a month viewed from either side highlights the same row, and the row
  // still highlights when part of it belongs to the month either side.
  const thisWeek = ymd(mondayOf(new Date()));
  for (const day of eachDay(gridStart, gridEnd)) {
    const key = ymd(day);
    const outside = day.getMonth() !== month;
    const dayEvents = byDay.get(key) ?? [];
    const classes = [
      "cal-cell",
      outside ? "cal-outside" : "",
      ymd(mondayOf(day)) === thisWeek ? "cal-this-week" : "",
      isDroppableDate(key) ? "cal-droppable" : "",
      key === today ? "cal-today" : "",
      // A corner fold on any day holding a session the runner moved: "you
      // touched this" is the one fact the date itself cannot tell you.
      dayEvents.some((w) => w.moved && w.activityId == null) ? "moved-day" : "",
    ]
      .filter(Boolean)
      .join(" ");
    const ws = dayEvents;
    const chips = ws
      .map(
        (w) =>
          `<button type="button" class="chip workout-chip chip-${w.kind}${w.activityId != null ? " chip-done" : ""}${isMovable(w) ? " movable" : ""}${eventStateClasses(w)}" data-event-index="${workouts.indexOf(w)}"${
            (() => {
              const tip = eventTitle(w) ?? (isMovable(w) ? "Drag to a new day" : null);
              return tip ? ` title="${escapeHtml(tip)}"` : "";
            })()
          } aria-label="${escapeHtml(w.title)}">${KIND_LABEL[w.kind] ?? w.kind}${
            w.distanceM ? ` ${(w.distanceM / 1000).toFixed(1)}k` : ""
          }</button>`,
      )
      .join("");
    // A pain dot beside the day number: the journal says it hurt that day.
    // It sits on the number's own line rather than floating in the middle of
    // the cell, so it reads as a fact about the day and not as a stray mark.
    const pain = painByDay.get(key);
    const painDot = pain
      ? `<span class="cal-pain ${pain >= 7 ? "sev-high" : pain >= 5 ? "sev-mid" : "sev-low"}" title="Pain logged: ${pain}/10"></span>`
      : "";
    cells.push(
      `<div class="${classes}" data-date="${key}"><span class="cal-day"><span class="cal-day-num">${day.getDate()}</span>${painDot}</span><div class="cal-workouts">${chips}</div></div>`,
    );
  }
  el("cal-grid").innerHTML = cells.join("");

  // The agenda is the SAME events as the grid, regrouped into a list. Seven
  // columns of text crowd at narrow widths, so below the breakpoint CSS hides
  // the grid and shows this instead — both are always in the DOM from one
  // render, so the selected month can never disagree between the two layouts.
  renderAgenda(month, gridStart, gridEnd, byDay, painByDay);

  // A month change slides the surface in from the direction you moved — the
  // grid and agenda travel together as one surface. First paint doesn't slide.
  if (dir !== "none") {
    const grid = el("cal-grid");
    const agenda = el("cal-agenda");
    void animateSlideIn(grid, dir);
    void animateSlideIn(agenda, dir);
  }
}

/// A day-by-day list of the grid's sessions, for narrow windows. Days with
/// nothing on them are skipped — an agenda of 31 rows where 5 have a run is
/// noise; the grid already shows the empty days as empty cells. Covers exactly
/// the grid's weeks, so a session on a neighbour-month day is never visible in
/// one layout and missing from the other.
///
/// On a phone this is the whole calendar, so it carries what the grid's header
/// carries on a desktop: the week's ran/planned ratio, the recovery-week label,
/// and the reset control for a week that has drifted.
function renderAgenda(
  month: number,
  from: Date,
  to: Date,
  byDay: Map<string, CalendarEvent[]>,
  painByDay: Map<string, number>,
): void {
  const rows: string[] = [];
  const todayKey = ymd(new Date());
  const thisWeek = ymd(mondayOf(new Date()));
  // The week currently open, so its header is emitted once when the days
  // cross into it rather than on every day.
  let openWeek: string | null = null;
  for (const dayDate of eachDay(from, to)) {
    const key = ymd(dayDate);
    const ws = byDay.get(key) ?? [];
    if (ws.length === 0) continue;
    const outside = dayDate.getMonth() !== month;
    const week = ymd(mondayOf(dayDate));
    if (week !== openWeek) {
      openWeek = week;
      rows.push(weekHeader(week, byDay, week === thisWeek));
    }
    const classes = [
      "agenda-day",
      outside ? "agenda-outside" : "",
      week === thisWeek ? "agenda-this-week" : "",
      key === todayKey ? "agenda-today" : "",
    ]
      .filter(Boolean)
      .join(" ");
    const pain = painByDay.get(key);
    const painDot = pain
      ? `<span class="cal-pain ${pain >= 7 ? "sev-high" : pain >= 5 ? "sev-mid" : "sev-low"}" title="Pain logged: ${pain}/10"></span>`
      : "";
    const items = ws
      .map(
        (w) => `<li><button type="button" class="agenda-row chip-${w.kind}${w.activityId != null ? " chip-done" : ""}${eventStateClasses(w)}" data-event-index="${monthEvents.indexOf(w)}"${
          (() => {
            const tip = eventTitle(w);
            return tip ? ` title="${escapeHtml(tip)}"` : "";
          })()
        }>
          <span class="agenda-kind">${KIND_LABEL[w.kind] ?? w.kind}</span>
          <span class="agenda-title">${escapeHtml(w.title)}</span>
          <span class="agenda-meta">${agendaMeta(w)}</span>
        </button></li>`,
      )
      .join("");
    const weekday = dayDate.toLocaleDateString(undefined, { weekday: "short" });
    const monthLabel = outside
      ? ` ${dayDate.toLocaleDateString(undefined, { month: "short" })}`
      : "";
    rows.push(
      `<div class="${classes}"><h3 class="agenda-date"><span>${dayDate.getDate()}</span> ${weekday}${monthLabel}${painDot}</h3><ul class="agenda-list">${items}</ul></div>`,
    );
  }
  el("cal-agenda").innerHTML = rows.length
    ? rows.join("")
    : `<p class="muted">No sessions scheduled this month.</p>`;
}

/// The banner saying the plan changed a day by itself. It sits above the grid
/// rather than only in the chip tooltip because an adjustment nobody noticed
/// reads as a bug — the runner must see that a session was softened *and* why,
/// in the same screen that shows the softened session. Positive-framed: the
/// plan gave up one hard day to keep the block on schedule, which is a decision
/// in the runner's favour, not a fault.
function renderAdjusted(events: CalendarEvent[]): void {
  const banner = el("cal-adjusted");
  // A banked run carries the reason its session was softened, but the run
  // itself is history — re-announcing it under a completed run would read as
  // "this is still a change to consider".
  const softened = events.filter((e) => e.adjusted && e.activityId == null);
  if (softened.length === 0) {
    banner.hidden = true;
    banner.textContent = "";
    return;
  }
  // Several sessions can be softened by one cause (a bad week of sleep moves
  // every hard day in it), and repeating the same reason three times makes the
  // banner look like an error log rather than one decision.
  const reasons = [...new Set(softened.map((e) => e.adjusted as string))];
  const subject =
    softened.length === 1
      ? "1 session softened"
      : `${softened.length} sessions softened`;
  banner.textContent = `Adjusted for you — ${reasons.join("; ")}. ${subject} to keep the rest of the block on schedule.`;
  banner.hidden = false;
}

/// The small print on an agenda row: what happened to the session, then its
/// size. "Adjusted" is stated here rather than left to the tooltip because a
/// tooltip cannot be reached by touch, and on a phone this row *is* the
/// calendar — the grid it would otherwise hover over is hidden. The full
/// reason lives in the banner above and in the sheet this row opens.
function agendaMeta(w: CalendarEvent): string {
  const parts: string[] = [];
  if (w.status === "skipped") parts.push("Skipped");
  else if (w.missed) parts.push("Missed");
  if (w.adjusted) parts.push("Adjusted");
  if (w.distanceM) parts.push(`${(w.distanceM / 1000).toFixed(1)} km`);
  if (w.durationMin) parts.push(`${Math.round(w.durationMin)} min`);
  return parts.join(" · ");
}

/// The band that opens a week in the agenda: the week's dates, its ran/planned
/// ratio, and — when the plan generated it that way — the recovery-week label.
function weekHeader(weekMonday: string, byDay: Map<string, CalendarEvent[]>, current: boolean): string {
  const monday = new Date(`${weekMonday}T12:00:00`);
  const sunday = new Date(monday);
  sunday.setDate(monday.getDate() + 6);
  const events: CalendarEvent[] = [];
  for (const day of eachDay(monday, sunday)) {
    events.push(...(byDay.get(ymd(day)) ?? []));
  }
  const ratio = weekRatio(events);
  const badge = downWeeks.has(weekMonday)
    ? ` <span class="week-badge" title="Your plan cuts volume this week on purpose.">Recovery week</span>`
    : "";
  // The reset appears on a week that has drifted, and nowhere else — a clean
  // week has nothing to put back, and a control that does nothing is noise.
  const drifted = events.some((e) => e.moved && e.activityId == null);
  const reset = drifted
    ? `<button type="button" class="secondary agenda-week-reset" data-reset-week="${weekMonday}" title="Put this week's sessions back on the days your plan generated">Reset</button>`
    : "";
  const range = `${monday.toLocaleDateString(undefined, { day: "numeric", month: "short" })} – ${sunday.toLocaleDateString(undefined, { day: "numeric", month: "short" })}`;
  return `<div class="agenda-week${current ? " agenda-this-week" : ""}"><h3 class="agenda-week-head" title="${escapeHtml(range)}">${escapeHtml(range)}${badge}<span class="agenda-week-ratio">${ratio.ran} / ${ratio.planned} km</span>${reset}</h3></div>`;
}

/// The week's ran-vs-planned mileage as one pair of numbers. One read rather
/// than two tiles: two numbers make the reader do the division, and the ratio
/// is the thing being asked about.
function weekRatio(events: CalendarEvent[]): { ran: string; planned: string } {
  const ranM = events
    .filter((e) => e.activityId != null)
    .reduce((s, e) => s + (e.distanceM ?? 0), 0);
  // Planned is the whole week's plan — banked sessions included — so the ratio
  // reads as progress against a fixed target rather than a shrinking one.
  const planM = events.reduce((s, e) => s + (e.distanceM ?? 0), 0);
  return { ran: (ranM / 1000).toFixed(1), planned: (planM / 1000).toFixed(1) };
}

/// The week's numbers above the grid, pinned to the REAL current week no
/// matter which month is on screen: what ran, what was planned, the average
/// pace of the running, and when the feed last went out. The events come from
/// the same calendar feed the grid renders, so a run matched minutes ago moves
/// these tiles the same way it moves a chip.
async function renderWeekStats(): Promise<void> {
  if (!inTauri) return;
  const monday = mondayOf(new Date());
  const sunday = new Date(monday);
  sunday.setDate(monday.getDate() + 6);
  const weekMonday = ymd(monday);
  const [events, publishing, deviations] = await Promise.all([
    tauri.calendarEvents(ymd(monday), ymd(sunday)),
    // Read-only config load; a missing/broken config only costs the stamp.
    tauri.calendarPublishing().catch(() => null),
    // A failed read just hides the reset control; the agenda's own buttons and
    // the backend remain the real paths.
    tauri.weekDeviations(weekMonday).catch(() => []),
  ]);
  const ran = events.filter((e) => e.activityId != null);
  const ranM = ran.reduce((s, e) => s + (e.distanceM ?? 0), 0);
  // Planned is the whole week's plan — banked sessions included — so the ratio
  // reads as progress against a fixed target, like the dashboard tiles.
  const planM = events.reduce((s, e) => s + (e.distanceM ?? 0), 0);
  // Pace weighted by distance, not an average of per-run paces: two 5 km runs
  // and one 16 km long run did not happen equally.
  const ranMin = ran.reduce((s, e) => s + (e.durationMin ?? 0), 0);
  const avgPace = ranM > 0 && ranMin > 0 ? (ranMin * 60) / (ranM / 1000) : null;
  const range = `${monday.toLocaleDateString(undefined, { month: "short", day: "numeric" })} – ${sunday.toLocaleDateString(undefined, { month: "short", day: "numeric" })}`;
  const published = publishing
    ? publishing.lastPublished
      ? // The stored stamp is UTC; toLocaleString renders it in local time.
        new Date(publishing.lastPublished).toLocaleString()
      : publishing.enabled
        ? "not yet published"
        : "publishing off"
    : "—";
  // One ratio rather than two numbers: side by side, "30.0 km ran" and
  // "38.0 km planned" ask the reader to do the division the tile exists to
  // show. The range moves into the label, where the date pair it describes is
  // already the subject.
  const badge = downWeeks.has(weekMonday)
    ? ` <span class="week-badge" title="Your plan cuts volume this week on purpose.">Recovery week</span>`
    : "";
  // The reset control appears only once the week has drifted, which is the
  // same condition the agenda's per-week button uses.
  const reset = deviations.length
    ? `<div class="cal-week-actions"><button type="button" id="cal-reset" class="secondary" data-reset-week="${weekMonday}">Reset week</button></div>`
    : "";
  el("cal-week").innerHTML = `<div class="tile-grid">${[
    `<div class="tile center" title="${escapeHtml(range)}"><b>${(ranM / 1000).toFixed(1)} / ${(planM / 1000).toFixed(1)} km</b><span>ran / planned${badge}</span></div>`,
    `<div class="tile center" title="${escapeHtml(range)}"><b>${avgPace != null ? `${fmtPace(avgPace)}/km` : "—"}</b><span>avg pace</span></div>`,
    `<div class="tile center pub"><b>${escapeHtml(published)}</b><span>last published</span></div>`,
  ].join("")}</div>${reset}`;
}

/// When a pointer-drag released, in performance.now terms. The click that
/// the browser fires after pointerup must not open the session detail.
let dragEndedAt = 0;

/// Drag-to-reschedule on the month grid.
///
/// This is deliberately pointer-driven rather than HTML5 drag-and-drop: the
/// desktop app renders in WKWebView, where `draggable="true"` on a button
/// never starts a drag session, so the chips looked draggable and silently
/// were not. A pointerdown/move/up gesture works identically in WebKit,
/// Chromium, and Firefox, and the same pattern already drives the settings
/// drawer in lib/overlay.ts.
function initDragAndDrop(): void {
  const grid = el("cal-grid");
  let dragged: CalendarEvent | null = null;
  let chip: HTMLElement | null = null;
  let ghost: HTMLElement | null = null;
  let activeId = -1;
  let startX = 0;
  let startY = 0;
  let moving = false;

  const clearTargets = () =>
    grid.querySelectorAll(".drop-target").forEach((cell) => cell.classList.remove("drop-target"));

  const cellAt = (x: number, y: number): HTMLElement | null => {
    const under = document.elementFromPoint(x, y)?.closest<HTMLElement>(".cal-cell[data-date]");
    return under && dragged && isDroppableDate(under.dataset.date!) ? under : null;
  };

  const reset = () => {
    dragged = null;
    chip = null;
    activeId = -1;
    moving = false;
    clearTargets();
    grid.classList.remove("is-dragging");
    grid.querySelectorAll(".dragging").forEach((node) => node.classList.remove("dragging"));
    ghost?.remove();
    ghost = null;
  };

  grid.addEventListener("pointerdown", (event) => {
    // Touch keeps its native behaviour: below the width breakpoint the grid
    // is replaced by the agenda, and hijacking touch here would break
    // scrolling the page on narrow windows.
    if (event.pointerType === "touch" || event.button !== 0) return;
    const target = (event.target as Element).closest<HTMLElement>(".workout-chip.movable");
    const selected = target ? monthEvents[Number(target.dataset.eventIndex)] : null;
    if (!target || !selected) return;
    dragged = selected;
    chip = target;
    activeId = event.pointerId;
    startX = event.clientX;
    startY = event.clientY;
    moving = false;
  });

  // Move and release are watched on the window, not the grid: once the pointer
  // travels past the grid's edge the grid stops seeing events, and a drag that
  // ends there would otherwise never be cleaned up. Pointer capture is
  // deliberately not used — it retargets the click that follows, which would
  // stop a plain click on a chip from opening the session.
  window.addEventListener("pointermove", (event) => {
    if (!dragged || !chip || event.pointerId !== activeId) return;
    if (!moving) {
      // Small jitter before the threshold stays a click, not a move.
      if (Math.abs(event.clientX - startX) < 5 && Math.abs(event.clientY - startY) < 5) return;
      moving = true;
      chip.classList.add("dragging");
      grid.classList.add("is-dragging");
      ghost = chip.cloneNode(true) as HTMLElement;
      ghost.classList.remove("dragging");
      ghost.classList.add("chip-ghost");
      ghost.style.width = `${chip.getBoundingClientRect().width}px`;
      document.body.appendChild(ghost);
    }
    event.preventDefault();
    ghost?.style.setProperty("transform", `translate(${event.clientX + 10}px, ${event.clientY + 10}px)`);
    const cell = cellAt(event.clientX, event.clientY);
    if (cell?.classList.contains("drop-target")) return;
    clearTargets();
    cell?.classList.add("drop-target");
  });

  const finish = (event: PointerEvent) => {
    if (!dragged || event.pointerId !== activeId) return;
    const workout = dragged;
    const cell = moving ? cellAt(event.clientX, event.clientY) : null;
    if (moving) {
      dragEndedAt = performance.now();
      const date = cell?.dataset.date;
      if (date && date !== workout.date) {
        // moveSession offers the way back: a mis-drag is a two-tap fix rather
        // than a hunt for the day the run used to be on.
        void moveSession(workout.workoutId!, date, workout.date).catch(fail);
      }
    }
    reset();
  };

  window.addEventListener("pointerup", finish);
  window.addEventListener("pointercancel", () => reset());
}

async function main(): Promise<void> {
  hydrateIcons();
  initSyncIndicator();
  initDragAndDrop();
  // One delegated listener covers both views: the grid's chips and the
  // agenda's rows carry the same data-event-index into the same array.
  el("calendar").addEventListener("click", (event) => {
    // A pointer drag ends with a click on the chip it moved. That gesture was
    // a reschedule, not a request to open the session.
    if (performance.now() - dragEndedAt < 250) return;
    // The week's reset control is a button too, and it is not a session —
    // checked first so it can never fall through and open a detail sheet.
    const reset = (event.target as Element).closest<HTMLButtonElement>("[data-reset-week]");
    if (reset) {
      openResetWeekSheet(reset.dataset.resetWeek!);
      return;
    }
    const button = (event.target as Element).closest<HTMLButtonElement>("[data-event-index]");
    const selected = button ? monthEvents[Number(button.dataset.eventIndex)] : null;
    if (selected) void openCalendarEvent(selected).catch(fail);
  });
  for (const event of ["runalytics:sync-completed", "runalytics:calendar-updated"]) {
    window.addEventListener(event, () => {
      // A sync or a reschedule moves both the month and the week's numbers —
      // and a republish moves the stamp in the stats strip.
      void Promise.all([renderMonth(), renderWeekStats()]).catch(fail);
    });
  }
  try {
    await Promise.all([renderMonth(), renderWeekStats()]);
  } catch (e) {
    if (!inTauri) {
      const message = `<div class="empty-state"><h3>Open your training calendar on your Mac</h3><p>Sessions appear here when Runalytics is connected to your watch in the desktop app.</p></div>`;
      el("cal-grid").innerHTML = message;
      el("cal-agenda").innerHTML = message;
      el<HTMLButtonElement>("cal-prev").disabled = true;
      el<HTMLButtonElement>("cal-next").disabled = true;
    } else {
      fail(e);
    }
  }

  el("cal-prev").addEventListener("click", () => {
    cursor.setMonth(cursor.getMonth() - 1);
    renderMonth("back").catch(fail);
  });
  el("cal-next").addEventListener("click", () => {
    cursor.setMonth(cursor.getMonth() + 1);
    renderMonth("next").catch(fail);
  });
}

// The calendar is the plan laid over the month — with no plan there are no
// scheduled sessions to show, so it redirects rather than rendering an empty grid.
async function boot(): Promise<void> {
  if (await requirePlanOrRedirect("gated")) return;
  // Race day bounds how far a session may be dragged, and the recovery weeks
  // are read from the same plan. The gate above already proved a plan exists;
  // if a read fails the marker is simply absent and the backend still refuses
  // anything it disagrees with.
  const [plan, down] = await Promise.all([
    tauri.currentPlan().catch(() => null),
    tauri.downWeeks().catch(() => [] as string[]),
  ]);
  planEnd = plan?.[0].targetDate ?? null;
  downWeeks = new Set(down);
  await main();
}

void boot();
