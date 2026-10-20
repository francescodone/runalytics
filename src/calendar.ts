import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import { tauri, el, fail, inTauri } from "./lib/ipc";
import { initSyncIndicator } from "./lib/sync";
import { requirePlanOrRedirect } from "./lib/navgate";
// Side-effect import: paints the saved colour scheme before first paint.
import "./lib/theme";
import "./lib/navigation";
import { hydrateIcons } from "./lib/icons";
import { animateSlideIn } from "./lib/motion";
import { escapeHtml, openCalendarEvent } from "./lib/workout-detail";
import type { CalendarEvent, CalendarPublishing } from "./lib/types";

let cursor = new Date();
cursor.setDate(1);
let monthEvents: CalendarEvent[] = [];
let renderVersion = 0;
let publishing: CalendarPublishing | null = null;

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
  for (let day = 1; day <= last.getDate(); day++) {
    const key = ymd(new Date(year, month, day));
    const ws = byDay.get(key) ?? [];
    const chips = ws
      .map(
        (w) =>
          `<button type="button" class="chip workout-chip chip-${w.kind}" data-event-index="${workouts.indexOf(w)}" aria-label="${escapeHtml(w.title)}">${KIND_LABEL[w.kind] ?? w.kind}${
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
      `<div class="cal-cell"><span class="cal-day"><span>${day}</span>${painDot}</span><div class="cal-workouts">${chips}</div></div>`,
    );
  }
  el("cal-grid").innerHTML = cells.join("");

  // The agenda is the SAME events as the grid, regrouped into a list. Seven
  // columns of text crowd at narrow widths, so below the breakpoint CSS hides
  // the grid and shows this instead — both are always in the DOM from one
  // render, so the selected month can never disagree between the two layouts.
  renderAgenda(year, month, last.getDate(), byDay, painByDay);

  renderLegend();

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
        (w) => `<li><button type="button" class="agenda-row chip-${w.kind}" data-event-index="${monthEvents.indexOf(w)}">
          <span class="agenda-kind">${KIND_LABEL[w.kind] ?? w.kind}</span>
          <span class="agenda-title">${escapeHtml(w.title)}</span>
          <span class="agenda-meta">${w.distanceM ? `${(w.distanceM / 1000).toFixed(1)} km` : ""}${w.distanceM && w.durationMin ? " · " : ""}${w.durationMin ? `${Math.round(w.durationMin)} min` : ""}</span>
        </button></li>`,
      )
      .join("");
    const weekday = new Date(year, month, day).toLocaleDateString(undefined, { weekday: "short" });
    rows.push(
      `<div class="agenda-day"><h3 class="agenda-date"><span>${day}</span> ${weekday}${painDot}</h3><ul class="agenda-list">${items}</ul></div>`,
    );
  }
  el("cal-agenda").innerHTML = rows.length
    ? rows.join("")
    : `<p class="muted">No sessions scheduled this month.</p>`;
}

/// What the coloured labels mean, right under the grid — the workout kinds
/// are jargon on first read, and the pain dot needs explaining too.
function renderLegend(): void {
  let legend = document.getElementById("cal-legend");
  if (!legend) {
    legend = document.createElement("p");
    legend.id = "cal-legend";
    legend.className = "muted heatmap-legend";
    el("cal-grid").after(legend);
  }
  const kinds = ["easy", "tempo", "intervals", "longRun", "race", "completed"]
    .map((k) => `<span class="chip chip-${k}">${KIND_LABEL[k]}</span>`)
    .join(" ");
  legend.innerHTML = `${kinds} · <span class="cal-pain sev-mid" aria-hidden="true"></span> pain logged that day`;
}

async function refreshIcs(): Promise<void> {
  const button = el<HTMLButtonElement>("ics-regenerate");
  button.disabled = true;
  try {
    el("ics-path").textContent = await tauri.icsPath();
    showPublishing(await tauri.calendarPublishing());
    await renderMonth();
  } catch (e) {
    if (!inTauri) {
      el("ics-path").textContent = "Available in the desktop app.";
      el("publish-status").textContent = "Publishing is available in the desktop app.";
    }
    else el("publish-status").textContent = String(e);
  } finally { button.disabled = false; }
}

function showPublishing(config: CalendarPublishing): void {
  publishing = config;
  el<HTMLInputElement>("calendar-enabled").checked = config.enabled;
  el<HTMLInputElement>("calendar-upload-url").value = config.uploadUrl;
  el<HTMLInputElement>("calendar-read-url").value = config.subscriptionUrl;
  el<HTMLInputElement>("calendar-username").value = config.username;
  el<HTMLInputElement>("calendar-password").placeholder = config.hasPassword ? "Saved in Keychain" : "Password or bearer token";
  el<HTMLInputElement>("calendar-subscription-url").value = config.subscriptionUrl;
  const ready = !!config.lastPublished && !!config.subscriptionUrl;
  el<HTMLButtonElement>("calendar-copy").disabled = !ready;
  el<HTMLButtonElement>("calendar-subscribe").disabled = !ready;
  el("publish-status").textContent = config.lastError
    ? `${config.lastError}. Local calendar is up to date.`
    : !config.enabled
      ? "Remote publishing is off."
      : config.lastPublished
        ? `Published ${new Date(config.lastPublished).toLocaleString()}`
        : "Not yet published.";
  el("publish-status").classList.toggle("error", !!config.lastError);
}

function initPublishing(): void {
  el("calendar-publishing-form").addEventListener("submit", async (event) => {
    event.preventDefault();
    const button = el<HTMLButtonElement>("calendar-save");
    button.disabled = true;
    el("publish-status").textContent = "Saving calendar settings...";
    const password = el<HTMLInputElement>("calendar-password");
    const clear = el<HTMLInputElement>("calendar-clear-password");
    try {
      const config: CalendarPublishing = {
        enabled: el<HTMLInputElement>("calendar-enabled").checked,
        uploadUrl: el<HTMLInputElement>("calendar-upload-url").value.trim(),
        subscriptionUrl: el<HTMLInputElement>("calendar-read-url").value.trim(),
        username: el<HTMLInputElement>("calendar-username").value.trim(),
        hasPassword: false, lastPublished: null, lastError: null,
      };
      const secret = clear.checked ? "" : password.value || null;
      showPublishing(await tauri.saveCalendarPublishing(config, secret));
      password.value = "";
      clear.checked = false;
      await renderMonth();
    } catch (error) { el("publish-status").textContent = String(error); }
    finally { button.disabled = false; }
  });
  el("calendar-copy").addEventListener("click", async () => {
    if (!publishing?.subscriptionUrl) return;
    try {
      await navigator.clipboard.writeText(publishing.subscriptionUrl);
      el("publish-status").textContent = "Subscription URL copied.";
    } catch { el("publish-status").textContent = "Clipboard unavailable. Select the subscription URL to copy it."; }
  });
  el("calendar-subscribe").addEventListener("click", () => {
    if (publishing?.subscriptionUrl) {
      void openUrl(publishing.subscriptionUrl.replace(/^https:/, "webcal:")).catch(fail);
    }
  });
}

async function main(): Promise<void> {
  hydrateIcons();
  initSyncIndicator();
  initPublishing();
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
      void tauri.calendarPublishing().then(showPublishing).catch(fail);
    });
  }
  try {
    await renderMonth();
  } catch (e) {
    if (!inTauri) {
      el("cal-grid").innerHTML =
        `<div class="empty-state"><p>Runalytics needs the desktop app for data — run \`npm run tauri dev\` and open the Calendar there.</p></div>`;
    } else {
      fail(e);
    }
  }
  void refreshIcs();

  el("cal-prev").addEventListener("click", () => {
    cursor.setMonth(cursor.getMonth() - 1);
    renderMonth("back").catch(fail);
  });
  el("cal-next").addEventListener("click", () => {
    cursor.setMonth(cursor.getMonth() + 1);
    renderMonth("next").catch(fail);
  });
  el("ics-regenerate").addEventListener("click", refreshIcs);
  el("ics-reveal").addEventListener("click", async () => {
    const path = el("ics-path").textContent;
    if (!path) return;
    if (!inTauri) {
      alert("Reveal is only available in the desktop app.");
      return;
    }
    try {
      await revealItemInDir(path);
    } catch {
      // Fall back to showing the folder in a browser-style listing is not
      // possible; open the parent directory path as a last resort.
      await openUrl(`file://${path.split("/").slice(0, -1).join("/")}`).catch(fail);
    }
  });
}

// The calendar is the plan laid over the month — with no plan there are no
// scheduled sessions to show, so it redirects rather than rendering an empty grid.
async function boot(): Promise<void> {
  if (await requirePlanOrRedirect("gated")) return;
  await main();
}

void boot();
