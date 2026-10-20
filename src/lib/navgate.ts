// Which tabs exist before a plan is in place.
//
// Until setup finishes (watch connected, history synced, first plan created)
// the nav offers Dashboard and Settings only. Everything else — Plan, Calendar,
// Injuries, Achievements — describes or changes a training plan, so there is
// nothing to show on them yet, and a greyed-out tab invites clicking it.
//
// The Plan page is the one exception: it is the single place a plan can be
// created, so it has to be reachable. It is reached from Settings → Setup, and
// while it is open it appears in the nav as the current page with a way back to
// Setup. Once a plan exists the tab is there permanently.
//
// The check runs on every page because the pages are separate documents: the
// nav is static HTML, so without this a finished URL would open a locked tab
// and a page with nothing to render on it.

import { tauri } from "./ipc";
import { icons } from "./icons";

/// Tabs that only mean something once a plan is in place.
const GATED_HREFS = ["plan.html", "calendar.html", "injury.html", "achievements.html"];

export interface NavGate {
  /** True once connect + sync + a first plan are all done. */
  unlocked: boolean;
}

function link(href: string): HTMLAnchorElement | null {
  return document.querySelector<HTMLAnchorElement>(`#nav a.nav-link[href="${href}"]`);
}

/// Paint the nav for the current state. `onPlanPage` reveals the Plan tab as
/// the active page even while it is otherwise hidden, because the user is
/// standing on it and the nav should say where they are.
export function applyNavGate(gate: NavGate, onPlanPage = false): void {
  const nav = document.getElementById("nav");
  if (!nav) return;
  for (const href of GATED_HREFS) {
    const anchor = link(href);
    if (!anchor) continue;
    const show = gate.unlocked || (onPlanPage && href === "plan.html");
    anchor.hidden = !show;
    if (!show) anchor.removeAttribute("aria-current");
  }
  if (onPlanPage && !gate.unlocked && !document.getElementById("nav-setup-link")) {
    const back = document.createElement("a");
    back.id = "nav-setup-link";
    back.className = "nav-link";
    back.href = "settings.html#setup";
    // Same icon the Setup tab carries, so the two read as one destination.
    back.innerHTML = `<span class="nav-icon">${icons.rocket}</span>Setup`;
    nav.querySelector("#nav-destinations")?.append(back);
  }
}

/// Read the state from the backend and paint the nav.
///
/// When the backend cannot be reached the nav stays as authored: this decides
/// which tabs *exist*, which is not a question a missing backend should answer.
/// Pages that need data still fail loudly on their own fetch.
export async function initNavGate(onPlanPage = false): Promise<NavGate> {
  let gate: NavGate = { unlocked: true };
  try {
    const [statuses, current] = await Promise.all([
      tauri.providerStatus(),
      tauri.currentPlan().catch(() => null),
    ]);
    const coros = statuses.find((s) => s.provider === "coros");
    gate = {
      unlocked: Boolean(
        coros?.connected && coros.lastSync != null && current?.[0] != null,
      ),
    };
  } catch {
    /* keep the full nav */
  }
  applyNavGate(gate, onPlanPage);
  return gate;
}

/// Send the user back to Dashboard + Settings when this tab only makes sense
/// with a plan. Returns true when a redirect was started.
///
/// Redirecting rather than rendering a "locked" screen: the page genuinely has
/// no subject — no plan, no calendar, no proposed plan change — and a second
/// explanation of that fact is worth less than putting them somewhere useful.
export async function requirePlanOrRedirect(self: "plan" | "gated"): Promise<boolean> {
  let unlocked = true;
  try {
    const [statuses, current] = await Promise.all([
      tauri.providerStatus(),
      tauri.currentPlan().catch(() => null),
    ]);
    const coros = statuses.find((s) => s.provider === "coros");
    unlocked = Boolean(coros?.connected && coros.lastSync != null && current?.[0] != null);
  } catch {
    unlocked = true;
  }
  if (unlocked) {
    void initNavGate(self === "plan");
    return false;
  }
  applyNavGate({ unlocked: false }, self === "plan");
  if (self === "plan") return false;
  window.location.replace("index.html");
  return true;
}
