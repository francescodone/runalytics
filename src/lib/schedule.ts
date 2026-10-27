/* Rescheduling: everything that changes *when* a session happens.

   Three surfaces reach for these — the month grid (drag), the agenda list
   (the only rearrange path on a phone, where the grid is hidden), and the
   session sheet — so the confirmations, the guards' copy and the refresh all
   live here once rather than three times.

   The backend already refuses a move it disagrees with (race day is fixed,
   completed runs are history, nothing moves into the past or past the end of
   the plan). Nothing here re-implements those rules: the sheets bound the
   picker the same way so the choice is offered, and the backend's error is
   what surfaces if the state moved under us. */

import { tauri, fail, ymdLocal, fmtDateDMY } from "./ipc";
import { createSheet, type Sheet } from "./overlay";
import { withBusy } from "./feedback";
import type { CalendarEvent, WeekDeviation } from "./types";

/// Tell every open surface that the schedule moved. The calendar re-renders
/// its month and its week strip; the sync indicator picks up a new publish
/// stamp from the same event.
export function announceScheduleChange(): void {
  window.dispatchEvent(new CustomEvent("runalytics:calendar-updated"));
}

/// A date-only `YYYY-MM-DD` as a real date. Parsed at local noon on purpose:
/// `new Date("2026-08-14")` is UTC midnight, which a west-of-UTC user reads as
/// the 13th.
function fromYmd(value: string): Date {
  return new Date(`${value}T12:00:00`);
}

function weekdayLabel(value: string): string {
  return `${fromYmd(value).toLocaleDateString(undefined, { weekday: "short" })} ${fmtDateDMY(value)}`;
}

/* ---- the toast, with an optional action ---- */

const TOAST_MS = 6000;

/// The current toast's hide timer, so a new message replaces the old one's
/// countdown instead of stacking a second timer on it.
let toastTimer = 0;

/// A session title as the runner wrote it, without the runner emoji the
/// calendar feed prefixes every title with. Strips leading non-letters rather
/// than a fixed character: the emoji is a surrogate pair, so `/^\S\s/` would
/// eat half of it and leave a stray code point in front of the name.
export function bareTitle(title: string): string {
  return title.replace(/^[^\p{L}\p{N}]+/u, "").trim();
}

function toastNode(): HTMLElement {
  let toast = document.getElementById("toast");
  if (!toast) {
    toast = document.createElement("div");
    toast.id = "toast";
    toast.setAttribute("role", "status");
    toast.setAttribute("aria-live", "polite");
    document.body.appendChild(toast);
  }
  return toast as HTMLElement;
}

/// A transient message that can carry one action — the Undo that turns a
/// mis-drag into a two-tap fix instead of a hunt for the original day.
///
/// The plain `fail()` toast is text-only and `pointer-events: none`, so an
/// action needs the interactive variant: the label and the button are both
/// real children, and the class is dropped again when the toast hides so a
/// later error toast cannot inherit a clickable box.
export function toastWithAction(
  message: string,
  action: { label: string; run: () => Promise<void> } | null = null,
): void {
  const toast = toastNode();
  toast.textContent = message;
  toast.classList.add("show");
  if (action) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "toast-action";
    button.textContent = action.label;
    button.addEventListener("click", () => {
      hideToast();
      void action.run().catch(fail);
    });
    toast.appendChild(button);
    toast.classList.add("has-action");
  } else {
    toast.classList.remove("has-action");
  }
  window.clearTimeout(toastTimer);
  toastTimer = window.setTimeout(hideToast, TOAST_MS);
}

function hideToast(): void {
  const toast = toastNode();
  toast.classList.remove("show", "has-action");
}

/* ---- moving one session ---- */

/// Move a session and offer the way back. `from` is the day it was on, which
/// is what Undo restores — the frontend already holds it, so no extra read.
export async function moveSession(
  workoutId: string,
  date: string,
  from: string,
): Promise<void> {
  await tauri.moveWorkout(workoutId, date);
  announceScheduleChange();
  toastWithAction(`Moved to ${weekdayLabel(date)}`, {
    label: "Undo",
    run: async () => {
      await tauri.moveWorkout(workoutId, from);
      announceScheduleChange();
    },
  });
}

let moveSheet: Sheet | null = null;

/// The touch route to a new day. On a phone the grid is hidden and a
/// pointer-drag is not available, so without this there is no way to
/// rearrange a plan on the device most plans are actually rearranged on.
export function openMoveSheet(event: CalendarEvent): void {
  moveSheet?.destroy();
  moveSheet = null;

  const sheet = createSheet({
    label: "Move session",
    onClosed: () => {
      if (moveSheet === sheet) moveSheet = null;
    },
  });
  moveSheet = sheet;
  sheet.setTitle("Move this session", bareTitle(event.title));

  const body = sheet.body;
  const field = document.createElement("label");
  field.className = "move-field";
  const caption = document.createElement("span");
  caption.textContent = "New day";
  const input = document.createElement("input");
  input.type = "date";
  input.id = "move-session-date";
  input.value = event.date;
  input.required = true;
  field.append(caption, input);

  const status = document.createElement("p");
  status.className = "move-status";
  status.setAttribute("role", "status");
  status.textContent = `Currently on ${weekdayLabel(event.date)}. A session can move from today to the end of your plan.`;

  const button = document.createElement("button");
  button.type = "button";
  button.textContent = "Move session";

  const actions = document.createElement("div");
  actions.className = "move-actions";
  actions.appendChild(button);
  body.append(field, status, actions);

  sheet.open();

  button.addEventListener("click", async () => {
    if (button.disabled) return;
    const date = input.value;
    if (!date) {
      status.textContent = "Pick the day you want to run it on.";
      return;
    }
    if (date === event.date) {
      status.textContent = "That is already the day it is on.";
      return;
    }
    try {
      await withBusy(button, "Moving…", () => moveSession(event.workoutId!, date, event.date));
      sheet.close();
    } catch (error) {
      status.textContent = String(error);
    }
  });

  // The bounds come from the plan rather than a guess, so the picker cannot
  // offer a day the backend is going to refuse. A failed read leaves the
  // input unbounded and the backend still answers.
  void tauri
    .currentPlan()
    .then((current) => {
      if (!current || !sheet.isOpen()) return;
      input.min = ymdLocal(new Date());
      input.max = current[0].targetDate;
    })
    .catch(() => undefined);
}

/* ---- resetting a week ---- */

let resetSheet: Sheet | null = null;

/// Put a week back onto the days the plan generated, after showing what that
/// means. The list is the backend's own `week_deviations` read, so the sheet
/// cannot promise to restore a session the reset is going to skip.
export function openResetWeekSheet(weekStart: string, onClose?: () => void): void {
  resetSheet?.destroy();
  resetSheet = null;

  const sheet = createSheet({
    label: "Reset week",
    onClosed: () => {
      if (resetSheet === sheet) resetSheet = null;
      onClose?.();
    },
  });
  resetSheet = sheet;
  sheet.setTitle("Reset this week", "Put the sessions back on the days your plan generated them.");

  const body = sheet.body;
  const list = document.createElement("ul");
  list.className = "reset-list";
  const status = document.createElement("p");
  status.className = "move-status";
  status.setAttribute("role", "status");
  const button = document.createElement("button");
  button.type = "button";
  button.disabled = true;
  button.textContent = "Reset week";
  const actions = document.createElement("div");
  actions.className = "move-actions";
  actions.appendChild(button);
  body.append(list, status, actions);
  status.textContent = "Checking this week…";
  sheet.open();

  const render = (deviations: WeekDeviation[]): void => {
    list.replaceChildren();
    for (const deviation of deviations) {
      const li = document.createElement("li");
      const title = document.createElement("span");
      title.className = "reset-title";
      title.textContent = bareTitle(deviation.title);
      const days = document.createElement("span");
      days.className = "reset-days";
      days.textContent = deviation.restorable
        ? `${weekdayLabel(deviation.to)} → ${weekdayLabel(deviation.from)}`
        : `${weekdayLabel(deviation.to)} · stays`;
      li.append(title, days);
      if (!deviation.restorable) li.classList.add("not-restorable");
      list.appendChild(li);
    }
    const restorable = deviations.filter((d) => d.restorable).length;
    button.disabled = restorable === 0;
    button.textContent = restorable === 0 ? "Nothing to reset" : `Move ${restorable} back`;
    status.textContent =
      restorable === 0
        ? "Nothing here can go back — a session whose planned day has already passed has nowhere to return to."
        : `Race day, runs you have already done and sessions you skipped stay where they are.`;
  };

  void tauri
    .weekDeviations(weekStart)
    .then((deviations) => {
      if (!sheet.isOpen()) return;
      render(deviations);
    })
    .catch((error) => {
      if (!sheet.isOpen()) return;
      status.textContent = String(error);
    });

  button.addEventListener("click", async () => {
    if (button.disabled) return;
    try {
      const count = await withBusy(button, "Resetting…", async () => {
        const moved = await tauri.resetWeek(weekStart);
        announceScheduleChange();
        return moved;
      });
      sheet.close();
      toastWithAction(
        count === 1 ? "One session is back on its planned day." : `${count} sessions are back on their planned days.`,
      );
    } catch (error) {
      status.textContent = String(error);
    }
  });
}

/* ---- skipping a session ---- */

/// Mark a session deliberately skipped, or undo that. Skipping is how a
/// session leaves the schedule without becoming a lapse, so the ratio and the
/// missed-session detector both change their answer — which is why the caller
/// is told to refresh rather than patching one row locally.
export async function setSkipped(event: CalendarEvent, skipped: boolean): Promise<void> {
  await tauri.setWorkoutSkipped(event.workoutId!, skipped);
  announceScheduleChange();
  toastWithAction(
    skipped ? "Marked as skipped — it won't count as a missed session." : "Back on the schedule.",
    {
      label: "Undo",
      run: async () => {
        await tauri.setWorkoutSkipped(event.workoutId!, !skipped);
        announceScheduleChange();
      },
    },
  );
}
