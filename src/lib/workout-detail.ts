import { tauri, fmtKm, fmtDateDMY, ymdLocal } from "./ipc";
import { createModal, type Modal } from "./overlay";
import { withBusy } from "./feedback";
import { renderRunReview } from "./run-review";
import { openMoveSheet, setSkipped } from "./schedule";
import type { ActivityReview, CalendarEvent, Settings } from "./types";

export function escapeHtml(value: string): string {
  const node = document.createElement("span");
  node.textContent = value;
  return node.innerHTML.replace(/"/g, "&quot;").replace(/'/g, "&#39;");
}

/// What the app says when the plan softened a session on its own. One function
/// because the chip tooltip, the agenda row and this sheet must word the same
/// change identically — three phrasings of one event reads as three bugs.
/// Positive-framed on purpose: the plan gave up a hard day to protect the rest
/// of the block, so the sentence says what it bought rather than what it took.
export function adjustedLabel(reason: string): string {
  return `Adjusted for you — ${reason}`;
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
/// Tear down the route map of the review currently on screen. Set whenever a
/// completed activity's review mounts; run when its modal closes.
let teardownReview: (() => void) | null = null;

export async function openCalendarEvent(event: CalendarEvent): Promise<void> {
  // `destroy()` drops the dialog without firing the close callback, so the
  // map of the run being replaced has to be released here as well as in
  // `onClosed` — otherwise replacing a run with a run leaks a canvas and a
  // theme observer every time.
  teardownReview?.();
  teardownReview = null;
  currentWorkoutModal?.destroy();
  currentWorkoutModal = null;

  const card = document.createElement("div");
  card.className = "workout-card";
  card.innerHTML = `<header class="workout-detail-head">
    <div><p class="eyebrow">${event.activityId == null ? "Planned session" : "Completed activity"}</p>
      <h2></h2><p class="muted workout-detail-date"></p></div>
    <div class="workout-detail-head-actions">
      <button type="button" class="secondary" data-copy>Copy</button>
      <button type="button" class="secondary" data-close autofocus>Close</button>
    </div>
    </header><div class="workout-detail-body"><p class="workout-adjusted" hidden></p>
    <div class="workout-notes"></div>
    <div class="workout-review" hidden></div>
    <div class="workout-actions" hidden>
      <button type="button" class="secondary" data-move>Move to&hellip;</button>
      <button type="button" class="secondary" data-skip>Skip this session</button>
    </div>
    <form class="workout-match" hidden><label for="workout-match-activity">Completed activity</label>
      <select id="workout-match-activity" required></select>
      <button type="submit">Replace planned session</button></form>
    <p class="workout-detail-status" role="status"></p></div>`;
  card.querySelector("h2")!.textContent = event.title;
  // The weekday stays — it is the orientation this line exists to give — but
  // the date itself carries the app's notation.
  card.querySelector(".workout-detail-date")!.textContent = `${new Date(`${event.date}T12:00:00`).toLocaleDateString(undefined, { weekday: "long" })} · ${fmtDateDMY(event.date)}`;
  // Above the prescription, not below it: the title the runner just read is the
  // softened one, so the reason belongs in the same frame as the value it
  // explains. A scroll away from it is an explanation that never gets read.
  const adjusted = card.querySelector<HTMLElement>(".workout-adjusted")!;
  if (event.adjusted) {
    adjusted.textContent = `${adjustedLabel(event.adjusted)}. An easier day today keeps the rest of the block on schedule.`;
    adjusted.hidden = false;
  }
  card.querySelector(".workout-notes")!.textContent = event.notes;
  // A session the runner moved says so here as well as on the calendar: the
  // marker in the grid is a fact about the day, and this is the surface where
  // the question "where was it before?" actually gets asked.
  if (event.moved && event.activityId == null && event.plannedDate) {
    card.querySelector(".workout-detail-date")!.textContent +=
      ` · moved from ${fmtDateDMY(event.plannedDate)}`;
  }

  // Native <dialog> via the shared modal controller: focus trap, Escape,
  // backdrop click, focus restore and the present/dismiss animation all come
  // from there instead of hand-rolled listeners on this element.
  const modal = createModal({
    card,
    label: event.title,
    className: "workout-modal",
    onClosed: () => {
      if (currentWorkoutModal === modal) currentWorkoutModal = null;
      teardownReview?.();
      teardownReview = null;
    },
  });
  currentWorkoutModal = modal;
  card.querySelector("[data-close]")!.addEventListener("click", () => modal.close());
  modal.open();

  // Declared before anything writes to it: the completed-activity path below
  // returns early, and the copy button still has to answer.
  const status = card.querySelector<HTMLElement>(".workout-detail-status")!;

  // The prescription is the runner's property, not ours to keep. One button
  // makes it portable — into a notes app, a group chat, a coach's inbox —
  // which is the difference between owning a plan and renting one.
  const copyButton = card.querySelector<HTMLButtonElement>("[data-copy]")!;
  copyButton.addEventListener("click", async () => {
    if (copyButton.disabled) return;
    try {
      await withBusy(copyButton, "Copying…", () => navigator.clipboard.writeText(event.notes));
      status.textContent = "Session copied.";
    } catch {
      status.textContent = "Clipboard unavailable. Select the text above to copy it.";
    }
  });

  // Every recorded run gets the full review — route, headline numbers, the
  // laps the plan asked for with their ranges, and the kilometre splits —
  // rendered by the shared component the activity sheet uses too, so the same
  // run reads identically from either entry point. The session verdict lives
  // inside that review, as the sentence under the lap chart: one read-out per
  // run rather than a separate block saying the same thing in another shape.
  // Best-effort: no review, no section.
  if (event.activityId != null) {
    const reviewEl = card.querySelector<HTMLElement>(".workout-review")!;
    Promise.all([tauri.activityReview(event.activityId), tauri.getSettings()])
      .then(([review, settings]: [ActivityReview, Settings]) => {
        // Same staleness rule as the verdict: the modal is reused, so a late
        // response must never paint a card it was not fetched for.
        if (!card.isConnected) return;
        reviewEl.hidden = false;
        teardownReview = renderRunReview(reviewEl, review, settings);
      })
      .catch(() => {
        /* A run with no reviewable data keeps the prescription — the review
           section simply is not offered. */
      });
  }

  if (event.activityId != null || !event.workoutId) return;

  // The two things a runner can still decide about a session that has not
  // happened: when it happens, and whether it happens at all. Race day is the
  // exception the backend refuses both for, so the controls are not offered.
  const actions = card.querySelector<HTMLElement>(".workout-actions")!;
  const moveButton = card.querySelector<HTMLButtonElement>("[data-move]")!;
  const skipButton = card.querySelector<HTMLButtonElement>("[data-skip]")!;
  if (event.kind !== "race") {
    actions.hidden = false;
    let skipped = event.status === "skipped";
    const renderSkip = (): void => {
      skipButton.textContent = skipped ? "Put it back on the plan" : "Skip this session";
      skipButton.classList.toggle("is-active", skipped);
    };
    renderSkip();
    moveButton.addEventListener("click", () => {
      // The move sheet replaces this one rather than stacking on it: two
      // dialogs over the same session is two ways to act on a stale view of it.
      modal.close();
      openMoveSheet(event);
    });
    skipButton.addEventListener("click", async () => {
      if (skipButton.disabled) return;
      const next = !skipped;
      try {
        await withBusy(skipButton, next ? "Skipping…" : "Restoring…", () =>
          setSkipped(event, next),
        );
        // Applied after withBusy resolves: its restore() replaces the button's
        // children, so a label set inside the action would be undone.
        skipped = next;
        renderSkip();
        status.textContent = next
          ? "Skipped — it won't count as a missed session."
          : "Back on the schedule.";
      } catch (error) {
        status.textContent = String(error);
      }
    });
  }

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