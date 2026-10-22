import { tauri, fmtKm, ymdLocal } from "./ipc";
import { createModal, type Modal } from "./overlay";
import { withBusy } from "./feedback";
import type { CalendarEvent } from "./types";

export function escapeHtml(value: string): string {
  const node = document.createElement("span");
  node.textContent = value;
  return node.innerHTML.replace(/"/g, "&quot;").replace(/'/g, "&#39;");
}

export async function openWorkout(workoutId: string, date: string): Promise<void> {
  const events = await tauri.calendarEvents("1970-01-01", "2100-12-31");
  const event = events.find((entry) => entry.workoutId === workoutId);
  if (!event) throw new Error(`Workout on ${date} is no longer available.`);
  await openCalendarEvent(event);
}

/// The workout modal is a single surface: opening another session replaces it
/// rather than stacking a second modal on top.
let currentWorkoutModal: Modal | null = null;

export async function openCalendarEvent(event: CalendarEvent): Promise<void> {
  currentWorkoutModal?.destroy();
  currentWorkoutModal = null;

  const card = document.createElement("div");
  card.className = "workout-card";
  card.innerHTML = `<header class="workout-detail-head">
    <div><p class="eyebrow">${event.activityId == null ? "Planned session" : "Completed activity"}</p>
      <h2></h2><p class="muted workout-detail-date"></p></div>
    <button type="button" class="secondary" data-close autofocus>Close</button>
    </header><div class="workout-detail-body"><div class="workout-notes"></div>
    <form class="workout-match" hidden><label for="workout-match-activity">Completed activity</label>
      <select id="workout-match-activity" required></select>
      <button type="submit">Replace planned session</button></form>
    <p class="workout-detail-status" role="status"></p></div>`;
  card.querySelector("h2")!.textContent = event.title;
  card.querySelector(".workout-detail-date")!.textContent = new Date(`${event.date}T12:00:00`).toLocaleDateString(undefined, { weekday: "long", day: "numeric", month: "long", year: "numeric" });
  card.querySelector(".workout-notes")!.textContent = event.notes;

  // Native <dialog> via the shared modal controller: focus trap, Escape,
  // backdrop click, focus restore and the present/dismiss animation all come
  // from there instead of hand-rolled listeners on this element.
  const modal = createModal({
    card,
    label: event.title,
    className: "workout-modal",
    onClosed: () => {
      if (currentWorkoutModal === modal) currentWorkoutModal = null;
    },
  });
  currentWorkoutModal = modal;
  card.querySelector("[data-close]")!.addEventListener("click", () => modal.close());
  modal.open();

  if (event.activityId != null || !event.workoutId) return;
  const status = card.querySelector<HTMLElement>(".workout-detail-status")!;
  try {
    const from = new Date(`${event.date}T12:00:00`);
    const to = new Date(from);
    from.setDate(from.getDate() - 7);
    to.setDate(to.getDate() + 7);
    const candidates = (await tauri.calendarEvents(ymdLocal(from), ymdLocal(to)))
      .filter((entry) => entry.activityId != null && entry.workoutId == null);
    if (!card.isConnected || candidates.length === 0) return;
    const form = card.querySelector<HTMLFormElement>(".workout-match")!;
    const select = card.querySelector<HTMLSelectElement>("#workout-match-activity")!;
    select.add(new Option("Select a recorded run", ""));
    for (const candidate of candidates) {
      select.add(new Option(`${candidate.date} - ${fmtKm(candidate.distanceM ?? 0)} - ${Math.round(candidate.durationMin ?? 0)} min`, String(candidate.activityId)));
    }
    form.hidden = false;
    form.addEventListener("submit", async (submit) => {
      submit.preventDefault();
      const button = form.querySelector<HTMLButtonElement>("button")!;
      if (button.disabled) return;
      status.textContent = "Updating calendar...";
      try {
        await withBusy(button, "Matching run…", () => tauri.matchCalendarActivity(event.workoutId!, Number(select.value)));
        window.dispatchEvent(new CustomEvent("runalytics:calendar-updated"));
        modal.close();
      } catch (error) { status.textContent = String(error); }
    });
  } catch (error) { status.textContent = String(error); }
}