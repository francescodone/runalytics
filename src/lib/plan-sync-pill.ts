// The plan-sync status pill.
//
// The channel already keeps a monotonic revision (`installed_rev`), but until
// now nothing showed it: the Settings pane reported whether the *channel* was
// available, never whether *this device* held what the channel holds. The pill
// answers the runner's actual question — "is my plan the one I published?" —
// and, on the phone, offers the import that answers it.
//
// It is a read-only check (`plan_sync_check`): a status light must never
// install anything, and the monotonic rule that stops a downgrade lives in
// `pull`, unchanged. The backend reports the same relation the pull would
// enforce, so the pill cannot promise an import the guard would then refuse.
//
// Placement follows the writer/importer split rather than one fixed slot: on
// the desktop it sits in the Plan actions header beside the facts it qualifies,
// on the phone in the page header, where it is the first thing read — iOS is
// pull-only, so this is the surface where a stale plan is most likely.

import { tauri, inTauri } from "./ipc";
import type { RemotePlan, SyncPillStatus } from "./types";
import { withBusy } from "./feedback";

/// Where the pill lives at a given width, so a viewport change can move the
/// one element rather than render a second copy of it.
type Slot = "page" | "panel";

let pill: HTMLElement | null = null;
let current: RemotePlan | null = null;
/// Which end of the channel this device is. Set from the backend, because the
/// platform — not the runner — decides it.
let writer = false;

/// "12 minutes ago" style, capped at a week: past that the date is more useful
/// than a number nobody can compare in their head.
function ago(epochSeconds: number): string {
  const seconds = Math.max(0, Math.floor(Date.now() / 1000 - epochSeconds));
  const minutes = Math.floor(seconds / 60);
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} hour${hours === 1 ? "" : "s"} ago`;
  const days = Math.floor(hours / 24);
  if (days < 7) return `${days} day${days === 1 ? "" : "s"} ago`;
  return new Date(epochSeconds * 1000).toLocaleDateString();
}

interface Copy {
  text: string;
  /// Settled, pending, or needing attention — drives the dot colour.
  tone: "ok" | "info" | "warn";
  /// Label when the runner can act here; absent for a read-only verdict.
  action?: string;
}

/// What to say, given which end of the channel this device is. The same status
/// means opposite things to the two ends: the writer's remote is its own
/// output, the importer's is an offer.
function copyFor(status: SyncPillStatus, remote: RemotePlan): Copy {
  const when = ago(remote.syncedAt);
  if (writer) {
    // The writer stamps `rev` from the plan's creation time, so an equal
    // revision is the published copy of exactly this plan.
    if (status === "upToDate") return { text: `Published ${when}`, tone: "ok" };
    return {
      text:
        status === "unpublished"
          ? "This plan is not published yet"
          : "The published plan is not this one",
      tone: "warn",
      action: "Publish now",
    };
  }
  switch (status) {
    case "upToDate":
      return { text: `Plan up to date · published ${when}`, tone: "ok" };
    case "updateAvailable":
      return {
        text: `New plan version · ${remote.workouts} workouts · ${when}`,
        tone: "info",
        action: "Import",
      };
    case "importAvailable":
      return { text: `A plan is waiting · ${remote.planName}`, tone: "info", action: "Import" };
    case "otherPlan":
      return {
        text: `A different plan is published · ${remote.planName}`,
        tone: "warn",
        action: "Import",
      };
    default:
      // `unpublished` is a writer-side verdict the backend only reports on the
      // publishing platform; an importer that somehow sees it has nothing
      // newer to take, which is the honest settled reading.
      return { text: `Plan up to date · published ${when}`, tone: "ok" };
  }
}

function render(remote: RemotePlan | null): void {
  if (!pill) return;
  current = remote;
  disarm();
  const text = pill.querySelector<HTMLElement>(".sync-pill-text");
  const action = pill.querySelector<HTMLButtonElement>(".sync-pill-action");
  // Nothing to report, or a writer with no plan of its own to compare against
  // — in both cases a pill would be noise.
  if (!remote || !text || !action || (writer && remote.status === "importAvailable")) {
    pill.hidden = true;
    return;
  }
  const copy = copyFor(remote.status, remote);
  pill.dataset.tone = copy.tone;
  text.textContent = copy.text;
  action.hidden = !copy.action;
  if (copy.action) action.textContent = copy.action;
  pill.hidden = false;
}

// Importing a *different* plan replaces the one already here, and that needs a
// real confirmation. `window.confirm` is not it: Tauri's WKWebView shows no
// dialog and returns falsy, which is why Abort plan on this page arms instead.
// The pill arms the same way — the second press is the confirmation.
let armed = false;
let armTimer = 0;

function disarm(): void {
  armed = false;
  clearTimeout(armTimer);
  const action = pill?.querySelector<HTMLButtonElement>(".sync-pill-action");
  if (action && current && !action.hidden) {
    action.textContent = copyFor(current.status, current).action ?? "";
  }
}

function arm(): void {
  armed = true;
  const action = pill?.querySelector<HTMLButtonElement>(".sync-pill-action");
  if (action) action.textContent = "Replace this plan?";
  clearTimeout(armTimer);
  armTimer = window.setTimeout(disarm, 6000);
}

async function act(): Promise<void> {
  if (writer) {
    await tauri.planSyncPush();
    return;
  }
  const outcome = await tauri.planSyncPull(current?.status === "otherPlan");
  if (outcome.installed) {
    // The plan changed underneath every view, including this page.
    window.dispatchEvent(new CustomEvent("runalytics:calendar-updated"));
  }
}

function place(slot: Slot): void {
  if (!pill) return;
  const target =
    slot === "page"
      ? document.querySelector(".page-header")
      : document.querySelector("#plan-view .panel-head");
  if (target && pill.parentElement !== target) target.append(pill);
}

/// Ask the channel what it holds. Silent on failure: a status light is never
/// worth an alert, and the Settings pane is where a real sync problem is read.
export async function refreshSyncPill(): Promise<void> {
  if (!pill || !inTauri) return;
  try {
    writer = (await tauri.planSyncStatus()).config.canPublish;
    render(await tauri.planSyncCheck());
  } catch {
    render(null);
  }
}

/// Mount the pill on the Plan page. No-op outside Tauri — with no backend
/// there is no channel to report on, and a decorative "up to date" would lie.
export function initSyncPill(): void {
  if (!inTauri || pill) return;
  const panelHead = document.querySelector("#plan-view .panel-head");
  if (!panelHead) return;

  pill = document.createElement("div");
  pill.id = "sync-pill";
  pill.className = "sync-pill";
  pill.setAttribute("aria-live", "polite");
  pill.hidden = true;
  pill.innerHTML =
    '<span class="sync-pill-dot" aria-hidden="true"></span>' +
    '<span class="sync-pill-text"></span>' +
    '<button type="button" class="sync-pill-action" hidden></button>';
  panelHead.append(pill);

  const action = pill.querySelector<HTMLButtonElement>(".sync-pill-action");
  action?.addEventListener("click", async () => {
    if (!action) return;
    // Arming happens before withBusy, not inside it: withBusy snapshots the
    // button's children and restores them when the call settles, so a label
    // set inside the action would be undone the moment it returned.
    if (!writer && current?.status === "otherPlan" && !armed) {
      arm();
      return;
    }
    try {
      await withBusy(action, "Working…", act);
      // After withBusy settles: its restore would undo anything render() did
      // to this button, and the sentence must describe the channel as it is
      // now that the action has run.
      await refreshSyncPill();
    } catch (error) {
      disarm();
      const text = pill?.querySelector<HTMLElement>(".sync-pill-text");
      if (text && current) text.textContent = String(error);
      if (pill) pill.dataset.tone = "warn";
    }
  });

  const compact = window.matchMedia("(max-width: 900px)");
  const apply = () => place(compact.matches ? "page" : "panel");
  apply();
  compact.addEventListener("change", apply);

  // A sync, a plan edit, or a publish from the Settings pane all change what
  // the channel holds.
  for (const event of ["runalytics:sync-completed", "runalytics:calendar-updated"]) {
    window.addEventListener(event, () => {
      void refreshSyncPill();
    });
  }
  void refreshSyncPill();
}
