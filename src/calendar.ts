import { tauri, el, fail, inTauri } from "./lib/ipc";
import { initSyncIndicator } from "./lib/sync";
import { requirePlanOrRedirect } from "./lib/navgate";
// Side-effect import: paints the saved colour scheme before first paint.
import "./lib/theme";
import "./lib/navigation";
import { hydrateIcons } from "./lib/icons";
import { animateSlideIn } from "./lib/motion";
import { escapeHtml, openCalendarEvent } from "./lib/workout-detail";
import type { CalendarEvent } from "./lib/types";

let cursor = new Date();
cursor.setDate(1);
let monthEvents: CalendarEvent[] = [];
let renderVersion = 0;
/// Race day of the plan on screen, as YYYY-MM-DD. `null` while no plan is
/// loaded, which is the fail-closed state: with no plan there is nothing that
/// could be rescheduled.
let planEnd: string | null = null;

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
    const classes = [
      "cal-cell",
      outside ? "cal-outside" : "",
      ymd(mondayOf(day)) === thisWeek ? "cal-this-week" : "",
      isDroppableDate(key) ? "cal-droppable" : "",
      key === today ? "cal-today" : "",
    ]
      .filter(Boolean)
      .join(" ");
    const ws = byDay.get(key) ?? [];
    const chips = ws
      .map(
        (w) =>
          `<button type="button" class="chip workout-chip chip-${w.kind}${w.activityId != null ? " chip-done" : ""}${isMovable(w) ? " movable" : ""}" data-event-index="${workouts.indexOf(w)}"${
            isMovable(w) ? ` title="Drag to a new day"` : ""
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
  for (const dayDate of eachDay(from, to)) {
    const key = ymd(dayDate);
    const ws = byDay.get(key) ?? [];
    if (ws.length === 0) continue;
    const outside = dayDate.getMonth() !== month;
    const classes = [
      "agenda-day",
      outside ? "agenda-outside" : "",
      ymd(mondayOf(dayDate)) === thisWeek ? "agenda-this-week" : "",
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
        (w) => `<li><button type="button" class="agenda-row chip-${w.kind}${w.activityId != null ? " chip-done" : ""}" data-event-index="${monthEvents.indexOf(w)}">
          <span class="agenda-kind">${KIND_LABEL[w.kind] ?? w.kind}</span>
          <span class="agenda-title">${escapeHtml(w.title)}</span>
          <span class="agenda-meta">${w.distanceM ? `${(w.distanceM / 1000).toFixed(1)} km` : ""}${w.distanceM && w.durationMin ? " · " : ""}${w.durationMin ? `${Math.round(w.durationMin)} min` : ""}</span>
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
        tauri
          .moveWorkout(workout.workoutId!, date)
          // Re-renders the grid; the Settings pane picks up the new publish time.
          .then(() => window.dispatchEvent(new CustomEvent("runalytics:calendar-updated")))
          .catch(fail);
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
    const button = (event.target as Element).closest<HTMLButtonElement>("[data-event-index]");
    const selected = button ? monthEvents[Number(button.dataset.eventIndex)] : null;
    if (selected) void openCalendarEvent(selected).catch(fail);
  });
  for (const event of ["runalytics:sync-completed", "runalytics:calendar-updated"]) {
    window.addEventListener(event, () => {
      void renderMonth().catch(fail);
    });
  }
  try {
    await renderMonth();
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
  // Race day bounds how far a session may be dragged. The gate above already
  // proved a plan exists; if the read fails the window stays closed and the
  // backend still refuses anything it disagrees with.
  planEnd = (await tauri.currentPlan().catch(() => null))?.[0].targetDate ?? null;
  await main();
}

void boot();
