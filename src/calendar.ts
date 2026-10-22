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

/// First and last day (YYYY-MM-DD) of the week containing today, Monday-based.
function currentWeek(): [string, string] {
  const today = new Date();
  const monday = new Date(today.getFullYear(), today.getMonth(), today.getDate() - ((today.getDay() + 6) % 7));
  const sunday = new Date(monday.getFullYear(), monday.getMonth(), monday.getDate() + 6);
  return [ymd(monday), ymd(sunday)];
}

function inCurrentWeek(date: string): boolean {
  const [from, to] = currentWeek();
  return date >= from && date <= to;
}

/// Only planned sessions still to run can move; completed runs are history and race day is fixed.
function isMovable(w: CalendarEvent): boolean {
  return !!w.workoutId && w.activityId == null && w.kind !== "race" && inCurrentWeek(w.date);
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
  const [workouts, painPairs] = await Promise.all([
    tauri.calendarEvents(ymd(first), ymd(last)),
    tauri.painDays(ymd(first), ymd(last)).catch(() => [] as [string, number][]),
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
  const lead = (first.getDay() + 6) % 7;
  for (let i = 0; i < lead; i++) cells.push(`<div class="cal-cell empty"></div>`);
  const todayKey = ymd(new Date());
  for (let day = 1; day <= last.getDate(); day++) {
    const key = ymd(new Date(year, month, day));
    const ws = byDay.get(key) ?? [];
    const chips = ws
      .map(
        (w) =>
          `<button type="button" class="chip workout-chip chip-${w.kind}${w.activityId != null ? " chip-done" : ""}${isMovable(w) ? " movable" : ""}" data-event-index="${workouts.indexOf(w)}"${
            isMovable(w) ? ` draggable="true" title="Drag to another day this week"` : ""
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
      `<div class="cal-cell${inCurrentWeek(key) ? " cal-this-week" : ""}${key === todayKey ? " cal-today" : ""}" data-date="${key}"><span class="cal-day"><span class="cal-day-num">${day}</span>${painDot}</span><div class="cal-workouts">${chips}</div></div>`,
    );
  }
  el("cal-grid").innerHTML = cells.join("");

  // The agenda is the SAME events as the grid, regrouped into a list. Seven
  // columns of text crowd at narrow widths, so below the breakpoint CSS hides
  // the grid and shows this instead — both are always in the DOM from one
  // render, so the selected month can never disagree between the two layouts.
  renderAgenda(year, month, last.getDate(), byDay, painByDay);

  // A month change slides the surface in from the direction you moved — the
  // grid and agenda travel together as one surface. First paint doesn't slide.
  if (dir !== "none") {
    const grid = el("cal-grid");
    const agenda = el("cal-agenda");
    void animateSlideIn(grid, dir);
    void animateSlideIn(agenda, dir);
  }
}

/// A day-by-day list of the month's sessions, for narrow windows. Days with
/// nothing on them are skipped — an agenda of 31 rows where 5 have a run is
/// noise; the grid already shows the empty days as empty cells.
function renderAgenda(
  year: number,
  month: number,
  daysInMonth: number,
  byDay: Map<string, CalendarEvent[]>,
  painByDay: Map<string, number>,
): void {
  const rows: string[] = [];
  const todayKey = ymd(new Date());
  for (let day = 1; day <= daysInMonth; day++) {
    const key = ymd(new Date(year, month, day));
    const ws = byDay.get(key) ?? [];
    if (ws.length === 0) continue;
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
    const weekday = new Date(year, month, day).toLocaleDateString(undefined, { weekday: "short" });
    rows.push(
      `<div class="agenda-day${key === todayKey ? " agenda-today" : ""}"><h3 class="agenda-date"><span>${day}</span> ${weekday}${painDot}</h3><ul class="agenda-list">${items}</ul></div>`,
    );
  }
  el("cal-agenda").innerHTML = rows.length
    ? rows.join("")
    : `<p class="muted">No sessions scheduled this month.</p>`;
}

function initDragAndDrop(): void {
  const grid = el("cal-grid");
  let dragged: CalendarEvent | null = null;
  const clearTargets = () =>
    grid.querySelectorAll(".drop-target").forEach((cell) => cell.classList.remove("drop-target"));
  const targetCell = (event: DragEvent) => {
    const cell = (event.target as Element).closest<HTMLElement>(".cal-cell[data-date]");
    return cell && dragged && inCurrentWeek(cell.dataset.date!) ? cell : null;
  };

  grid.addEventListener("dragstart", (event) => {
    const chip = (event.target as Element).closest<HTMLElement>(".workout-chip.movable");
    const selected = chip ? monthEvents[Number(chip.dataset.eventIndex)] : null;
    if (!chip || !selected || !event.dataTransfer) return;
    dragged = selected;
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData("text/plain", selected.workoutId!);
    chip.classList.add("dragging");
    grid.classList.add("is-dragging");
  });
  grid.addEventListener("dragend", () => {
    dragged = null;
    clearTargets();
    grid.classList.remove("is-dragging");
    grid.querySelectorAll(".dragging").forEach((chip) => chip.classList.remove("dragging"));
  });
  grid.addEventListener("dragover", (event) => {
    const cell = targetCell(event);
    if (!cell) return;
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
    if (!cell.classList.contains("drop-target")) {
      clearTargets();
      cell.classList.add("drop-target");
    }
  });
  grid.addEventListener("drop", (event) => {
    const cell = targetCell(event);
    const workout = dragged;
    clearTargets();
    if (!cell || !workout) return;
    event.preventDefault();
    const date = cell.dataset.date!;
    if (date === workout.date) return;
    tauri
      .moveWorkout(workout.workoutId!, date)
      // Re-renders the grid; the Settings pane picks up the new publish time.
      .then(() => window.dispatchEvent(new CustomEvent("runalytics:calendar-updated")))
      .catch(fail);
  });
}

async function main(): Promise<void> {
  hydrateIcons();
  initSyncIndicator();
  initDragAndDrop();
  // One delegated listener covers both views: the grid's chips and the
  // agenda's rows carry the same data-event-index into the same array.
  el("calendar").addEventListener("click", (event) => {
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
  await main();
}

void boot();
